// SPDX-License-Identifier: Apache-2.0

package cpuset

import (
	"strings"
	"testing"
)

// mustParse is a test helper: a bad list is a test bug, not a failure case.
func mustParse(t *testing.T, s string) Set {
	t.Helper()
	got, err := Parse(s)
	if err != nil {
		t.Fatalf("Parse(%q): %v", s, err)
	}
	return got
}

// cgroupRoot is the cgroup v2 mount point; containerCgroup is a realistic
// kubelet cgroup path under it.
const cgroupRoot = "/sys/fs/cgroup"
const containerCgroup = "/sys/fs/cgroup/kubepods.slice/kubepods-burstable.slice/kubepods-burstable-pod0abc.slice/cri-containerd-1def.scope"

func TestPartitionPlanChildHoldsOnlyExclusive(t *testing.T) {
	e, err := Parse("2,3")
	if err != nil {
		t.Fatal(err)
	}
	s, err := Parse("0")
	if err != nil {
		t.Fatal(err)
	}
	plan, err := PartitionPlan(cgroupRoot, containerCgroup, e, s)
	if err != nil {
		t.Fatal(err)
	}
	if want := containerCgroup + "/ergon-isolated"; plan.Child != want {
		t.Errorf("child = %q, want %q", plan.Child, want)
	}
	// The child's cpuset.cpus must be the exclusive CPUs only: never the
	// shared ones (isolating them would strip load balancing from helpers).
	for _, w := range plan.Writes {
		if w.Kind == WriteCpus {
			if w.Path != plan.Child+"/cpuset.cpus" {
				t.Errorf("cpuset.cpus write on %q, want only on the child", w.Path)
			}
			if w.Value != "2,3" {
				t.Errorf("child cpuset.cpus = %q, want 2,3", w.Value)
			}
		}
	}
}

func TestPartitionPlanWriteOrder(t *testing.T) {
	e := mustParse(t, "2,3")
	s := mustParse(t, "0")
	plan, err := PartitionPlan(cgroupRoot, containerCgroup, e, s)
	if err != nil {
		t.Fatal(err)
	}
	if len(plan.Writes) < 4 {
		t.Fatalf("got %d writes, want at least 4: %v", len(plan.Writes), plan.Writes)
	}
	// 1. mkdir the child first.
	if plan.Writes[0].Kind != WriteMkdir || plan.Writes[0].Path != plan.Child {
		t.Errorf("first write = %+v, want mkdir %s", plan.Writes[0], plan.Child)
	}
	// Last two: partition=isolated on the child, and it must come after every
	// cpuset.cpus.exclusive write (the kernel needs exclusive down the
	// hierarchy before a remote partition is accepted).
	last := plan.Writes[len(plan.Writes)-1]
	if last.Kind != WritePartition || last.Value != "isolated" {
		t.Errorf("last write = %+v, want partition=isolated", last)
	}
	if last.Path != plan.Child+"/cpuset.cpus.partition" {
		t.Errorf("partition write on %q, want the child", last.Path)
	}
	seenCpus := false
	for i, w := range plan.Writes {
		switch w.Kind {
		case WriteCpus:
			seenCpus = true
		case WriteExclusive, WriteExclusiveAdd:
			if !seenCpus {
				t.Errorf("write %d (%+v) is an exclusive write before the child's cpuset.cpus", i, w)
			}
			if i >= len(plan.Writes)-1 {
				t.Errorf("exclusive write %d comes after the partition write", i)
			}
		case WritePartition:
			if i != len(plan.Writes)-1 {
				t.Errorf("partition write at %d, want last", i)
			}
		}
	}
}

