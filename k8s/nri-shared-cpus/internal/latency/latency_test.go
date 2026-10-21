// SPDX-License-Identifier: Apache-2.0

package latency

import (
	"errors"
	"strings"
	"testing"
)

// --- rlimit table ---

func TestRlimitTableDefaults(t *testing.T) {
	cfg := RlimitConfig{Memlock: "inf", Nofile: "65536", RtPrio: "50"}
	got, err := Table(cfg, "memlock,rt-prio,nofile")
	if err != nil {
		t.Fatal(err)
	}
	want := []Rlimit{
		{Type: "RLIMIT_MEMLOCK", Hard: Inf, Soft: Inf},
		{Type: "RLIMIT_RTPRIO", Hard: 50, Soft: 50},
		{Type: "RLIMIT_NOFILE", Hard: 65536, Soft: 65536},
	}
	if len(got) != len(want) {
		t.Fatalf("got %d rlimits %v, want %d", len(got), got, len(want))
	}
	for i, w := range want {
		if got[i] != w {
			t.Errorf("rlimit %d: got %+v, want %+v", i, got[i], w)
		}
	}
}

func TestRlimitTableSubset(t *testing.T) {
	cfg := RlimitConfig{Memlock: "inf", Nofile: "65536", RtPrio: "50"}
	got, err := Table(cfg, "memlock")
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 1 || got[0].Type != "RLIMIT_MEMLOCK" {
		t.Fatalf("got %v, want only RLIMIT_MEMLOCK", got)
	}
}

func TestRlimitTableUnknownKind(t *testing.T) {
	cfg := RlimitConfig{Memlock: "inf"}
	_, err := Table(cfg, "memlock,bogus")
	if err == nil {
		t.Fatal("want an error for an unknown rlimit kind")
	}
	if !strings.Contains(err.Error(), "bogus") {
		t.Errorf("error %q must name the bad kind", err)
	}
}

func TestRlimitTableMissingConfig(t *testing.T) {
	// rt-prio asked for but the plugin config has no rtPrio value.
	_, err := Table(RlimitConfig{Memlock: "inf"}, "rt-prio")
	if err == nil {
		t.Fatal("want an error when the kind has no configured value")
	}
	if !errors.As(err, new(*GrantError)) {
		t.Errorf("want a *GrantError, got %T", err)
	}
}

func TestRlimitTableEmpty(t *testing.T) {
	got, err := Table(RlimitConfig{Memlock: "inf"}, "")
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 0 {
		t.Fatalf("got %v, want none", got)
	}
}

func TestRlimitParseValues(t *testing.T) {
	cases := []struct {
		in   string
		want uint64
	}{
		{"inf", Inf},
		{"unlimited", Inf},
		{"infinity", Inf},
		{"65536", 65536},
		{"1048576", 1048576},
	}
	for _, c := range cases {
		got, err := ParseRlimitValue(c.in)
		if err != nil {
			t.Errorf("ParseRlimitValue(%q): %v", c.in, err)
			continue
		}
		if got != c.want {
			t.Errorf("ParseRlimitValue(%q) = %d, want %d", c.in, got, c.want)
		}
	}
	for _, bad := range []string{"", "0", "-1", "1.5", "12x"} {
		if _, err := ParseRlimitValue(bad); err == nil {
			t.Errorf("ParseRlimitValue(%q) must fail", bad)
		}
	}
}

// --- NUMA node pick ---

func TestPickMemsFromRequest(t *testing.T) {
	mems, nodes, err := PickMems("1", []int{0, 1}, 1)
	if err != nil {
		t.Fatal(err)
	}
	if mems != "1" || nodes != "1" {
		t.Errorf("got mems=%q nodes=%q, want both 1", mems, nodes)
	}
}

func TestPickMemsDefaultToNic(t *testing.T) {
	mems, nodes, err := PickMems("", []int{0, 1}, 1)
	if err != nil {
		t.Fatal(err)
	}
	if mems != "1" || nodes != "1" {
		t.Errorf("got mems=%q nodes=%q, want the NIC node 1", mems, nodes)
	}
}

func TestPickMemsSingleNumaNoOp(t *testing.T) {
	// c8g.xlarge is expected single-NUMA: mems=0 is a no-op but still valid.
	mems, nodes, err := PickMems("0", []int{0}, 0)
	if err != nil {
		t.Fatal(err)
	}
	if mems != "0" || nodes != "0" {
		t.Errorf("got mems=%q nodes=%q, want 0", mems, nodes)
	}
}

