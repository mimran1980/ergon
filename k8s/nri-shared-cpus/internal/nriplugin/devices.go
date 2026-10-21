// SPDX-License-Identifier: Apache-2.0

package nriplugin

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"

	"github.com/containerd/nri/pkg/api"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/latency"
)

// deviceNumbers returns the major and minor of a character device from its
// inode. A missing path gives 0,0: runc still creates the node and the open
// in the pod fails with a clear errno.
func deviceNumbers(path string) (int64, int64) {
	info, err := os.Stat(path)
	if err != nil {
		return 0, 0
	}
	st, ok := info.Sys().(*syscall.Stat_t)
	if !ok {
		return 0, 0
	}
	rdev := uint64(st.Rdev) //nolint:unconvert // Rdev is uint32 on some arches
	return int64((rdev >> 8) & 0xfff), int64((rdev & 0xff) | ((rdev >> 12) & 0xfff00))
}

// requestSeccomp converts the container's NRI seccomp policy to the pure
// type. Every field is carried across: a policy the runtime built with
// argument filters or a per-rule errno must come out of the plugin with
// those restrictions still in force.
func requestSeccomp(c *api.Container) *latency.SeccompPolicy {
	sp := c.GetLinux().GetSeccompPolicy()
	if sp == nil {
		return nil
	}
	out := &latency.SeccompPolicy{
		DefaultAction:    sp.GetDefaultAction(),
		Architectures:    append([]string(nil), sp.GetArchitectures()...),
		Flags:            append([]string(nil), sp.GetFlags()...),
		ListenerPath:     sp.GetListenerPath(),
		ListenerMetadata: sp.GetListenerMetadata(),
	}
	if e := sp.GetDefaultErrno(); e != nil {
		v := e.GetValue()
		out.DefaultErrno = &v
	}
	for _, sc := range sp.GetSyscalls() {
		rule := latency.SeccompSyscall{
			Names:  append([]string(nil), sc.GetNames()...),
			Action: sc.GetAction(),
		}
		if e := sc.GetErrnoRet(); e != nil {
			v := e.GetValue()
			rule.ErrnoRet = &v
		}
		for _, a := range sc.GetArgs() {
			rule.Args = append(rule.Args, latency.SeccompArg{
				Index:    a.GetIndex(),
				Value:    a.GetValue(),
				ValueTwo: a.GetValueTwo(),
				Op:       a.GetOp(),
			})
		}
		out.Syscalls = append(out.Syscalls, rule)
	}
	return out
}

// requestHugepageLimitBytes is the container's hugepages-2Mi limit in bytes
// (0 when it asks for none). The sized hugetlbfs mount is exactly this size:
// Kubernetes isolates hugepages at container scope, so two containers cannot
// overcommit one mount. A hugepage limit for any other page size is ignored
// here; the plugin only mounts 2 MiB pages.
//
// Kubernetes does not put these limits on the OCI/NRI container object. It
// writes hugetlb.2MB.max on the pod cgroup instead, so the cgroup is the
// source of truth and the NRI resources are only a fallback.
func requestHugepageLimitBytes(c *api.Container) int64 {
	for _, l := range c.GetLinux().GetResources().GetHugepageLimits() {
		if is2MiB(l.GetPageSize()) {
			return int64(l.GetLimit())
		}
	}
	return 0
}

// hugepageLimitFromCgroup reads the pod cgroup's hugetlb.2MB.max. Both
// cgroupfs ("kubepods/pod") and systemd ("kubelet-kubepods-pod….slice") paths
// are accepted; the latter is what kind and EKS send. podPath is the pod's own
// cgroups path; when empty it is derived from the container's by dropping the
// ":cri-containerd:<id>" tail. A missing file or "max" means the pod asked for
// no hugepages: the caller fails closed on 0.
func hugepageLimitFromCgroup(cgroupRoot, podPath, cgroupsPath string) int64 {
	path := podPath
	if path == "" {
		path = podCgroupFrom(cgroupsPath)
	}
	if path == "" {
		return 0
	}
	dir, err := cgroupFsPath(cgroupRoot, path)
	if err != nil {
		return 0
	}
	b, err := os.ReadFile(filepath.Join(dir, "hugetlb.2MB.max"))
	if err != nil {
		return 0
	}
	s := strings.TrimSpace(string(b))
	if s == "" || s == "max" {
		return 0
	}
	n, err := strconv.ParseInt(s, 10, 64)
	if err != nil || n <= 0 {
		return 0
	}
	return n
}

