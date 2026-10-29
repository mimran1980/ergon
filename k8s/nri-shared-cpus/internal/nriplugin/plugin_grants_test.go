// SPDX-License-Identifier: Apache-2.0

package nriplugin

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/containerd/nri/pkg/api"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/latency"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

// withHugepages adds a hugepage limit to a test container.
func withHugepages(c *api.Container, pageSize string, limit uint64) *api.Container {
	c.Linux.Resources.HugepageLimits = append(c.Linux.Resources.HugepageLimits,
		&api.HugepageLimit{PageSize: pageSize, Limit: limit})
	return c
}

// withCgroupsPath sets the container's cgroup path (for the partition child).
func withCgroupsPath(c *api.Container, p string) *api.Container {
	c.Linux.CgroupsPath = p
	return c
}

func adjEnv(adj *api.ContainerAdjustment) map[string]string {
	m := map[string]string{}
	for _, e := range adj.GetEnv() {
		m[e.Key] = e.Value
	}
	return m
}

// --- item 1: sized hugetlbfs mount ---

func TestCreateHugepageMountSizedNoCgroupLimit(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := withHugepages(ctr("ctr-1", "1,5", nil, map[string]string{
		names.HugepagesPrefix + "app": "2M",
	}), "2MB", 256*1024*1024)
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil {
		t.Fatal(err)
	}
	if adj == nil {
		t.Fatal("no adjustment")
	}
	var mount *api.Mount
	for _, m := range adj.GetMounts() {
		if m.GetDestination() == latency.HugepageMountPoint {
			mount = m
		}
	}
	if mount == nil {
		t.Fatalf("no hugetlbfs mount at %s; mounts are %v", latency.HugepageMountPoint, adj.GetMounts())
	}
	if mount.GetType() != "hugetlbfs" {
		t.Errorf("mount type = %q, want hugetlbfs", mount.GetType())
	}
	opts := strings.Join(mount.GetOptions(), ",")
	if !strings.Contains(opts, "pagesize=2M") {
		t.Errorf("mount options %v must carry pagesize=2M", mount.GetOptions())
	}
	if !strings.Contains(opts, "size=268435456") {
		t.Errorf("mount options %v must carry size=<the hugepages-2Mi limit>", mount.GetOptions())
	}
	// Kubernetes already put the hugetlb cgroup limit on this container from
	// the pod spec's hugepages-2Mi. Re-sending it as an NRI hugepage limit
	// made containerd SEGV on apply (kind e2e). The plugin only mounts.
	limits := adj.GetLinux().GetResources().GetHugepageLimits()
	if len(limits) != 0 {
		t.Errorf("hugepage limits are %v, want none: the cgroup limit comes from the pod spec, not from NRI", limits)
	}
	env := adjEnv(adj)
	if env[names.EnvHugepagesDir] != latency.HugepageMountPoint {
		t.Errorf("%s = %q, want %q", names.EnvHugepagesDir, env[names.EnvHugepagesDir], latency.HugepageMountPoint)
	}
	if env[names.EnvHugepageSize] != "2097152" {
		t.Errorf("%s = %q, want 2097152", names.EnvHugepageSize, env[names.EnvHugepageSize])
	}
	if env[latency.EnvAeronPageSize] != "2097152" {
		t.Errorf("%s = %q, want 2097152", latency.EnvAeronPageSize, env[latency.EnvAeronPageSize])
	}
	if env[latency.EnvAeronSparse] != "false" {
		t.Errorf("%s = %q, want false", latency.EnvAeronSparse, env[latency.EnvAeronSparse])
	}
	if _, ok := env[latency.EnvAeronDir]; ok {
		t.Error("the app points AERON_DIR at $ERGON_HUGEPAGES_DIR/aeron itself; the plugin must not set it")
	}
}

