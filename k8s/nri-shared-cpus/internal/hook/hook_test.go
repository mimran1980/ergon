// SPDX-License-Identifier: Apache-2.0

package hook

import (
	"bytes"
	"errors"
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
)

// startChild starts a process that stays alive and returns its pid.
func startChild(t *testing.T) int {
	t.Helper()
	cmd := exec.Command("sleep", "30")
	if err := cmd.Start(); err != nil {
		t.Skipf("cannot start a child here: %v", err)
	}
	pid := cmd.Process.Pid
	t.Cleanup(func() {
		_ = cmd.Process.Kill()
		_, _ = cmd.Process.Wait()
	})
	for i := 0; i < 50; i++ {
		if _, err := os.Stat(fmt.Sprintf("/proc/%d/task", pid)); err == nil {
			return pid
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Fatalf("child pid %d never appeared in /proc", pid)
	return 0
}

func ownCPUs(t *testing.T) cpuset.Set {
	t.Helper()
	var mask unix.CPUSet
	if err := unix.SchedGetaffinity(0, &mask); err != nil {
		t.Fatal(err)
	}
	var cpus []int
	for c := 0; c < 1024; c++ {
		if mask.IsSet(c) {
			cpus = append(cpus, c)
		}
	}
	if len(cpus) == 0 {
		t.Fatal("no CPU in our own affinity")
	}
	return cpuset.New(cpus...)
}

func TestPinProcessSetsEveryThread(t *testing.T) {
	pid := startChild(t)
	self := ownCPUs(t)
	shared := cpuset.New(self.Slice()[0])
	if err := PinProcess(pid, shared); err != nil {
		t.Fatalf("PinProcess: %v", err)
	}
	var got unix.CPUSet
	if err := unix.SchedGetaffinity(pid, &got); err != nil {
		t.Fatal(err)
	}
	for _, c := range self.Slice() {
		want := c == shared.Slice()[0]
		if got.IsSet(c) != want {
			t.Fatalf("cpu %d: affinity set=%v, want %v (mask %s)", c, got.IsSet(c), want, shared.List())
		}
	}
}

func TestRunBadJSON(t *testing.T) {
	var errOut bytes.Buffer
	if err := Run(strings.NewReader("not json"), &errOut, cpuset.New(0), nil); err == nil {
		t.Fatal("Run accepted bad JSON")
	}
}

func TestRunNoPID(t *testing.T) {
	var errOut bytes.Buffer
	if err := Run(strings.NewReader(`{"id":"x"}`), &errOut, cpuset.New(0), nil); err == nil {
		t.Fatal("Run accepted state without a pid")
	}
}

func TestRunEmptyShared(t *testing.T) {
	var errOut bytes.Buffer
	if err := Run(strings.NewReader(`{"pid":1}`), &errOut, cpuset.Set{}, nil); err == nil {
		t.Fatal("Run accepted an empty shared list")
	}
}

func TestRunTooBig(t *testing.T) {
	var errOut bytes.Buffer
	big := strings.Repeat("a", MaxStateBytes+10)
	if err := Run(strings.NewReader(big), &errOut, cpuset.New(0), nil); err == nil {
		t.Fatal("Run accepted a state larger than the limit")
	}
}

func TestRunOnChild(t *testing.T) {
	pid := startChild(t)
	self := ownCPUs(t)
	shared := cpuset.New(self.Slice()[0])
	state := fmt.Sprintf(`{"id":"testctr","pid":%d,"status":"creating"}`, pid)
	var errOut bytes.Buffer
	if err := Run(strings.NewReader(state), &errOut, shared, nil); err != nil {
		t.Fatalf("Run: %v (stderr %q)", err, errOut.String())
	}
	if !strings.Contains(errOut.String(), "testctr") {
		t.Errorf("hook did not log the container id: %q", errOut.String())
	}
}

func TestRunMissingPID(t *testing.T) {
	var errOut bytes.Buffer
	state := `{"id":"gone","pid":999999999}`
	if err := Run(strings.NewReader(state), &errOut, cpuset.New(0), nil); err == nil {
		t.Fatal("Run accepted a pid that does not exist")
	}
}

// slackOf returns pid's timer slack in nanoseconds.
func slackOf(t *testing.T, pid int) int64 {
	t.Helper()
	data, err := os.ReadFile(fmt.Sprintf("/proc/%d/timerslack_ns", pid))
	if err != nil {
		t.Fatalf("read /proc/%d/timerslack_ns: %v", pid, err)
	}
	var ns int64
	if _, err := fmt.Sscanf(strings.TrimSpace(string(data)), "%d", &ns); err != nil {
		t.Fatalf("parse timer slack %q: %v", data, err)
	}
	return ns
}

// canWriteForeignTimerSlack reports whether this process may write another
// process's /proc/PID/timerslack_ns, which the kernel allows only with
// CAP_SYS_NICE (timerslack_ns_write, `task != current`).
func canWriteForeignTimerSlack() bool {
	cmd := exec.Command("sleep", "30")
	if err := cmd.Start(); err != nil {
		return false
	}
	defer func() {
		_ = cmd.Process.Kill()
		_, _ = cmd.Process.Wait()
	}()
	return os.WriteFile(
		fmt.Sprintf("/proc/%d/timerslack_ns", cmd.Process.Pid), []byte("1"), 0o644,
	) == nil
}

// requireTimerSlackCapability fails the test when this binary cannot write
// another task's /proc/PID/timerslack_ns. The kernel refuses `task != current`
// without CAP_SYS_NICE, and a Go test runs on a runtime thread, so even a
// write to this process's own pid needs it. The gate (scripts/check.sh) runs
// the tests with --cap-add=SYS_NICE, which is what the OCI hook has.
func requireTimerSlackCapability(t *testing.T) {
	t.Helper()
	if !canWriteForeignTimerSlack() {
		t.Fatal("cannot write another task's /proc/PID/timerslack_ns: the kernel needs CAP_SYS_NICE " +
			"(timerslack_ns_write). Run scripts/check.sh, which adds it, or run with CAP_SYS_NICE.")
	}
}

// The kernel copies a task's timer slack to every thread and child it later
// makes (copy_process). A thread that already exists keeps its own, so the
// hook writes the threads that are there now and inherits into the rest.
// Both halves are covered here: a thread that already exists is written, and
// a child process created afterwards inherits.
func TestSetTimerSlackReachesExistingThreadsAndLaterChildren(t *testing.T) {
	requireTimerSlackCapability(t)
	me := os.Getpid()
	before, err := tidsOf(me)
	if err != nil {
		t.Fatalf("tidsOf: %v", err)
	}
	if err := SetTimerSlack(me, TimerSlackNS); err != nil {
		t.Fatalf("SetTimerSlack: %v", err)
	}
	defer func() {
		_ = SetTimerSlack(me, 50_000)
	}()

	// Every thread that existed before the write now has the new value: this
	// is why the hook walks TIDs rather than writing only pid.
	after, err := tidsOf(me)
	if err != nil {
		t.Fatalf("tidsOf: %v", err)
	}
	for _, tid := range after {
		if got := slackOf(t, tid); got != TimerSlackNS {
			t.Fatalf("tid %d: timer slack = %d, want %d (existing thread not written)", tid, got, TimerSlackNS)
		}
	}
	if len(after) < len(before) {
		t.Fatalf("lost threads: before %v after %v", before, after)
	}

	// A child process created after the write inherits it, and keeps it
	// across exec.
	out, err := exec.Command("sh", "-c", "cat /proc/self/timerslack_ns").Output()
	if err != nil {
		t.Fatalf("child: %v", err)
	}
	if got := strings.TrimSpace(string(out)); got != strconv.Itoa(TimerSlackNS) {
		t.Fatalf("child process: timer slack = %s, want %d (not inherited)", got, TimerSlackNS)
	}
}

// A process that is not the caller's own needs CAP_SYS_NICE. The OCI hook has
// it (containerd runs hooks as root with full capabilities), so the write must
// reach the container. Without it the refusal must be the kernel's EPERM and
// must not be reported as done.
func TestRunSetsTimerSlackOnAnotherProcess(t *testing.T) {
	requireTimerSlackCapability(t)
	pid := startChild(t)
	self := ownCPUs(t)
	shared := cpuset.New(self.Slice()[0])
	state := fmt.Sprintf(`{"id":"slackctr","pid":%d,"status":"creating"}`, pid)
	var errOut bytes.Buffer
	if err := Run(strings.NewReader(state), &errOut, shared, nil); err != nil {
		t.Fatalf("Run: %v (stderr %q)", err, errOut.String())
	}
	if got := slackOf(t, pid); got != TimerSlackNS {
		t.Fatalf("after Run: timer slack = %d, want %d", got, TimerSlackNS)
	}
	if !strings.Contains(errOut.String(), "timer slack") {
		t.Errorf("hook did not report the timer slack: %q", errOut.String())
	}
}

func TestSetTimerSlackMissingPID(t *testing.T) {
	if err := SetTimerSlack(999999999, TimerSlackNS); err == nil {
		t.Fatal("SetTimerSlack accepted a pid that does not exist")
	}
}

func TestSetTimerSlackBelowTheKernelFloor(t *testing.T) {
	if err := SetTimerSlack(os.Getpid(), 0); err == nil {
		t.Fatal("SetTimerSlack accepted a slack of 0")
	}
}

// The kernel rule behind reading a task's timer slack (timerslack_ns_show,
// fs/proc/base.c): free only for `p == current`, otherwise CAP_SYS_NICE over
// the task's user namespace, refused with EPERM. There is no thread-group
// exception, and /proc/self resolves to the leader (fs/proc/self.c), so a
// sibling thread reading /proc/self/timerslack_ns is reading the leader's
// file and hits that rule. /proc/<own tid> is `p == current` and is always
// free.
//
// Both halves are asserted here, using the capability as the discriminator
// rather than skipping: with it the leader's file reads back, without it the
// same read is EPERM. That is the rule either way.
func TestTimerslackReadNeedsTheCapabilityOverAnotherTask(t *testing.T) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	pid := os.Getpid()
	tid := unix.Gettid()

	// This thread's own file always reads, whatever the capability.
	ownPath := filepath.Join("/proc", strconv.Itoa(tid), "timerslack_ns")
	own, err := os.ReadFile(ownPath)
	if err != nil {
		t.Fatalf("read %s: %v (a thread's own timerslack_ns must always read)", ownPath, err)
	}
	if _, err := strconv.ParseInt(strings.TrimSpace(string(own)), 10, 64); err != nil {
		t.Fatalf("parse %s %q: %v", ownPath, own, err)
	}

	if tid == pid {
		// This goroutine happens to be on the leader: /proc/self is this
		// task, so the rule exempts it and the two must agree.
		self, err := os.ReadFile("/proc/self/timerslack_ns")
		if err != nil {
			t.Fatalf("read /proc/self/timerslack_ns from the leader: %v", err)
		}
		if strings.TrimSpace(string(self)) != strings.TrimSpace(string(own)) {
			t.Fatalf("/proc/self says %q, want %q", self, own)
		}
		return
	}

	// A sibling thread: /proc/self is the leader's file, so the rule is the
	// capability one. Assert both outcomes, not just the one we can force.
	self, err := os.ReadFile("/proc/self/timerslack_ns")
	privileged := canWriteForeignTimerSlack()
	switch {
	case err == nil:
		if !privileged {
			t.Fatalf("read /proc/self/timerslack_ns from tid %d (pid %d) without CAP_SYS_NICE: got %q, want EPERM",
				tid, pid, self)
		}
	case errors.Is(err, unix.EPERM):
		if privileged {
			t.Fatalf("read /proc/self/timerslack_ns from tid %d (pid %d) with CAP_SYS_NICE: %v", tid, pid, err)
		}
	default:
		t.Fatalf("read /proc/self/timerslack_ns from tid %d (pid %d): %v, want the value or EPERM", tid, pid, err)
	}
}
