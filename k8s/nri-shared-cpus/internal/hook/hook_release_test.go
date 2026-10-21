// SPDX-License-Identifier: Apache-2.0

package hook

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
)

func TestParseArgsRelease(t *testing.T) {
	shared, part, err := ParseArgs([]string{"--isolated", "1,5", "--release-child", "/sys/fs/cgroup/pod/ctr/ergon-isolated"})
	if err != nil {
		t.Fatal(err)
	}
	if !shared.IsEmpty() {
		t.Errorf("shared = %q, want empty", shared)
	}
	if part == nil || !part.Release {
		t.Fatalf("part = %+v, want a release", part)
	}
	if part.Exclusive.String() != "1,5" {
		t.Errorf("exclusive = %q", part.Exclusive)
	}
	if part.Child != "/sys/fs/cgroup/pod/ctr/ergon-isolated" {
		t.Errorf("child = %q", part.Child)
	}
}

func TestParseArgsReleaseRefused(t *testing.T) {
	for _, tc := range []struct {
		name string
		args []string
	}{
		{"release without isolated", []string{"--release-child", "/c/ergon-isolated"}},
		{"isolated without release", []string{"--isolated", "1,5"}},
		{"relative child", []string{"--isolated", "1,5", "--release-child", "ergon-isolated"}},
		{"create and release", []string{"--isolated", "1,5", "--partition-child", "/c/ergon-isolated", "--release-child", "/c/ergon-isolated"}},
		{"release with shared", []string{"--shared", "0", "--isolated", "1,5", "--release-child", "/c/ergon-isolated"}},
	} {
		t.Run(tc.name, func(t *testing.T) {
			if _, _, err := ParseArgs(tc.args); err == nil {
				t.Errorf("ParseArgs(%v) must refuse", tc.args)
			}
		})
	}
}

// setupPartitionFixture builds one isolated child under root and returns the
// plan it was built from.
func setupPartitionFixture(t *testing.T, root, rel string, exclusive cpuset.Set) *cpuset.Plan {
	t.Helper()
	ctrDir := writeTree(t, root, rel)
	writeTree(t, root, filepath.Dir(rel))
	plan, err := cpuset.PartitionPlan(root, ctrDir, exclusive, cpuset.New(0))
	if err != nil {
		t.Fatal(err)
	}
	if err := ApplyPartition(plan); err != nil {
		t.Fatal(err)
	}
	return plan
}

// useTempRoot points cgroupRootOf at a fixture tree, the way useTempLock
// points the partition lock at a scratch file. Without it Run's release path
// would look for the child under the node's real cgroup mount.
func useTempRoot(t *testing.T, root string) {
	t.Helper()
	old := CgroupRoot
	CgroupRoot = root
	t.Cleanup(func() { CgroupRoot = old })
}

func readSet(t *testing.T, path string) cpuset.Set {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	text := strings.TrimSpace(string(data))
	if text == "" {
		return cpuset.Set{}
	}
	set, err := cpuset.Parse(text)
	if err != nil {
		t.Fatal(err)
	}
	return set
}

// TestReleasePartitionTakesBackTheAncestors: the create unioned the exclusive
// CPUs into every ancestor's cpuset.cpus.exclusive. A teardown that removes
// only the child leaves them reserved for a partition that no longer exists,
// and the next container that wants those CPUs is refused with EINVAL.
func TestReleasePartitionTakesBackTheAncestors(t *testing.T) {
	useTempLock(t)
	root := t.TempDir()
	plan := setupPartitionFixture(t, root, "kubepods/pod/ctr", cpuset.New(1, 5))

	for _, dir := range []string{"kubepods", "kubepods/pod", "kubepods/pod/ctr"} {
		p := filepath.Join(root, dir, "cpuset.cpus.exclusive")
		if got := readSet(t, p); !got.Equals(cpuset.New(1, 5)) {
			t.Fatalf("%s = %s after the create, want 1,5", p, got)
		}
	}

	if err := ReleasePartition(root, plan.Child, cpuset.New(1, 5)); err != nil {
		t.Fatal(err)
	}

	for _, dir := range []string{"kubepods", "kubepods/pod", "kubepods/pod/ctr"} {
		p := filepath.Join(root, dir, "cpuset.cpus.exclusive")
		if got := readSet(t, p); !got.IsEmpty() {
			t.Errorf("%s = %s after the release, want empty", p, got)
		}
	}
	if _, err := os.Stat(plan.Child); !os.IsNotExist(err) {
		t.Errorf("the child %s is still there after the release (err %v)", plan.Child, err)
	}
}