func TestCreateHugepageMountSizeFromPodCgroup(t *testing.T) {
	// Kubernetes never puts hugepages-2Mi on the OCI/NRI container. It writes
	// hugetlb.2MB.max on the pod cgroup. CreateContainer must fall back to
	// that file when NRI has no HugepageLimits, or the sized mount is refused
	// for a pod that does have a limit.
	p := testPlugin(t, "0,4")
	root := t.TempDir()
	p.cfg.CgroupRoot = root
	podPath := "kubepods/pod-huge"
	if err := os.MkdirAll(filepath.Join(root, podPath), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, podPath, "hugetlb.2MB.max"), []byte("16777216\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	pd := pod()
	pd.Linux = &api.LinuxPodSandbox{CgroupsPath: podPath}
	c := ctr("ctr-1", "1,5", nil, map[string]string{
		names.HugepagesPrefix + "app": "2M",
	})
	c.Linux.CgroupsPath = podPath + ":cri-containerd:cid-huge"
	adj, _, err := p.CreateContainer(context.Background(), pd, c)
	if err != nil {
		t.Fatal(err)
	}
	var mount *api.Mount
	for _, m := range adj.GetMounts() {
		if m.GetDestination() == latency.HugepageMountPoint {
			mount = m
		}
	}
	if mount == nil {
		t.Fatalf("no hugetlbfs mount; mounts are %v", adj.GetMounts())
	}
	opts := strings.Join(mount.GetOptions(), ",")
	if !strings.Contains(opts, "size=16777216") {
		t.Errorf("mount options %v must carry size=16777216 from the pod cgroup", mount.GetOptions())
	}
}

func TestCreateHugepageWithoutLimitRefused(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", nil, map[string]string{
		names.HugepagesPrefix + "app": "2M",
	})
	_, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err == nil {
		t.Fatal("the annotation without a hugepages-2Mi limit must refuse the create")
	}
	if !strings.Contains(err.Error(), "hugepages-2Mi") {
		t.Errorf("refusal %q must name the fix", err)
	}
}

// --- item 2: cpu-partition child cgroup ---

func TestCreateCpuPartitionInjectsHookAndEnv(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := withCgroupsPath(ctr("ctr-1", "1,5", nil, map[string]string{
		names.CpuPartitionPrefix + "app": names.CpuPartitionValue,
	}), "kubepods/pod-1/ctr-1")
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil {
		t.Fatal(err)
	}
	if adj == nil {
		t.Fatal("no adjustment")
	}
	env := adjEnv(adj)
	want := "/sys/fs/cgroup/kubepods/pod-1/ctr-1/" + names.CpuPartitionChild
	if env[names.EnvIsolatedCgroup] != want {
		t.Errorf("%s = %q, want %q", names.EnvIsolatedCgroup, env[names.EnvIsolatedCgroup], want)
	}
	hooks := adj.GetHooks()
	if hooks == nil || len(hooks.GetCreateRuntime()) != 1 {
		t.Fatalf("hooks are %v, want one createRuntime hook for the partition", hooks)
	}
	args := hooks.GetCreateRuntime()[0].GetArgs()
	joined := strings.Join(args, " ")
	if !strings.Contains(joined, "--isolated 1,5") {
		t.Errorf("hook args %v must carry --isolated 1,5", args)
	}
	if !strings.Contains(joined, "--partition-child "+want) {
		t.Errorf("hook args %v must carry --partition-child %s", args, want)
	}
	// A partition also needs its poststop: the create's ancestor reservations
	// have to come back down, or the next container that wants those CPUs is
	// refused and the isolation of a sibling can be broken by a late release.
	if len(hooks.GetPoststop()) != 1 {
		t.Fatalf("poststop hooks are %v, want one to release the partition", hooks.GetPoststop())
	}
	release := strings.Join(hooks.GetPoststop()[0].GetArgs(), " ")
	if !strings.Contains(release, "--isolated 1,5") || !strings.Contains(release, "--release-child "+want) {
		t.Errorf("poststop args %v must release %s with --isolated 1,5", hooks.GetPoststop()[0].GetArgs(), want)
	}
}