func TestPickMemsUnknownNode(t *testing.T) {
	_, _, err := PickMems("3", []int{0, 1}, 0)
	if err == nil {
		t.Fatal("want an error for a NUMA node the machine does not have")
	}
	if !strings.Contains(err.Error(), "3") {
		t.Errorf("error %q must name the bad node", err)
	}
}

func TestPickMemsNoNodes(t *testing.T) {
	// An empty list means the topology was never read. Nothing may be bound
	// from it, and a request must not be dropped in silence.
	for _, ask := range []string{"", "0", "0,1"} {
		_, _, err := PickMems(ask, nil, -1)
		if err == nil {
			t.Fatalf("PickMems(%q) with no topology: want a refusal", ask)
		}
		if !strings.Contains(err.Error(), "NUMA topology") {
			t.Errorf("error %q must name the missing topology", err)
		}
	}
}

func TestPickMemsEmptyRequestOnASingleNode(t *testing.T) {
	// "\"\" is the NIC's node, and a one-node machine has only one answer.
	mems, nodes, err := PickMems("", []int{0}, -1)
	if err != nil {
		t.Fatal(err)
	}
	if mems != "0" || nodes != "0" {
		t.Errorf("got mems=%q nodes=%q, want 0", mems, nodes)
	}
}

func TestPickMemsEmptyRequestWithoutANicNode(t *testing.T) {
	// Several nodes and no data NIC recorded: node 0 would be a guess, so the
	// request is refused and names the fix.
	_, _, err := PickMems("", []int{0, 1}, -1)
	if err == nil {
		t.Fatal("want a refusal when the NIC's node is unknown on a multi-node machine")
	}
	if !strings.Contains(err.Error(), "numa.ergon.io") {
		t.Errorf("error %q must name the annotation to set", err)
	}
}

// --- seccomp clone + io_uring allow ---

func TestSeccompIOUringAllow(t *testing.T) {
	in := &SeccompPolicy{
		DefaultAction: "SCMP_ACT_ERRNO",
		Architectures: []string{"SCMP_ARCH_X86_64"},
		Syscalls: []SeccompSyscall{
			{Names: []string{"read", "write"}, Action: "SCMP_ACT_ALLOW"},
		},
	}
	out, err := WithIOUring(in, ModeIOUring)
	if err != nil {
		t.Fatal(err)
	}
	if out == in {
		t.Error("WithIOUring must clone, not mutate the request's policy")
	}
	if out.DefaultAction != in.DefaultAction {
		t.Errorf("default action changed: %q", out.DefaultAction)
	}
	found := false
	for _, s := range out.Syscalls {
		for _, n := range s.Names {
			if n == "io_uring_setup" || n == "io_uring_enter" || n == "io_uring_register" {
				found = true
				if s.Action != "SCMP_ACT_ALLOW" {
					t.Errorf("io_uring rule action %q, want SCMP_ACT_ALLOW", s.Action)
				}
			}
		}
	}
	if !found {
		t.Error("io_uring syscalls must be allowed")
	}
	// the input is untouched
	if len(in.Syscalls) != 1 {
		t.Errorf("input policy was mutated: %v", in.Syscalls)
	}
}

func TestSeccompNilInput(t *testing.T) {
	out, err := WithIOUring(nil, ModeIOUring)
	if err != nil {
		t.Fatal(err)
	}
	if out == nil {
		t.Fatal("a nil request policy must still produce a policy with io_uring allowed")
	}
	if out.DefaultAction == "" {
		t.Error("default action must be set")
	}
}

func TestSeccompRuntimeDefaultUnchanged(t *testing.T) {
	in := &SeccompPolicy{
		DefaultAction: "SCMP_ACT_ERRNO",
		Syscalls:      []SeccompSyscall{{Names: []string{"read"}, Action: "SCMP_ACT_ALLOW"}},
	}
	out, err := WithIOUring(in, ModeRuntimeDefault)
	if err != nil {
		t.Fatal(err)
	}
	if out.DefaultAction != in.DefaultAction {
		t.Error("runtime-default must not change the policy")
	}
	for _, s := range out.Syscalls {
		for _, n := range s.Names {
			if strings.HasPrefix(n, "io_uring") {
				t.Errorf("runtime-default must not add %q", n)
			}
		}
	}
}