// TestReleasePartitionKeepsASiblingsReservation: two containers share every
// ancestor. Releasing one must leave the other's CPUs reserved there.
func TestReleasePartitionKeepsASiblingsReservation(t *testing.T) {
	useTempLock(t)
	root := t.TempDir()
	planA := setupPartitionFixture(t, root, "kubepods/podA.slice/cri-a.scope", cpuset.New(1, 5))
	planB := setupPartitionFixture(t, root, "kubepods/podB.slice/cri-b.scope", cpuset.New(2, 3))

	if err := ReleasePartition(root, planA.Child, cpuset.New(1, 5)); err != nil {
		t.Fatal(err)
	}

	for _, dir := range []string{"kubepods", "kubepods/podB.slice", "kubepods/podB.slice/cri-b.scope"} {
		p := filepath.Join(root, dir, "cpuset.cpus.exclusive")
		if got := readSet(t, p); !got.Equals(cpuset.New(2, 3)) {
			t.Errorf("%s = %s after releasing A, want B's 2,3 to stay", p, got)
		}
	}
	if _, err := os.Stat(planA.Child); !os.IsNotExist(err) {
		t.Errorf("A's child %s is still there (err %v)", planA.Child, err)
	}
	if _, err := os.Stat(planB.Child); err != nil {
		t.Errorf("B's child %s must survive A's release: %v", planB.Child, err)
	}
	// A's own ancestors are now free of both.
	for _, dir := range []string{"kubepods/podA.slice", "kubepods/podA.slice/cri-a.scope"} {
		p := filepath.Join(root, dir, "cpuset.cpus.exclusive")
		if got := readSet(t, p); !got.IsEmpty() {
			t.Errorf("%s = %s after the release, want empty", p, got)
		}
	}
}

// TestReleasePartitionIsIdempotent: the container may already be gone when the
// poststop hook runs, and a second release must not fail or take a sibling's
// CPUs with it.
func TestReleasePartitionIsIdempotent(t *testing.T) {
	useTempLock(t)
	root := t.TempDir()
	plan := setupPartitionFixture(t, root, "kubepods/pod/ctr", cpuset.New(1, 5))

	if err := ReleasePartition(root, plan.Child, cpuset.New(1, 5)); err != nil {
		t.Fatal(err)
	}
	if err := ReleasePartition(root, plan.Child, cpuset.New(1, 5)); err != nil {
		t.Fatalf("a second release must succeed: %v", err)
	}
}

// TestReleasePartitionOnAChildTheKernelAlreadyRemoved: the runtime may have
// torn the cgroup down first. The exclusive set is passed in on the command
// line, so the ancestor reservation is still taken back without it.
func TestReleasePartitionOnAChildTheKernelAlreadyRemoved(t *testing.T) {
	useTempLock(t)
	root := t.TempDir()
	plan := setupPartitionFixture(t, root, "kubepods/pod/ctr", cpuset.New(1, 5))
	if err := os.RemoveAll(plan.Child); err != nil {
		t.Fatal(err)
	}

	if err := ReleasePartition(root, plan.Child, cpuset.New(1, 5)); err != nil {
		t.Fatal(err)
	}
	for _, dir := range []string{"kubepods", "kubepods/pod", "kubepods/pod/ctr"} {
		p := filepath.Join(root, dir, "cpuset.cpus.exclusive")
		if got := readSet(t, p); !got.IsEmpty() {
			t.Errorf("%s = %s, want the reservation taken back", p, got)
		}
	}
}

// TestReleasePartitionKeepsAnUncountedCPU: a CPU that some other partition
// holds under the same ancestor is not this container's to give back.
func TestReleasePartitionKeepsAnUncountedCPU(t *testing.T) {
	useTempLock(t)
	root := t.TempDir()
	plan := setupPartitionFixture(t, root, "kubepods/pod/ctr", cpuset.New(1, 5))
	// Someone else's partition, under a different container but the same
	// ancestors, holds CPU 1 as well.
	other := writeTree(t, root, "kubepods/pod/other/ergon-isolated")
	for name, seed := range map[string]string{
		"cpuset.cpus":            "1\n",
		"cpuset.cpus.partition":  "isolated\n",
		"cpuset.cpus.exclusive":  "1\n",
		"cgroup.type":            "threaded\n",
		"cgroup.subtree_control": "\n",
	} {
		if err := os.WriteFile(filepath.Join(other, name), []byte(seed), 0o644); err != nil {
			t.Fatal(err)
		}
	}

	if err := ReleasePartition(root, plan.Child, cpuset.New(1, 5)); err != nil {
		t.Fatal(err)
	}
	// CPU 1 is still the other partition's, so it stays reserved at the shared
	// levels. CPU 5 is only this container's, so it goes.
	for _, dir := range []string{"kubepods", "kubepods/pod"} {
		p := filepath.Join(root, dir, "cpuset.cpus.exclusive")
		if got := readSet(t, p); !got.Equals(cpuset.New(1)) {
			t.Errorf("%s = %s, want just the other partition's CPU 1", p, got)
		}
	}
}

// TestRunReleasesWithoutAPid: the poststop hook runs after the container's
// processes are gone, so a release must not need one.
func TestRunReleasesWithoutAPid(t *testing.T) {
	useTempLock(t)
	root := t.TempDir()
	useTempRoot(t, root)
	plan := setupPartitionFixture(t, root, "kubepods/pod/ctr", cpuset.New(1, 5))

	var stderr strings.Builder
	err := Run(strings.NewReader(`{"id":"ctr"}`), &stderr,
		cpuset.Set{}, &PartitionRequest{Exclusive: cpuset.New(1, 5), Child: plan.Child, Release: true})
	if err != nil {
		t.Fatalf("Run: %v (stderr %q)", err, stderr.String())
	}
	if _, err := os.Stat(plan.Child); !os.IsNotExist(err) {
		t.Errorf("the child %s is still there (err %v)", plan.Child, err)
	}
	if got := readSet(t, filepath.Join(root, "kubepods", "cpuset.cpus.exclusive")); !got.IsEmpty() {
		t.Errorf("kubepods reservation = %s, want it taken back", got)
	}
}