func TestCreateCpuPartitionWithoutSharedOptIn(t *testing.T) {
	p := testPlugin(t, "0,4")
	// No millicpu marker: the partition grant stands alone on a Guaranteed
	// integer-CPU container. The exclusive set is two whole cores (1,5 and
	// 2,6 under n/n+4 SMT): a core whose other thread is unmanaged is refused,
	// which is the point of the core-layout rule.
	c := withCgroupsPath(ctr("ctr-1", "1,2,5,6", nil, map[string]string{
		names.CpuPartitionPrefix + "app": names.CpuPartitionValue,
	}), "kubepods/pod-1/ctr-1")
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil {
		t.Fatal(err)
	}
	if adj == nil {
		t.Fatal("no adjustment")
	}
	// The cpuset is not widened: only the shared-CPU opt-in widens it.
	if cpu := adj.GetLinux().GetResources().GetCpu(); cpu != nil && cpu.GetCpus() != "" {
		t.Errorf("cpuset was widened to %q; a partition-only container keeps the kubelet's", cpu.GetCpus())
	}
	env := adjEnv(adj)
	if !strings.HasSuffix(env[names.EnvIsolatedCgroup], "/"+names.CpuPartitionChild) {
		t.Errorf("%s = %q, want the ergon-isolated child", names.EnvIsolatedCgroup, env[names.EnvIsolatedCgroup])
	}
	hooks := adj.GetHooks()
	if hooks == nil || len(hooks.GetCreateRuntime()) != 1 {
		t.Fatalf("hooks are %v, want one createRuntime hook", hooks)
	}
	args := hooks.GetCreateRuntime()[0].GetArgs()
	if !strings.Contains(strings.Join(args, " "), "--isolated 1,2,5,6") {
		t.Errorf("hook args %v must name the exclusive CPUs 1,2,5,6", args)
	}
	if len(hooks.GetPoststop()) != 1 {
		t.Fatalf("poststop hooks are %v, want one to release the partition", hooks.GetPoststop())
	}
	if !strings.Contains(strings.Join(hooks.GetPoststop()[0].GetArgs(), " "), "--release-child") {
		t.Errorf("poststop args %v must carry --release-child", hooks.GetPoststop()[0].GetArgs())
	}
}

func TestCreateCpuPartitionNoCgroupsPathRefused(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", nil, map[string]string{
		names.CpuPartitionPrefix + "app": names.CpuPartitionValue,
	})
	_, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err == nil {
		t.Fatal("the partition needs the container's cgroup path; refuse without one")
	}
	if !strings.Contains(err.Error(), "cgroup") {
		t.Errorf("refusal %q must name the cgroup path", err)
	}
}

func TestCreateCpuPartitionSharedOverlapRefused(t *testing.T) {
	p := testPlugin(t, "0,4")
	// CPU 4 is shared and also in the container's cpuset: never isolate it.
	c := withCgroupsPath(ctr("ctr-1", "4,5", nil, map[string]string{
		names.CpuPartitionPrefix + "app": names.CpuPartitionValue,
	}), "kubepods/pod-1/ctr-1")
	_, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err == nil {
		t.Fatal("a shared CPU must never be isolated; refuse")
	}
	if !strings.Contains(err.Error(), "shared") {
		t.Errorf("refusal %q must name the shared CPUs", err)
	}
}

// --- item 5: device grants ---

func TestCreateCpuqosDeviceGranted(t *testing.T) {
	p := testPlugin(t, "0,4")
	p.cfg.GOARCH = "amd64"
	p.cfg.HostCpuDmaLatency = "/dev/cpu_dma_latency"
	c := ctr("ctr-1", "1,5", nil, map[string]string{
		names.CpuqosPrefix + "app": "1",
	})
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil {
		t.Fatal(err)
	}
	found := false
	for _, d := range adj.GetLinux().GetDevices() {
		if d.GetPath() == "/dev/cpu_dma_latency" {
			found = true
			if d.GetType() != "c" {
				t.Errorf("device type = %q, want c", d.GetType())
			}
		}
	}
	if !found {
		t.Fatalf("no /dev/cpu_dma_latency in the adjustment; devices are %v", adj.GetLinux().GetDevices())
	}
	if adjEnv(adj)[names.EnvCpuDmaLatency] != "/dev/cpu_dma_latency" {
		t.Errorf("%s = %q", names.EnvCpuDmaLatency, adjEnv(adj)[names.EnvCpuDmaLatency])
	}
}