func TestSeccompUnconfinedRefused(t *testing.T) {
	_, err := WithIOUring(&SeccompPolicy{}, ModeUnconfined)
	if err == nil {
		t.Fatal("unconfined must be refused by default")
	}
	if !errors.As(err, new(*GrantError)) {
		t.Errorf("want a *GrantError, got %T", err)
	}
}

// --- sysctl map (candidate 7) ---

func TestSysctlVerdicts(t *testing.T) {
	cases := []struct {
		name string
		want SysctlVerdict
	}{
		{"net.core.rmem_max", SysctlNodeOnly},
		{"net.core.wmem_max", SysctlNodeOnly},
		{"net.core.rmem_default", SysctlNodeOnly},
		{"net.core.wmem_default", SysctlNodeOnly},
		{"net.core.busy_poll", SysctlNodeOnly},
		{"net.core.busy_read", SysctlNodeOnly},
		{"net.ipv4.tcp_rmem", SysctlPodOK},
		{"net.ipv4.tcp_wmem", SysctlPodOK},
		{"kernel.shmmax", SysctlPodOK},
		{"net.core.gro_flush_timeout", SysctlNetdevAttr},
		{"net.core.napi_defer_hard_irqs", SysctlNetdevAttr},
	}
	for _, c := range cases {
		if got := SysctlVerdictFor(c.name); got != c.want {
			t.Errorf("SysctlVerdictFor(%q) = %v, want %v", c.name, got, c.want)
		}
	}
}

func TestSysctlRefusalNamesTheFix(t *testing.T) {
	msg := RefuseSysctl("net.core.rmem_max")
	if !strings.Contains(msg, "net.core.rmem_max") {
		t.Errorf("message %q must name the sysctl", msg)
	}
	if !strings.Contains(msg, "node") {
		t.Errorf("message %q must say it is a node setting", msg)
	}
}

// --- env building ---

func TestGrantEnvRlimits(t *testing.T) {
	rl := []Rlimit{
		{Type: "RLIMIT_MEMLOCK", Hard: Inf, Soft: Inf},
		{Type: "RLIMIT_NOFILE", Hard: 65536, Soft: 65536},
		{Type: "RLIMIT_RTPRIO", Hard: 50, Soft: 50},
	}
	env := GrantEnv(EnvGrant{Rlimits: rl, SchedFifo: 50, SeccompMode: ModeIOUring, NumaNodes: "0", PtpDevice: "/dev/ptp0"})
	m := kvMap(env)
	want := map[string]string{
		"ERGON_RLIMIT_MEMLOCK": "inf",
		"ERGON_RLIMIT_NOFILE":  "65536",
		"ERGON_RLIMIT_RTPRIO":  "50",
		"ERGON_SCHED_FIFO":     "50",
		"ERGON_SECCOMP":        "io-uring",
		"ERGON_NUMA_NODES":     "0",
		"ERGON_PTP_DEVICE":     "/dev/ptp0",
	}
	for k, v := range want {
		if m[k] != v {
			t.Errorf("env %s = %q, want %q", k, m[k], v)
		}
	}
}

func TestGrantEnvMinimal(t *testing.T) {
	// ERGON_SECCOMP always names the mode so the app names the fix instead
	// of "EPERM". Every other key is absent without a grant.
	env := GrantEnv(EnvGrant{SeccompMode: ModeRuntimeDefault})
	m := kvMap(env)
	if m["ERGON_SECCOMP"] != "runtime-default" {
		t.Errorf("ERGON_SECCOMP = %q, want runtime-default", m["ERGON_SECCOMP"])
	}
	for _, k := range []string{"ERGON_RLIMIT_MEMLOCK", "ERGON_RLIMIT_NOFILE", "ERGON_RLIMIT_RTPRIO",
		"ERGON_SCHED_FIFO", "ERGON_NUMA_NODES", "ERGON_PTP_DEVICE", "ERGON_HUGEPAGES"} {
		if _, ok := m[k]; ok {
			t.Errorf("env %s must be absent without a grant", k)
		}
	}
}

// --- ptp device pick ---

func TestPickPtpAuto(t *testing.T) {
	got, err := PickPtp([]string{"/dev/ptp0", "/dev/ptp1"}, "auto")
	if err != nil {
		t.Fatal(err)
	}
	if got != "/dev/ptp0" {
		t.Errorf("got %q, want the first PHC /dev/ptp0", got)
	}
}

func TestPickPtpExplicit(t *testing.T) {
	got, err := PickPtp([]string{"/dev/ptp0", "/dev/ptp1"}, "/dev/ptp1")
	if err != nil {
		t.Fatal(err)
	}
	if got != "/dev/ptp1" {
		t.Errorf("got %q, want /dev/ptp1", got)
	}
}

