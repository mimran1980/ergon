// SPDX-License-Identifier: Apache-2.0

//go:build cgroup_kernel

// Package cgroup_test runs the partition plan against a real cgroup v2
// kernel. The unit fixtures in internal/hook manufacture cpuset.* files on
// ordinary directories, so they cannot show the two things the kernel decides:
// that a child has no cpuset files until the parent enables the controller,
// and that a child of a threaded domain is "domain invalid" until its
// cgroup.type says "threaded". This test is what shows them.
//
// It needs write access to the cgroup v2 root. Run it through
// scripts/check-cgroup.sh, which starts a privileged container for it.
package cgroup_test

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"testing"
	"time"

	"golang.org/x/sys/unix"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/hook"
)

// cgroupRoot returns the cgroup v2 mount point, and fails the test when this
// process cannot write there: the whole point of this file is to run on a
// real kernel.
func cgroupRoot(t *testing.T) string {
	t.Helper()
	data, err := os.ReadFile("/proc/self/mountinfo")
	if err != nil {
		t.Fatalf("read /proc/self/mountinfo: %v", err)
	}
	for _, line := range strings.Split(string(data), "\n") {
		fields := strings.Fields(line)
		i := indexOf(fields, "-")
		if i < 0 || i+1 >= len(fields) || fields[i+1] != "cgroup2" {
			continue
		}
		if len(fields) < 5 {
			continue
		}
		root := fields[4]
		// cgroup2 has no arbitrary files: a child cgroup is the only thing
		// that can be created here, so that is the probe.
		probe := filepath.Join(root, ".ergon-cgroup-probe")
		if err := os.Mkdir(probe, 0o755); err != nil {
			t.Fatalf("cannot create a child cgroup under %s (%v): this test needs write access to the cgroup v2 hierarchy; run it through scripts/check-cgroup.sh", root, err)
		}
		_ = os.Remove(probe)
		return root
	}
	t.Fatal("no cgroup2 mount in /proc/self/mountinfo")
	return ""
}

func indexOf(fields []string, s string) int {
	for i, f := range fields {
		if f == s {
			return i
		}
	}
	return -1
}

// enableCpuset writes cpuset into a cgroup's cgroup.subtree_control. This is
// setup for the test tree, not the code under test: without it the next level
// down has no cpuset files at all. The write is "+cpuset", because
// cgroup.subtree_control takes operations and a bare name is EINVAL.
func enableCpuset(t *testing.T, dir string) {
	t.Helper()
	path := filepath.Join(dir, "cgroup.subtree_control")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	for _, f := range strings.Fields(string(data)) {
		if strings.TrimPrefix(f, "+") == "cpuset" {
			return
		}
	}
	if err := os.WriteFile(path, []byte("+cpuset\n"), 0o644); err != nil {
		t.Fatalf("enable cpuset in %s: %v", path, err)
	}
}

// tree builds a container cgroup under a scratch root and returns the cgroup
// root, the container directory and the two CPUs the test uses: the first two
// on the machine's own effective set, so this works on any host. The isolated
// child is not created here; the plan is supposed to create it.
func tree(t *testing.T) (root, ctr string, exclusive, shared cpuset.Set) {
	t.Helper()
	root = cgroupRoot(t)

	all, err := cpuset.Parse(read(t, filepath.Join(root, "cpuset.cpus.effective")))
	if err != nil || all.Len() < 2 {
		t.Fatalf("the cgroup root holds %s: this test needs at least two CPUs", read(t, filepath.Join(root, "cpuset.cpus.effective")))
	}
	cpus := all.Slice()
	exclusive, shared = cpuset.New(cpus[0]), cpuset.New(cpus[1])
	// A CPU already claimed as exclusive by a sibling is rejected by the
	// kernel's exclusivity rule, and the error names nothing useful. Say what
	// is in the way instead. A leftover from a previous run is the usual cause.
	for _, want := range []cpuset.Set{exclusive, shared} {
		if by := claimedBy(root, want); by != "" {
			t.Fatalf("CPU %s is already exclusive to %s: remove it before running this test", want.List(), by)
		}
	}
	// The cgroup root has no cpuset.mems, only cpuset.mems.effective.
	mems := read(t, filepath.Join(root, "cpuset.mems.effective"))

	base := filepath.Join(root, "ergon-kernel-test-"+strconv.Itoa(os.Getpid())+"-"+strconv.FormatInt(time.Now().UnixNano(), 36))
	// Every level needs its parent to enable cpuset first. Stop one short of
	// the container: the plan enables cpuset on the container itself.
	enableCpuset(t, root)
	for _, rel := range []string{"", "pod", "pod/ctr"} {
		dir := base
		if rel != "" {
			dir = filepath.Join(base, rel)
		}
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatalf("mkdir %s: %v", dir, err)
		}
		write(t, filepath.Join(dir, "cpuset.cpus"), exclusive.Union(shared).String())
		write(t, filepath.Join(dir, "cpuset.mems"), mems)
		if rel != "pod/ctr" {
			enableCpuset(t, dir)
		}
	}
	ctr = filepath.Join(base, "pod", "ctr")
	t.Cleanup(func() { remove(t, base, root) })
	return root, ctr, exclusive, shared
}