func TestCreatePtpAddsNodeFactsMount(t *testing.T) {
	p := testPlugin(t, "0,4")
	p.cfg.HostPtp = []string{"/dev/ptp0"}
	c := ctr("ctr-1", "1,5", nil, map[string]string{
		names.PtpPrefix + "app": "auto",
	})
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil {
		t.Fatal(err)
	}
	found := false
	for _, m := range adj.GetMounts() {
		if m.GetDestination() == names.NodeFactsMountPoint {
			found = true
			if m.GetSource() != names.NodeFactsPath {
				t.Errorf("node facts source = %q, want %q", m.GetSource(), names.NodeFactsPath)
			}
			if !strings.Contains(strings.Join(m.GetOptions(), ","), "ro") {
				t.Errorf("node facts options %v must be read-only", m.GetOptions())
			}
		}
	}
	if !found {
		t.Fatalf("no node facts bind mount; mounts are %v", adj.GetMounts())
	}
	if adjEnv(adj)[names.EnvNodeFacts] != names.NodeFactsMountPoint {
		t.Errorf("%s = %q", names.EnvNodeFacts, adjEnv(adj)[names.EnvNodeFacts])
	}
}

// --- pure helper: the partition child path ---

func TestPartitionChild(t *testing.T) {
	for _, tc := range []struct {
		name, root, path, want string
	}{
		{"relative", "/sys/fs/cgroup", "kubepods/pod/ctr", "/sys/fs/cgroup/kubepods/pod/ctr/ergon-isolated"},
		{"absolute", "/sys/fs/cgroup", "/sys/fs/cgroup/kubepods/pod/ctr", "/sys/fs/cgroup/kubepods/pod/ctr/ergon-isolated"},
		{"cleaned", "/sys/fs/cgroup", "kubepods/./pod/../pod/ctr", "/sys/fs/cgroup/kubepods/pod/ctr/ergon-isolated"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			got, err := partitionChild(tc.root, tc.path)
			if err != nil {
				t.Fatal(err)
			}
			if got != tc.want {
				t.Errorf("partitionChild(%q, %q) = %q, want %q", tc.root, tc.path, got, tc.want)
			}
		})
	}
	for _, tc := range []struct {
		name, root, path string
	}{
		{"empty path", "/sys/fs/cgroup", ""},
		{"outside root", "/sys/fs/cgroup", "/other/pod/ctr"},
		{"is the root", "/sys/fs/cgroup", "."},
	} {
		t.Run("refuse "+tc.name, func(t *testing.T) {
			if _, err := partitionChild(tc.root, tc.path); err == nil {
				t.Errorf("partitionChild(%q, %q) must refuse", tc.root, tc.path)
			}
		})
	}
}

func TestIs2MiBPageSizes(t *testing.T) {
	// Every spelling Kubernetes, CRI, OCI and NRI have been seen to use for
	// 2 MiB pages. A miss means the sized hugetlbfs is refused even though
	// the container has a hugepages-2Mi limit.
	yes := []string{"2MB", "2mb", "2M", "2m", "2Mi", "2mi", "2MiB", "2097152", " 2MB "}
	no := []string{"", "1Gi", "1GB", "2GB", "2097152B", "4MB", "hugepages-2Mi", "2"}
	for _, s := range yes {
		if !is2MiB(s) {
			t.Errorf("is2MiB(%q) = false, want true", s)
		}
	}
	for _, s := range no {
		if is2MiB(s) {
			t.Errorf("is2MiB(%q) = true, want false", s)
		}
	}
}