func TestPickPtpMissing(t *testing.T) {
	if _, err := PickPtp([]string{}, "auto"); err == nil {
		t.Fatal("want an error when the node has no PHC")
	}
	if _, err := PickPtp([]string{"/dev/ptp0"}, "/dev/ptp9"); err == nil {
		t.Fatal("want an error when the asked-for PHC is absent")
	}
}

func TestPickPtpOff(t *testing.T) {
	got, err := PickPtp([]string{"/dev/ptp0"}, "off")
	if err != nil {
		t.Fatal(err)
	}
	if got != "" {
		t.Errorf("off must grant no device, got %q", got)
	}
}

// --- helpers ---

func kvMap(kvs []KV) map[string]string {
	m := make(map[string]string, len(kvs))
	for _, kv := range kvs {
		m[kv.Key] = kv.Value
	}
	return m
}

// --- sized hugetlbfs mount (item 1) ---

func TestHugepageMountSizeEqualsLimit(t *testing.T) {
	m, envs, err := HugepageMount(256 * 1024 * 1024)
	if err != nil {
		t.Fatal(err)
	}
	if m.Type != "hugetlbfs" {
		t.Errorf("type = %q, want hugetlbfs", m.Type)
	}
	if m.Destination != "/dev/hugepages" {
		t.Errorf("destination = %q, want /dev/hugepages", m.Destination)
	}
	if m.Source != "none" {
		t.Errorf("source = %q, want none", m.Source)
	}
	if m.LimitBytes != 256*1024*1024 {
		t.Errorf("limit = %d, want 256 MiB", m.LimitBytes)
	}
	if m.PageSize != "2MB" {
		t.Errorf("page size = %q, want 2MB (the NRI/OCI spelling)", m.PageSize)
	}
	opts := strings.Join(m.Options, ",")
	if !strings.Contains(opts, "pagesize=2M") {
		t.Errorf("options %v must name pagesize=2M", m.Options)
	}
	if !strings.Contains(opts, "size=268435456") {
		t.Errorf("options %v must name size=<bytes> equal to the limit", m.Options)
	}
	m2 := kvMap(envs)
	if m2[EnvHugepagesDir] != "/dev/hugepages" {
		t.Errorf("%s = %q, want /dev/hugepages", EnvHugepagesDir, m2[EnvHugepagesDir])
	}
	if m2[EnvHugepageSize] != "2097152" {
		t.Errorf("%s = %q, want 2097152", EnvHugepageSize, m2[EnvHugepageSize])
	}
}

func TestHugepageMountNoLimitRefused(t *testing.T) {
	// The annotation asks for a sized mount but the container has no
	// hugepages-2Mi limit: fail closed and name the fix.
	_, _, err := HugepageMount(0)
	if err == nil {
		t.Fatal("must refuse when the container has no hugepages-2Mi limit")
	}
	var ge *GrantError
	if !errors.As(err, &ge) {
		t.Fatalf("want a *GrantError, got %T", err)
	}
	if !strings.Contains(err.Error(), "hugepages-2Mi") {
		t.Errorf("refusal %q must name hugepages-2Mi", err)
	}
}

func TestHugepageMountNegativeRefused(t *testing.T) {
	if _, _, err := HugepageMount(-1); err == nil {
		t.Fatal("must refuse a negative limit")
	}
}

func TestHugepageAnnotationValue(t *testing.T) {
	if err := CheckHugepageAnnotation("2M"); err != nil {
		t.Errorf("2M must be accepted: %v", err)
	}
	for _, bad := range []string{"", "1G", "2MB", "4K", "2m"} {
		if err := CheckHugepageAnnotation(bad); err == nil {
			t.Errorf("%q must be refused (only \"2M\" is supported)", bad)
		} else if !strings.Contains(err.Error(), "2M") {
			t.Errorf("refusal %q must name the accepted value", err)
		}
	}
}

// --- cpu-partition.ergon.io value (item 2) ---

func TestCpuPartitionValue(t *testing.T) {
	if err := CheckCpuPartitionValue("isolated"); err != nil {
		t.Errorf("isolated must be accepted: %v", err)
	}
	for _, bad := range []string{"", "root", "member", "isolated ", "Isolated", "isolated,yes"} {
		if err := CheckCpuPartitionValue(bad); err == nil {
			t.Errorf("%q must be refused (only \"isolated\" is accepted)", bad)
		} else if !strings.Contains(err.Error(), "isolated") {
			t.Errorf("refusal %q must name the accepted value", err)
		}
	}
}

