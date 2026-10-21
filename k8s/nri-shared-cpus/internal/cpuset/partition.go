// SPDX-License-Identifier: Apache-2.0

package cpuset

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

// WriteKind is the kind of one ordered write in a partition plan.
type WriteKind string

const (
	// WriteMkdir creates the child cgroup directory. Value is empty.
	WriteMkdir WriteKind = "mkdir"
	// WriteCpus writes cpuset.cpus.
	WriteCpus WriteKind = "cpuset.cpus"
	// WriteExclusive writes cpuset.cpus.exclusive, replacing whatever is
	// there. Only the new child is ever written this way.
	WriteExclusive WriteKind = "cpuset.cpus.exclusive"
	// WriteExclusiveAdd unions Value into cpuset.cpus.exclusive rather than
	// replacing it. Every directory this partition shares with another
	// container's is written this way: cpuset.cpus.exclusive is a replace, so
	// writing only this container's CPUs there drops the CPUs of every
	// partition already under that ancestor, and a previously valid sibling
	// partition loses its exclusive CPUs.
	WriteExclusiveAdd WriteKind = "cpuset.cpus.exclusive+"
	// WritePartition writes cpuset.cpus.partition.
	WritePartition WriteKind = "cpuset.cpus.partition"
	// WriteSubtreeControl enables the controller named by Value in the
	// cgroup's cgroup.subtree_control, keeping whatever is already enabled.
	// A child cgroup has no cpuset.* files at all until the parent enables
	// the controller there: writing cpuset.cpus first is a write to a file
	// that does not exist. A populated container can only host a cpuset
	// child as a threaded domain (cgroup-v2.rst, "Threads").
	WriteSubtreeControl WriteKind = "cgroup.subtree_control+"
	// WriteCgroupType writes cgroup.type. A child created under a threaded
	// domain is "domain invalid": it takes no task, and the move fails with
	// EOPNOTSUPP, until this says "threaded". A threaded child is still a
	// legal cpuset partition root, which is what lets one hot thread live on
	// the exclusive CPUs while the helpers stay on the shared ones.
	WriteCgroupType WriteKind = "cgroup.type"
)

// Write is one filesystem write in a partition plan. Path is absolute; Value
// is the file content (empty for WriteMkdir), except for WriteExclusiveAdd
// where it is the CPU list to union into the current content.
type Write struct {
	Kind  WriteKind
	Path  string
	Value string
}

// Plan is the ordered setup that turns a child cgroup into an isolated
// cpuset partition holding only the exclusive CPUs. The kernel owns the
// order: cpuset.cpus.exclusive must be written down the hierarchy before the
// partition write is accepted (cgroup-v2.rst, "remote partition").
type Plan struct {
	// Child is the absolute path of the child cgroup.
	Child string
	// Writes are in apply order.
	Writes []Write
}

// PartitionPlan returns the child cgroup path and the ordered writes that
// make it a "isolated" partition of exactly exclusive.
//
// cgroupRoot is the cgroup v2 mount point (it has no cpuset.cpus.exclusive).
// containerCgroup is the container's own cgroup directory. exclusive is the
// container's exclusive CPUs (ERGON_ISOLATED_CPUS); shared is the CPUs helpers
// use (ERGON_SHARED_CPUS).
//
// The hierarchy is part of the plan, not a precondition. A child cgroup has
// no cpuset.* files until the container enables the controller in its
// cgroup.subtree_control, and a populated container can only host a cpuset
// child as a threaded domain: the child is then created "domain invalid" and
// takes no task until its cgroup.type says "threaded". A threaded child is
// still a legal cpuset partition root, so one hot thread can live on the
// exclusive CPUs while the helpers stay on the container's shared CPUs.
//
// The plan never writes "isolated" on a cgroup that holds shared CPUs: the
// child's cpuset.cpus is exactly exclusive, and an exclusive/shared overlap is
// refused, because an isolated shared CPU would take load balancing away from
// the helpers.
func PartitionPlan(cgroupRoot, containerCgroup string, exclusive, shared Set) (*Plan, error) {
	if cgroupRoot == "" {
		return nil, fmt.Errorf("partition: the cgroup root is empty")
	}
	if !filepath.IsAbs(cgroupRoot) {
		return nil, fmt.Errorf("partition: the cgroup root %q is not absolute", cgroupRoot)
	}
	if containerCgroup == "" {
		return nil, fmt.Errorf("partition: the container cgroup is empty")
	}
	if !filepath.IsAbs(containerCgroup) {
		return nil, fmt.Errorf("partition: the container cgroup %q is not absolute", containerCgroup)
	}
	root := filepath.Clean(cgroupRoot)
	dir := filepath.Clean(containerCgroup)
	if dir == root {
		return nil, fmt.Errorf("partition: the container cgroup is the cgroup root")
	}
	if !strings.HasPrefix(dir, root+"/") {
		return nil, fmt.Errorf("partition: the container cgroup %q is not under the cgroup root %q", containerCgroup, cgroupRoot)
	}
	if exclusive.IsEmpty() {
		return nil, fmt.Errorf("partition: the exclusive CPU list is empty")
	}
	if both := exclusive.Intersection(shared); !both.IsEmpty() {
		return nil, fmt.Errorf("partition: CPU %s is both exclusive and shared; an isolated partition must not hold shared CPUs", both)
	}

	child := filepath.Join(dir, "ergon-isolated")
	plan := &Plan{Child: child}
	plan.Writes = append(plan.Writes, Write{Kind: WriteMkdir, Path: child})

	// The controller has to be enabled on the container before the child can
	// have cpuset files, and the child has to be threaded before it can take
	// a task. The kernel owns that order too: a populated container that
	// enables a threaded controller becomes a threaded domain, and a child
	// created under a threaded domain is "domain invalid" until its
	// cgroup.type says "threaded".
	plan.Writes = append(plan.Writes, Write{
		Kind:  WriteSubtreeControl,
		Path:  filepath.Join(dir, "cgroup.subtree_control"),
		Value: "cpuset",
	})
	plan.Writes = append(plan.Writes, Write{
		Kind:  WriteCgroupType,
		Path:  filepath.Join(child, "cgroup.type"),
		Value: "threaded",
	})

	plan.Writes = append(plan.Writes, Write{Kind: WriteCpus, Path: filepath.Join(child, "cpuset.cpus"), Value: exclusive.List()})

	// cpuset.cpus.exclusive down the hierarchy, outermost first, so each
	// level's exclusive.effective is non-empty before its child claims the
	// CPUs. The cgroup root has no such file.
	//
	// Every level above the new child is shared with whatever else lives
	// under it, so it is a union with what is already reserved there: two
	// pods in different containers share kubepods.slice, and replacing that
	// file with only this container's CPUs would take the exclusive CPUs away
	// from the partition the first pod is already running in. The child is
	// this container's alone, so it is written outright. Concurrent hooks
	// update the shared levels under one lock in ApplyPartition.
	for _, ancestor := range Ancestors(root, dir) {
		plan.Writes = append(plan.Writes, Write{
			Kind:  WriteExclusiveAdd,
			Path:  filepath.Join(ancestor, "cpuset.cpus.exclusive"),
			Value: exclusive.List(),
		})
	}
	// The child's own value is explicit rather than implicit, and it holds
	// only this container's exclusive CPUs.
	plan.Writes = append(plan.Writes, Write{
		Kind:  WriteExclusive,
		Path:  filepath.Join(child, "cpuset.cpus.exclusive"),
		Value: exclusive.List(),
	})

	plan.Writes = append(plan.Writes, Write{
		Kind:  WritePartition,
		Path:  filepath.Join(child, "cpuset.cpus.partition"),
		Value: "isolated",
	})
	return plan, nil
}