// remove puts every task back on the cgroup root and takes the scratch tree
// down. A cgroup still holding a task cannot be removed, and a half-removed
// tree keeps its exclusive CPUs, which makes the next create fail the kernel's
// exclusivity rule. So this has to actually work, and say so when it does not.
func remove(t *testing.T, base, root string) {
	t.Helper()
	var dirs []string
	// cgroup2 directories hold pseudo-files that cannot be unlinked; only the
	// directories themselves are removed, with rmdir.
	_ = filepath.Walk(base, func(path string, info os.FileInfo, err error) error {
		if err == nil && info.IsDir() {
			dirs = append(dirs, path)
		}
		return nil
	})
	// Deepest first, so children go before their parents.
	var left []string
	for i := len(dirs) - 1; i >= 0; i-- {
		d := dirs[i]
		evacuate(t, d, root)
		// Take the cgroup out of any partition before rmdir. A cgroup2
		// directory is removed with rmdir, not by unlinking its files.
		_ = os.WriteFile(filepath.Join(d, "cpuset.cpus.partition"), []byte("member\n"), 0o644)
		if err := os.Remove(d); err != nil {
			left = append(left, d)
		}
	}
	if len(left) > 0 {
		t.Errorf("could not remove the scratch cgroup tree; these still hold CPUs and will block the next run: %s", strings.Join(left, ", "))
	}
}

// evacuate moves every task in a cgroup to the root. cgroup.procs is
// write-only inside a threaded subtree (reading it is EOPNOTSUPP: the
// processes belong to the thread root), so the readable list there is
// cgroup.threads. Either way, writing a tid to the root's cgroup.procs moves
// the whole process out.
func evacuate(t *testing.T, dir, root string) {
	t.Helper()
	for _, name := range []string{"cgroup.threads", "cgroup.procs"} {
		data, err := os.ReadFile(filepath.Join(dir, name))
		if err != nil {
			continue
		}
		for _, tid := range strings.Fields(string(data)) {
			if err := os.WriteFile(filepath.Join(root, "cgroup.procs"), []byte(tid+"\n"), 0o644); err != nil {
				t.Errorf("move tid %s out of %s: %v", tid, dir, err)
			}
		}
	}
}

func apply(t *testing.T, root, ctr string, exclusive, shared cpuset.Set) *cpuset.Plan {
	t.Helper()
	plan, err := cpuset.PartitionPlan(root, ctr, exclusive, shared)
	if err != nil {
		t.Fatalf("PartitionPlan: %v", err)
	}
	// The lock file lives outside the cgroup filesystem; keep it in a scratch
	// dir so a test never touches /run.
	old := hook.PartitionLockPath
	hook.PartitionLockPath = filepath.Join(t.TempDir(), "partition.lock")
	t.Cleanup(func() { hook.PartitionLockPath = old })
	if err := hook.ApplyPartition(plan); err != nil {
		t.Fatalf("ApplyPartition: %v", err)
	}
	return plan
}

// claimedBy names a child of root that already holds any of want as exclusive,
// or "" when the CPUs are free.
func claimedBy(root string, want cpuset.Set) string {
	entries, err := os.ReadDir(root)
	if err != nil {
		return ""
	}
	for _, e := range entries {
		if !e.IsDir() {
			continue
		}
		dir := filepath.Join(root, e.Name())
		data, err := os.ReadFile(filepath.Join(dir, "cpuset.cpus.exclusive"))
		if err != nil {
			continue
		}
		have, err := cpuset.Parse(strings.TrimSpace(string(data)))
		if err != nil {
			continue
		}
		if !have.Intersection(want).IsEmpty() {
			return dir
		}
	}
	return ""
}

func read(t *testing.T, path string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	return strings.TrimSpace(string(data))
}

func write(t *testing.T, path, value string) {
	t.Helper()
	if err := os.WriteFile(path, []byte(value+"\n"), 0o644); err != nil {
		t.Fatalf("write %s=%q: %v", path, value, err)
	}
}

// affinity returns a thread's Cpus_allowed_list straight from the kernel.
func affinity(t *testing.T, tid int) string {
	t.Helper()
	return allowedList(t, filepath.Join("/proc/self/task", strconv.Itoa(tid), "status"), fmt.Sprintf("tid %d", tid))
}

// processAffinity returns another process's Cpus_allowed_list.
func processAffinity(t *testing.T, pid int) string {
	t.Helper()
	return allowedList(t, filepath.Join("/proc", strconv.Itoa(pid), "status"), fmt.Sprintf("pid %d", pid))
}