// --- cpuqos / /dev/cpu_dma_latency (item 5, x86 only) ---

func TestPickCpuQosAMD64Grants(t *testing.T) {
	path, err := PickCpuQos("1", "amd64", "/dev/cpu_dma_latency")
	if err != nil {
		t.Fatal(err)
	}
	if path != "/dev/cpu_dma_latency" {
		t.Errorf("path = %q, want /dev/cpu_dma_latency", path)
	}
}

func TestPickCpuQosAMD64NoDeviceFailsClosed(t *testing.T) {
	_, err := PickCpuQos("1", "amd64", "")
	if err == nil {
		t.Fatal("on amd64 a missing /dev/cpu_dma_latency must fail closed")
	}
	if !strings.Contains(err.Error(), "cpu_dma_latency") {
		t.Errorf("refusal %q must name the device", err)
	}
}

func TestPickCpuQosArm64Ignored(t *testing.T) {
	// Graviton: the annotation is ignored, no device, no error.
	path, err := PickCpuQos("1", "arm64", "")
	if err != nil {
		t.Fatalf("arm64 must ignore the annotation, got %v", err)
	}
	if path != "" {
		t.Errorf("arm64 must grant no device, got %q", path)
	}
}

func TestPickCpuQosBadValue(t *testing.T) {
	for _, bad := range []string{"", "0", "-1", "1.5", "fast"} {
		if _, err := PickCpuQos(bad, "amd64", "/dev/cpu_dma_latency"); err == nil {
			t.Errorf("cpuqos.ergon.io value %q must be refused (positive microseconds)", bad)
		}
	}
}

// --- env contract for the new grants ---

func TestGrantEnvHugepages(t *testing.T) {
	_, envs, err := HugepageMount(2 * 1024 * 1024 * 1024)
	if err != nil {
		t.Fatal(err)
	}
	got := kvMap(GrantEnv(EnvGrant{HugepageEnvs: envs}))
	if got[EnvHugepagesDir] != "/dev/hugepages" {
		t.Errorf("%s missing from GrantEnv", EnvHugepagesDir)
	}
	if got[EnvHugepageSize] != "2097152" {
		t.Errorf("%s missing from GrantEnv", EnvHugepageSize)
	}
	if _, ok := got["ERGON_HUGEPAGES"]; ok {
		t.Error("ERGON_HUGEPAGES is replaced by ERGON_HUGEPAGES_DIR/_SIZE and must not be set")
	}
}

func TestGrantEnvNoHugepages(t *testing.T) {
	got := kvMap(GrantEnv(EnvGrant{}))
	for _, k := range []string{EnvHugepagesDir, EnvHugepageSize, "ERGON_HUGEPAGES"} {
		if _, ok := got[k]; ok {
			t.Errorf("%s must be absent when no hugepage mount was granted", k)
		}
	}
}

func TestGrantEnvPartition(t *testing.T) {
	got := kvMap(GrantEnv(EnvGrant{Partition: &PartitionGrant{Child: "/sys/fs/cgroup/x/isolated"}}))
	if got[EnvIsolatedCgroup] != "/sys/fs/cgroup/x/isolated" {
		t.Errorf("%s = %q, want the child path", EnvIsolatedCgroup, got[EnvIsolatedCgroup])
	}
}

func TestGrantEnvCpuqos(t *testing.T) {
	got := kvMap(GrantEnv(EnvGrant{CpuqosDevice: "/dev/cpu_dma_latency"}))
	if got[EnvCpuDmaLatency] != "/dev/cpu_dma_latency" {
		t.Errorf("%s = %q", EnvCpuDmaLatency, got[EnvCpuDmaLatency])
	}
}

func TestGrantEnvPtpAndNodeFacts(t *testing.T) {
	got := kvMap(GrantEnv(EnvGrant{PtpDevice: "/dev/ptp0", NodeFacts: "/run/ergon/node.json"}))
	if got[EnvPtpDevice] != "/dev/ptp0" {
		t.Errorf("%s = %q", EnvPtpDevice, got[EnvPtpDevice])
	}
	if got[EnvNodeFacts] != "/run/ergon/node.json" {
		t.Errorf("%s = %q", EnvNodeFacts, got[EnvNodeFacts])
	}
}