// Ancestors returns every directory between cgroupRoot (exclusive) and dir
// (inclusive), outermost first. That is the path a remote partition's
// cpuset.cpus.exclusive has to be written down.
func Ancestors(root, dir string) []string {
	var out []string
	for cur := dir; cur != root; cur = filepath.Dir(cur) {
		out = append(out, cur)
	}
	// Outermost first.
	for i, j := 0, len(out)-1; i < j; i, j = i+1, j-1 {
		out[i], out[j] = out[j], out[i]
	}
	return out
}

// ExclusiveFile is the kernel file a remote partition writes its ancestor
// reservations to. It exists on non-root cpuset-enabled cgroups from Linux
// 6.7 (cgroup-v2.rst, "cpuset.cpus.exclusive"); writing it down the hierarchy
// is mandatory for a remote partition, which is what this plan makes. A
// kernel without it cannot host one at all.
//
// The filesystem is the authority, not osrelease: a vendor kernel that
// backported the file provides the same one, and a version gate would refuse
// it for no reason.
const ExclusiveFile = "cpuset.cpus.exclusive"

// MissingExclusive rewrites an error from an access to ExclusiveFile into what
// a bare "no such file" actually means. Two things produce it: a kernel that
// does not provide the file at all, and a cgroup whose cpuset controller is
// not enabled. Either way a remote partition cannot be made there, and the
// bare ENOENT does not say so. Every other error is returned unchanged.
func MissingExclusive(path string, err error) error {
	if filepath.Base(path) != ExclusiveFile {
		return err
	}
	if !errors.Is(err, fs.ErrNotExist) {
		return err
	}
	return fmt.Errorf("%s does not exist: a remote partition needs %s (Linux 6.7 or a vendor backport of it), and the cpuset controller must be enabled on that cgroup: %w",
		path, ExclusiveFile, err)
}

// HasExclusive reports whether ExclusiveFile exists on a cgroup that has the
// cpuset controller enabled. A cgroup with cpuset.cpus is the probe: the file
// sits next to it. dir is where to start looking — its chain up to root first,
// then root's children. probed is false when nothing found has the cpuset
// controller enabled, so the filesystem cannot answer yet: the caller then
// lets the write itself report the missing file.
func HasExclusive(root, dir string) (present, probed bool) {
	for _, d := range Ancestors(root, dir) {
		if _, err := os.Stat(filepath.Join(d, "cpuset.cpus")); err != nil {
			continue
		}
		_, err := os.Stat(filepath.Join(d, ExclusiveFile))
		return err == nil, true
	}
	entries, err := os.ReadDir(root)
	if err != nil {
		return false, false
	}
	for _, e := range entries {
		if !e.IsDir() {
			continue
		}
		d := filepath.Join(root, e.Name())
		if _, err := os.Stat(filepath.Join(d, "cpuset.cpus")); err != nil {
			continue
		}
		_, err := os.Stat(filepath.Join(d, ExclusiveFile))
		return err == nil, true
	}
	return false, false
}