func allowedList(t *testing.T, path, who string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read the affinity of %s: %v", who, err)
	}
	for _, line := range strings.Split(string(data), "\n") {
		if strings.HasPrefix(line, "Cpus_allowed_list:") {
			return strings.TrimSpace(strings.TrimPrefix(line, "Cpus_allowed_list:"))
		}
	}
	t.Fatalf("no Cpus_allowed_list for %s", who)
	return ""
}

// TestThePlanMakesATaskTakingPartition is the P1 end to end: the writes the
// plan produces must leave a child a real kernel will take a task in, and
// that task must land on the exclusive CPUs.
func TestThePlanMakesATaskTakingPartition(t *testing.T) {
	root, ctr, exclusive, shared := tree(t)
	plan := apply(t, root, ctr, exclusive, shared)
	child := plan.Child

	if got := read(t, filepath.Join(ctr, "cgroup.subtree_control")); !strings.Contains(got, "cpuset") {
		t.Errorf("container cgroup.subtree_control = %q, want it to enable cpuset", got)
	}
	if got := read(t, filepath.Join(child, "cgroup.type")); got != "threaded" {
		t.Errorf("child cgroup.type = %q, want threaded (a domain invalid child takes no task)", got)
	}
	if got := read(t, filepath.Join(child, "cpuset.cpus")); got != exclusive.String() {
		t.Errorf("child cpuset.cpus = %q, want %q", got, exclusive.String())
	}
	if got := read(t, filepath.Join(child, "cpuset.cpus.partition")); got != "isolated" {
		t.Errorf("child cpuset.cpus.partition = %q, want isolated (not an invalid partition)", got)
	}

	// The partition took the exclusive CPUs out of the container.
	left, err := cpuset.Parse(read(t, filepath.Join(ctr, "cpuset.cpus.effective")))
	if err != nil {
		t.Fatalf("parse the container's effective CPUs: %v", err)
	}
	if left.Contains(exclusive.Slice()[0]) {
		t.Errorf("container cpuset.cpus.effective = %s: the partition must take %s away", left.String(), exclusive.String())
	}

	// A real process, so the test itself never sits inside the partition.
	cmd := exec.Command("sleep", "30")
	if err := cmd.Start(); err != nil {
		t.Fatalf("start sleep: %v", err)
	}
	defer func() {
		_ = cmd.Process.Kill()
		_, _ = cmd.Process.Wait()
	}()
	// cgroup.procs takes the whole process. In a threaded member cgroup it is
	// write-only: reading it is EOPNOTSUPP, because the processes belong to
	// the thread root. cgroup.threads is the readable membership list.
	write(t, filepath.Join(child, "cgroup.procs"), strconv.Itoa(cmd.Process.Pid))
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		if strings.Contains(read(t, filepath.Join(child, "cgroup.threads")), strconv.Itoa(cmd.Process.Pid)) {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if !strings.Contains(read(t, filepath.Join(child, "cgroup.threads")), strconv.Itoa(cmd.Process.Pid)) {
		t.Fatalf("the kernel refused pid %d in the child (cgroup.threads = %q)",
			cmd.Process.Pid, read(t, filepath.Join(child, "cgroup.threads")))
	}
	// The kernel confined it: this is the isolation the partition is for.
	if got := processAffinity(t, cmd.Process.Pid); got != exclusive.String() {
		t.Errorf("pid %d affinity = %s, want %s", cmd.Process.Pid, got, exclusive.String())
	}
	_ = shared // the helpers' CPUs are checked in the other test
}

// TestOneThreadCanLiveInThePartition is the membership contract: the child is
// threaded, so one thread of a multi-threaded program can be confined to the
// exclusive CPUs while the helpers stay on the shared ones. This is the
// hot/helper split, enforced by the kernel rather than by sched_setaffinity.
func TestOneThreadCanLiveInThePartition(t *testing.T) {
	root, ctr, exclusive, shared := tree(t)
	plan := apply(t, root, ctr, exclusive, shared)
	child := plan.Child

	// Two OS threads: the caller and one helper.
	ready := make(chan int)
	done := make(chan struct{})
	go func() {
		runtime.LockOSThread()
		ready <- unix.Gettid()
		<-done
	}()
	helperTid := <-ready
	runtime.LockOSThread()
	hotTid := unix.Gettid()
	defer close(done)

	// A thread only moves inside the threaded subtree once its whole process
	// is in there: put the process in first, then scatter the threads.
	write(t, filepath.Join(child, "cgroup.procs"), strconv.Itoa(os.Getpid()))
	write(t, filepath.Join(ctr, "cgroup.threads"), strconv.Itoa(helperTid))

	if got := affinity(t, helperTid); got != shared.String() {
		t.Errorf("helper tid %d affinity = %s, want %s (the container's shared CPUs)", helperTid, got, shared.String())
	}
	if got := affinity(t, hotTid); got != exclusive.String() {
		t.Errorf("hot tid %d affinity = %s, want %s (the partition's exclusive CPUs)", hotTid, got, exclusive.String())
	}
}