func TestPartitionPlanExclusiveDownTheHierarchy(t *testing.T) {
	e := mustParse(t, "2,3")
	s := mustParse(t, "0")
	plan, err := PartitionPlan(cgroupRoot, containerCgroup, e, s)
	if err != nil {
		t.Fatal(err)
	}
	// Every ancestor above the child that has a cpuset must get
	// cpuset.cpus.exclusive covering E, top-down, before the partition write.
	// Ancestors union into whatever is already reserved there and the child
	// replaces its own file, so both kinds count.
	var exclusivePaths []string
	for _, w := range plan.Writes {
		if w.Kind == WriteExclusive || w.Kind == WriteExclusiveAdd {
			exclusivePaths = append(exclusivePaths, w.Path)
		}
	}
	if len(exclusivePaths) < 3 {
		t.Fatalf("got %d exclusive writes %v, want one per ancestor plus the child", len(exclusivePaths), exclusivePaths)
	}
	// Top-down: each path is a prefix of the next.
	for i := 1; i < len(exclusivePaths); i++ {
		if !strings.HasPrefix(exclusivePaths[i], strings.TrimSuffix(exclusivePaths[i-1], "/cpuset.cpus.exclusive")) {
			t.Errorf("exclusive writes are not top-down: %v", exclusivePaths)
		}
	}
	// The container cgroup itself is included, and so is the child.
	foundContainer, foundChild := false, false
	for _, p := range exclusivePaths {
		if p == containerCgroup+"/cpuset.cpus.exclusive" {
			foundContainer = true
		}
		if p == plan.Child+"/cpuset.cpus.exclusive" {
			foundChild = true
		}
	}
	if !foundContainer {
		t.Errorf("no exclusive write on the container cgroup: %v", exclusivePaths)
	}
	if !foundChild {
		t.Errorf("no exclusive write on the child: %v", exclusivePaths)
	}
	// The cgroup root itself has no cpuset.cpus.exclusive: never write it.
	for _, p := range exclusivePaths {
		if p == "/sys/fs/cgroup/cpuset.cpus.exclusive" {
			t.Error("must not write cpuset.cpus.exclusive on the cgroup root")
		}
	}
}

func TestPartitionPlanNeverIsolatesShared(t *testing.T) {
	// E and S overlap: the child would hold a shared CPU and isolating it
	// would strip load balancing from the helpers. Must refuse.
	e := mustParse(t, "2,3")
	s := mustParse(t, "3,0")
	if _, err := PartitionPlan(cgroupRoot, containerCgroup, e, s); err == nil {
		t.Fatal("must refuse when the exclusive set overlaps the shared set")
	}
}

func TestPartitionPlanEmptyExclusive(t *testing.T) {
	e := Set{}
	s := mustParse(t, "0")
	if _, err := PartitionPlan(cgroupRoot, containerCgroup, e, s); err == nil {
		t.Fatal("must refuse an empty exclusive set")
	}
}

func TestPartitionPlanBadCgroup(t *testing.T) {
	e := mustParse(t, "2,3")
	s := mustParse(t, "0")
	if _, err := PartitionPlan(cgroupRoot, "", e, s); err == nil {
		t.Error("must refuse an empty cgroup path")
	}
	if _, err := PartitionPlan(cgroupRoot, "kubepods.scope", e, s); err == nil {
		t.Error("must refuse a relative cgroup path")
	}
	if _, err := PartitionPlan("", containerCgroup, e, s); err == nil {
		t.Error("must refuse an empty cgroup root")
	}
	if _, err := PartitionPlan("/sys/fs/cgroup", "/other/root/x.scope", e, s); err == nil {
		t.Error("must refuse a container cgroup outside the cgroup root")
	}
}

func TestPartitionPlanParentCpusUntouched(t *testing.T) {
	e := mustParse(t, "2,3")
	s := mustParse(t, "0")
	plan, err := PartitionPlan(cgroupRoot, containerCgroup, e, s)
	if err != nil {
		t.Fatal(err)
	}
	for _, w := range plan.Writes {
		if w.Kind == WriteCpus && w.Path != plan.Child+"/cpuset.cpus" {
			t.Errorf("write %+v touches cpuset.cpus outside the child", w)
		}
	}
}

func TestPartitionPlanExclusiveValueIsOnlyE(t *testing.T) {
	e := mustParse(t, "2,3")
	s := mustParse(t, "0")
	plan, err := PartitionPlan(cgroupRoot, containerCgroup, e, s)
	if err != nil {
		t.Fatal(err)
	}
	for _, w := range plan.Writes {
		if (w.Kind == WriteExclusive || w.Kind == WriteExclusiveAdd) && w.Value != "2,3" {
			t.Errorf("exclusive write %+v: value must be the exclusive CPUs only", w)
		}
	}
}