// podCgroupFrom drops the ":cri-containerd:<id>" tail of an OCI cgroups path,
// leaving the pod's cgroup. An empty input gives empty.
func podCgroupFrom(cgroupsPath string) string {
	if cgroupsPath == "" {
		return ""
	}
	if i := strings.IndexByte(cgroupsPath, ':'); i >= 0 {
		return cgroupsPath[:i]
	}
	return cgroupsPath
}

// cgroupFsPath maps an OCI/NRI cgroupsPath to a real cgroup filesystem
// directory under cgroupRoot. Two shapes are accepted:
//
//   - cgroupfs driver: a path ("kubepods/pod/ctr") or an absolute path;
//   - systemd driver (kind, EKS): a unit name
//     "a-b-c.slice[:cri-containerd:<id>]", expanded into nested directories
//     (a.slice/a-b.slice/a-b-c.slice) and, for the container tail, a
//     cri-containerd-<id>.scope child.
//
// An empty path, or one that escapes cgroupRoot, is a typed refusal.
func cgroupFsPath(cgroupRoot, cgroupsPath string) (string, error) {
	if cgroupsPath == "" {
		return "", refuse(0, "bad-cgroup", "the container has no cgroup path")
	}
	root := filepath.Clean(cgroupRoot)
	if strings.Contains(cgroupsPath, "/") || filepath.IsAbs(cgroupsPath) {
		dir := cgroupsPath
		if !filepath.IsAbs(dir) {
			dir = filepath.Join(root, dir)
		}
		dir = filepath.Clean(dir)
		if dir == root || !strings.HasPrefix(dir, root+"/") {
			return "", refuse(0, "bad-cgroup", "cgroup path %q is not under %s", cgroupsPath, root)
		}
		return dir, nil
	}
	sliceName := cgroupsPath
	var scope string
	if i := strings.IndexByte(cgroupsPath, ':'); i >= 0 {
		sliceName = cgroupsPath[:i]
		rest := cgroupsPath[i+1:]
		// "cri-containerd:<id>" becomes cri-containerd-<id>.scope. systemd
		// unit names use '-' where the cgroup path used ':'.
		scope = strings.ReplaceAll(rest, ":", "-") + ".scope"
	}
	if !strings.HasSuffix(sliceName, ".slice") {
		return "", refuse(0, "bad-cgroup", "cgroup path %q is neither a path nor a systemd slice", cgroupsPath)
	}
	parts := expandSystemdSlice(sliceName)
	dir := filepath.Join(append([]string{root}, parts...)...)
	if scope != "" {
		dir = filepath.Join(dir, scope)
	}
	dir = filepath.Clean(dir)
	if !strings.HasPrefix(dir, root+"/") {
		return "", refuse(0, "bad-cgroup", "cgroup path %q is not under %s", cgroupsPath, root)
	}
	// The nested expansion is what kubelet writes. A single-level layout
	// (the slice name as one directory directly under the cgroup root) is
	// also a real shape, and the flat one is what a bare unit name resolves
	// to when the hierarchy is not walked. Take it when the nested directory
	// is not there.
	if _, err := os.Stat(dir); err != nil {
		flat := filepath.Join(root, sliceName)
		if scope != "" {
			flat = filepath.Join(flat, scope)
		}
		flat = filepath.Clean(flat)
		if strings.HasPrefix(flat, root+"/") {
			if _, err := os.Stat(flat); err == nil {
				return flat, nil
			}
		}
	}
	return dir, nil
}

// expandSystemdSlice turns "a-b-c.slice" into its nested directories
// ["a.slice", "a-b.slice", "a-b-c.slice"]. Pod UIDs have their hyphens
// stripped by kubelet, so a simple split on '-' is unambiguous here.
func expandSystemdSlice(name string) []string {
	base := strings.TrimSuffix(name, ".slice")
	var out []string
	cur := ""
	for _, p := range strings.Split(base, "-") {
		if cur == "" {
			cur = p
		} else {
			cur += "-" + p
		}
		out = append(out, cur+".slice")
	}
	return out
}

// is2MiB reports whether an NRI/OCI/CRI hugepage page-size string names 2 MiB
// pages. Spellings seen in the wild: OCI "2MB"/"2M", Kubernetes resource
// "2Mi"/"2MiB", and the decimal byte count "2097152".
func is2MiB(pageSize string) bool {
	switch strings.ToLower(strings.TrimSpace(pageSize)) {
	case "2mb", "2m", "2mi", "2mib", "2097152":
		return true
	}
	return false
}