func TestRequestHugepageLimitBytesSpelling(t *testing.T) {
	// The limit must be found no matter which 2 MiB spelling the runtime uses,
	// and a 1 GiB limit must not be mistaken for one.
	for _, pageSize := range []string{"2MB", "2M", "2Mi", "2MiB", "2097152"} {
		c := withHugepages(ctr("ctr-1", "1,5", nil, nil), pageSize, 16*1024*1024)
		if got := requestHugepageLimitBytes(c); got != 16*1024*1024 {
			t.Errorf("requestHugepageLimitBytes(pageSize=%q) = %d, want %d", pageSize, got, 16*1024*1024)
		}
	}
	c := withHugepages(ctr("ctr-1", "1,5", nil, nil), "1GB", 1024*1024*1024)
	if got := requestHugepageLimitBytes(c); got != 0 {
		t.Errorf("a 1GB limit must not count as 2Mi: got %d", got)
	}
	c2 := withHugepages(withHugepages(ctr("ctr-1", "1,5", nil, nil), "1GB", 1), "2Mi", 222)
	if got := requestHugepageLimitBytes(c2); got != 222 {
		t.Errorf("mixed limits: got %d, want the 2Mi one (222)", got)
	}
}

func TestHugepageLimitFromCgroup(t *testing.T) {
	// Kubernetes does not put hugepage limits on the OCI/NRI container. It
	// writes hugetlb.2MB.max on the pod cgroup. The sized hugetlbfs mount
	// takes its size= from that file.
	root := t.TempDir()
	podPath := "kubelet-kubepods-podabc.slice"
	if err := os.MkdirAll(filepath.Join(root, podPath), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, podPath, "hugetlb.2MB.max"), []byte("16777216\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	// Container cgroup sits under the pod cgroup and has no useful limit.
	ctrPath := podPath + ":cri-containerd:cid"
	if err := os.MkdirAll(filepath.Join(root, ctrPath), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, ctrPath, "hugetlb.2MB.max"), []byte("max\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	got := hugepageLimitFromCgroup(root, podPath, ctrPath)
	if got != 16777216 {
		t.Errorf("pod cgroup limit = %d, want 16777216", got)
	}
	// The pod path alone is enough.
	if got := hugepageLimitFromCgroup(root, podPath, ""); got != 16777216 {
		t.Errorf("pod-only path = %d, want 16777216", got)
	}
	// A pod cgroup with "max" means no limit.
	pod2 := "kubelet-kubepods-podmax.slice"
	if err := os.MkdirAll(filepath.Join(root, pod2), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, pod2, "hugetlb.2MB.max"), []byte("max\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if got := hugepageLimitFromCgroup(root, pod2, ""); got != 0 {
		t.Errorf("max = %d, want 0", got)
	}
	// No file: no limit.
	if got := hugepageLimitFromCgroup(root, "nope", ""); got != 0 {
		t.Errorf("missing = %d, want 0", got)
	}
	// Derive the pod path from the container path when only that is known.
	if got := hugepageLimitFromCgroup(root, "", ctrPath); got != 16777216 {
		t.Errorf("derived pod path = %d, want 16777216", got)
	}
}

// --- adjustment keys must be unique ---
//
// NRI's FieldOwners treats a duplicate key in one adjustment as "plugins X and
// X both tried to set env KEY" and the create fails. Claim every key the way
// the runtime does.

func claimAdjustment(t *testing.T, adj *api.ContainerAdjustment, plugin string) {
	t.Helper()
	owners := api.NewOwningPlugins()
	const id = "ctr"
	for _, e := range adj.GetEnv() {
		if err := owners.ClaimEnv(id, e.GetKey(), plugin); err != nil {
			t.Errorf("env %s: %v (raw env %v)", e.GetKey(), err, adj.GetEnv())
		}
	}
	for _, m := range adj.GetMounts() {
		if err := owners.ClaimMount(id, m.GetDestination(), plugin); err != nil {
			t.Errorf("mount %s: %v (raw mounts %v)", m.GetDestination(), err, adj.GetMounts())
		}
	}
	for _, d := range adj.GetLinux().GetDevices() {
		if err := owners.ClaimDevice(id, d.GetPath(), plugin); err != nil {
			t.Errorf("device %s: %v", d.GetPath(), err)
		}
	}
	for _, r := range adj.GetRlimits() {
		if err := owners.ClaimRlimit(id, r.GetType(), plugin); err != nil {
			t.Errorf("rlimit %s: %v", r.GetType(), err)
		}
	}
}

func TestCreateAdjustmentUniqueKeys(t *testing.T) {
	// The three live shapes that tripped NRI's field-owner check: seccomp
	// plus memlock, rlimits alone, and ptp (the node-facts bind).
	for _, tc := range []struct {
		name string
		pod  map[string]string
		ctr  map[string]string
	}{
		{"seccomp-yes", map[string]string{
			names.SeccompPrefix + "app": "io-uring",
			names.RlimitsPrefix + "app": "memlock",
		}, nil},
		{"rlimits-yes", map[string]string{
			names.RlimitsPrefix + "app": "memlock,rt-prio,nofile",
		}, nil},
		{"ptp-yes", map[string]string{
			names.PtpPrefix + "app": "auto",
		}, nil},
		{"both-ann", map[string]string{
			names.RlimitsPrefix + "app": "memlock",
		}, map[string]string{
			names.RlimitsPrefix + "app": "memlock",
		}},
		{"all-grants", map[string]string{
			names.SeccompPrefix + "app":   "io-uring",
			names.RlimitsPrefix + "app":   "memlock,nofile",
			names.PtpPrefix + "app":       "auto",
			names.CpuqosPrefix + "app":    "1",
			names.HugepagesPrefix + "app": "2M",
		}, nil},
	} {
		t.Run(tc.name, func(t *testing.T) {
			p := testPlugin(t, "0,4")
			p.cfg = grantConfig(t)
			p.cfg.GOARCH = "amd64"
			p.cfg.HostCpuDmaLatency = "/dev/cpu_dma_latency"
			pd := pod()
			pd.Annotations = tc.pod
			c := ctr("ctr-1", "1,5", nil, tc.ctr)
			if tc.name == "all-grants" {
				c = withHugepages(c, "2MB", 16*1024*1024)
			}
			adj, _, err := p.CreateContainer(context.Background(), pd, c)
			if err != nil {
				t.Fatal(err)
			}
			if adj == nil {
				t.Fatal("no adjustment")
			}
			claimAdjustment(t, adj, "50-ergon-shared-cpus")
		})
	}
}

// --- systemd cgroups path (the kind/EKS cgroup driver) ---
//
// kind and EKS use cgroupDriver: systemd. NRI's cgroupsPath is a systemd unit
// name ("a-b-c.slice:cri-containerd:id"), not a filesystem path. Expand the
// slice into its nested directories and the ":cri-containerd:id" tail into a
// cri-containerd-<id>.scope child.

func TestCgroupFsPathSystemd(t *testing.T) {
	for _, tc := range []struct {
		name, root, path, want string
	}{
		{
			"systemd pod slice",
			"/sys/fs/cgroup",
			"kubelet-kubepods-burstable-pod3f2e096e4507d9cf7c1ae0e6b6bfe278.slice",
			"/sys/fs/cgroup/kubelet.slice/kubelet-kubepods.slice/kubelet-kubepods-burstable.slice/kubelet-kubepods-burstable-pod3f2e096e4507d9cf7c1ae0e6b6bfe278.slice",
		},
		{
			"systemd guaranteed pod slice",
			"/sys/fs/cgroup",
			"kubelet-kubepods-pod8616f28d851e484f9e3576f0b48f5bad.slice",
			"/sys/fs/cgroup/kubelet.slice/kubelet-kubepods.slice/kubelet-kubepods-pod8616f28d851e484f9e3576f0b48f5bad.slice",
		},
		{
			"systemd container scope",
			"/sys/fs/cgroup",
			"kubelet-kubepods-burstable-pod3f2e096e4507d9cf7c1ae0e6b6bfe278.slice:cri-containerd:8145bb76deadbeef",
			"/sys/fs/cgroup/kubelet.slice/kubelet-kubepods.slice/kubelet-kubepods-burstable.slice/kubelet-kubepods-burstable-pod3f2e096e4507d9cf7c1ae0e6b6bfe278.slice/cri-containerd-8145bb76deadbeef.scope",
		},
		{
			"slash form still works",
			"/sys/fs/cgroup",
			"kubepods/pod/ctr",
			"/sys/fs/cgroup/kubepods/pod/ctr",
		},
		{
			"absolute slash form",
			"/sys/fs/cgroup",
			"/sys/fs/cgroup/kubepods/pod/ctr",
			"/sys/fs/cgroup/kubepods/pod/ctr",
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			got, err := cgroupFsPath(tc.root, tc.path)
			if err != nil {
				t.Fatal(err)
			}
			if got != tc.want {
				t.Errorf("cgroupFsPath(%q, %q) = %q, want %q", tc.root, tc.path, got, tc.want)
			}
		})
	}
	for _, tc := range []struct {
		name, root, path string
	}{
		{"empty", "/sys/fs/cgroup", ""},
		{"outside root", "/sys/fs/cgroup", "/other/pod"},
		{"is the root", "/sys/fs/cgroup", "."},
	} {
		t.Run("refuse "+tc.name, func(t *testing.T) {
			if _, err := cgroupFsPath(tc.root, tc.path); err == nil {
				t.Errorf("cgroupFsPath(%q, %q) must refuse", tc.root, tc.path)
			}
		})
	}
}

func TestHugepageLimitFromCgroupSystemd(t *testing.T) {
	// The live kind node: systemd cgroup driver, pod cgroup carries
	// hugetlb.2MB.max, the container cgroup is "max".
	root := t.TempDir()
	podDir := filepath.Join(root,
		"kubelet.slice", "kubelet-kubepods.slice",
		"kubelet-kubepods-burstable.slice",
		"kubelet-kubepods-burstable-pod3f2e096e4507d9cf7c1ae0e6b6bfe278.slice")
	if err := os.MkdirAll(podDir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(podDir, "hugetlb.2MB.max"), []byte("16777216\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	ctrDir := filepath.Join(podDir, "cri-containerd-8145bb76deadbeef.scope")
	if err := os.MkdirAll(ctrDir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(ctrDir, "hugetlb.2MB.max"), []byte("max\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	podPath := "kubelet-kubepods-burstable-pod3f2e096e4507d9cf7c1ae0e6b6bfe278.slice"
	ctrPath := podPath + ":cri-containerd:8145bb76deadbeef"
	if got := hugepageLimitFromCgroup(root, podPath, ctrPath); got != 16777216 {
		t.Errorf("systemd pod cgroup limit = %d, want 16777216", got)
	}
	if got := hugepageLimitFromCgroup(root, "", ctrPath); got != 16777216 {
		t.Errorf("systemd derived pod path = %d, want 16777216", got)
	}
}

func TestPartitionChildSystemd(t *testing.T) {
	got, err := partitionChild("/sys/fs/cgroup",
		"kubelet-kubepods-burstable-pod3f2e096e4507d9cf7c1ae0e6b6bfe278.slice:cri-containerd:8145bb76deadbeef")
	if err != nil {
		t.Fatal(err)
	}
	want := "/sys/fs/cgroup/kubelet.slice/kubelet-kubepods.slice/kubelet-kubepods-burstable.slice/" +
		"kubelet-kubepods-burstable-pod3f2e096e4507d9cf7c1ae0e6b6bfe278.slice/" +
		"cri-containerd-8145bb76deadbeef.scope/" + names.CpuPartitionChild
	if got != want {
		t.Errorf("partitionChild = %q, want %q", got, want)
	}
}