// TestPartitionPlanEnablesCpusetOnTheContainer is the P1 regression: a child
// cgroup has no cpuset.* files until the parent enables the controller in its
// cgroup.subtree_control. Writing cpuset.cpus on the child first fails with
// "Permission denied" on a real kernel. The plan has to write +cpuset on the
// container itself, before any cpuset write on the child.
func TestPartitionPlanEnablesCpusetOnTheContainer(t *testing.T) {
	e := mustParse(t, "2,3")
	s := mustParse(t, "0")
	plan, err := PartitionPlan(cgroupRoot, containerCgroup, e, s)
	if err != nil {
		t.Fatal(err)
	}
	at := -1
	for i, w := range plan.Writes {
		if w.Kind == WriteSubtreeControl {
			if w.Path != containerCgroup+"/cgroup.subtree_control" {
				t.Errorf("subtree_control write on %q, want the container cgroup", w.Path)
			}
			if w.Value != "cpuset" {
				t.Errorf("subtree_control value %q, want the cpuset controller", w.Value)
			}
			at = i
		}
	}
	if at < 0 {
		t.Fatal("no +cpuset write on the container's cgroup.subtree_control: the child would have no cpuset files at all")
	}
	for i, w := range plan.Writes {
		if w.Kind == WriteCpus && i < at {
			t.Errorf("child cpuset.cpus at %d comes before the +cpuset write at %d", i, at)
		}
	}
}

// TestPartitionPlanMakesTheChildThreaded is the other half of the P1: a
// populated container that enables a threaded controller becomes a threaded
// domain, and a child created under it is "domain invalid" — it accepts no
// task, with EOPNOTSUPP, until cgroup.type is written "threaded". The
// threaded child is still a legal cpuset partition, so one hot thread can
// live there and the helpers stay on the container's shared CPUs.
func TestPartitionPlanMakesTheChildThreaded(t *testing.T) {
	e := mustParse(t, "2,3")
	s := mustParse(t, "0")
	plan, err := PartitionPlan(cgroupRoot, containerCgroup, e, s)
	if err != nil {
		t.Fatal(err)
	}
	typeAt, subtreeAt := -1, -1
	for i, w := range plan.Writes {
		switch {
		case w.Kind == WriteSubtreeControl:
			subtreeAt = i
		case w.Kind == WriteCgroupType:
			typeAt = i
			if w.Path != plan.Child+"/cgroup.type" {
				t.Errorf("cgroup.type write on %q, want the child", w.Path)
			}
			if w.Value != "threaded" {
				t.Errorf("cgroup.type value %q, want threaded", w.Value)
			}
		}
	}
	if typeAt < 0 {
		t.Fatal("no cgroup.type=threaded write on the child: it stays domain invalid and takes no task")
	}
	if subtreeAt >= 0 && typeAt < subtreeAt {
		t.Errorf("cgroup.type at %d comes before the +cpuset write at %d", typeAt, subtreeAt)
	}
	for i, w := range plan.Writes {
		if w.Kind == WriteCpus && i < typeAt {
			t.Errorf("child cpuset.cpus at %d comes before cgroup.type at %d", i, typeAt)
		}
	}
}

// TestPartitionPlanOnlyTheChildReplaces is the P1 regression: cpuset.cpus.
// exclusive is a replace, not a union. Every directory a partition shares with
// another container's has to be written as a union with what is already
// reserved there, and only the new child — this container's alone — may be
// written outright. Getting that wrong starts a second pod by taking the
// exclusive CPUs away from the first one.
func TestPartitionPlanOnlyTheChildReplaces(t *testing.T) {
	e := mustParse(t, "2,3")
	s := mustParse(t, "0")
	plan, err := PartitionPlan(cgroupRoot, containerCgroup, e, s)
	if err != nil {
		t.Fatal(err)
	}
	replaced := 0
	for _, w := range plan.Writes {
		switch w.Kind {
		case WriteExclusive:
			replaced++
			if w.Path != plan.Child+"/cpuset.cpus.exclusive" {
				t.Errorf("replace write on %q, which a sibling can share; only the child may be replaced", w.Path)
			}
		case WriteExclusiveAdd:
			if w.Path == plan.Child+"/cpuset.cpus.exclusive" {
				t.Error("the child is this container's alone: it is written outright, not unioned")
			}
		}
	}
	if replaced != 1 {
		t.Errorf("%d replace writes, want exactly 1 (the child)", replaced)
	}
}
