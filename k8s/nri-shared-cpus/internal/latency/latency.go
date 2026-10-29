// SPDX-License-Identifier: Apache-2.0

// Package latency holds the pure functions behind the latency-pod grants:
// the rlimit table, the NUMA node pick, the seccomp widening, the netns
// sysctl verdicts, the env contract and the PHC device pick. Every function
// here is free of I/O so it can be unit tested, and the NRI plugin only
// applies what this package returns.
package latency

import (
	"fmt"
	"net/netip"
	"sort"
	"strconv"
	"strings"
)

// Inf is "unlimited" as a POSIX rlimit value.
const Inf = ^uint64(0)

// GrantError is a typed refusal: the pod asked for a grant the node or the
// plugin config cannot give. The kubelet shows the text in the pod's events.
type GrantError struct {
	// Kind is the metric label (rlimit, seccomp, numa, ptp, sysctl).
	Kind string
	// Msg names the fix.
	Msg string
}

func (e *GrantError) Error() string { return "ergon-shared-cpus: " + e.Msg }

func grantErr(kind, format string, args ...any) *GrantError {
	return &GrantError{Kind: kind, Msg: fmt.Sprintf(format, args...)}
}

// KV is one environment variable (same shape as nriplugin.KV).
type KV struct{ Key, Value string }

// ---------------------------------------------------------------------------
// rlimits
// ---------------------------------------------------------------------------

// RlimitConfig is the node-side rlimit ceiling. Values come from the plugin
// config only: a pod author never names a number.
type RlimitConfig struct {
	// Memlock is "inf" or a byte count (RLIMIT_MEMLOCK).
	Memlock string
	// Nofile is "inf" or a file-descriptor count (RLIMIT_NOFILE).
	Nofile string
	// RtPrio is "inf" or a priority (RLIMIT_RTPRIO).
	RtPrio string
	// Nice is "inf" or a nice value (RLIMIT_NICE). Optional.
	Nice string
}

// Rlimit is one POSIX rlimit to apply to the container.
type Rlimit struct {
	Type string
	Hard uint64
	Soft uint64
}

// rlimit kinds the annotation may name, and their config field / type.
var rlimitKinds = map[string]struct {
	cfg func(*RlimitConfig) string
	typ string
}{
	"memlock": {func(c *RlimitConfig) string { return c.Memlock }, "RLIMIT_MEMLOCK"},
	"nofile":  {func(c *RlimitConfig) string { return c.Nofile }, "RLIMIT_NOFILE"},
	"rt-prio": {func(c *RlimitConfig) string { return c.RtPrio }, "RLIMIT_RTPRIO"},
	"nice":    {func(c *RlimitConfig) string { return c.Nice }, "RLIMIT_NICE"},
}

// Table builds the rlimit grants for the kinds named in the annotation
// `rlimits.ergon.io/<container>`, e.g. "memlock,rt-prio,nofile". The values
// come from cfg. An unknown kind, or a kind whose config value is empty, is a
// typed refusal (fail closed).
func Table(cfg RlimitConfig, kinds string) ([]Rlimit, error) {
	kinds = strings.TrimSpace(kinds)
	if kinds == "" {
		return nil, nil
	}
	seen := map[string]bool{}
	var out []Rlimit
	for _, k := range strings.Split(kinds, ",") {
		k = strings.TrimSpace(k)
		if k == "" {
			continue
		}
		kind, ok := rlimitKinds[k]
		if !ok {
			return nil, grantErr("rlimit", "rlimits.ergon.io: unknown kind %q; use memlock, nofile, rt-prio, nice", k)
		}
		if seen[k] {
			return nil, grantErr("rlimit", "rlimits.ergon.io: kind %q is listed twice", k)
		}
		seen[k] = true
		raw := strings.TrimSpace(kind.cfg(&cfg))
		if raw == "" {
			return nil, grantErr("rlimit", "rlimits.ergon.io: %s is asked for but this node's config has no value for it", k)
		}
		v, err := ParseRlimitValue(raw)
		if err != nil {
			return nil, grantErr("rlimit", "rlimits.ergon.io: %s: %v", k, err)
		}
		out = append(out, Rlimit{Type: kind.typ, Hard: v, Soft: v})
	}
	return out, nil
}

// ParseRlimitValue parses "inf"/"unlimited"/"infinity" as Inf, or a positive
// decimal count. Zero and negatives are refused: a zero MEMLOCK would make
// mlockall fail with ENOMEM.
func ParseRlimitValue(s string) (uint64, error) {
	switch strings.ToLower(s) {
	case "inf", "unlimited", "infinity":
		return Inf, nil
	}
	n, err := strconv.ParseUint(s, 10, 64)
	if err != nil || n == 0 {
		return 0, fmt.Errorf("%q is not a positive byte count or \"inf\"", s)
	}
	return n, nil
}

// FormatRlimit renders a value for ERGON_RLIMIT_*: "inf" or the decimal count.
func FormatRlimit(v uint64) string {
	if v == Inf {
		return "inf"
	}
	return strconv.FormatUint(v, 10)
}

// ---------------------------------------------------------------------------
// NUMA
// ---------------------------------------------------------------------------

// PickMems chooses cpuset.mems and the MPOL_BIND nodes for a latency pod.
// requested is the `numa.ergon.io/<container>` annotation ("0", "0,1", or
// empty for "the NIC's node"). nodeNumaNodes is every NUMA node the machine
// has, as DiscoverHostNuma read it; nicNode is the node the data NIC is on
// (-1 when none is recorded).
//
// Every answer is a binding the machine can actually take. There is no silent
// no-op: a request that cannot be answered is refused and names what is
// missing. An empty nodeNumaNodes means the topology was never read, so
// nothing is chosen from it.
func PickMems(requested string, nodeNumaNodes []int, nicNode int) (mems string, nodes string, err error) {
	if len(nodeNumaNodes) == 0 {
		return "", "", grantErr("numa",
			"numa.ergon.io: this node's NUMA topology is unknown (nothing read /sys/devices/system/node), so no node can be chosen; the plugin reads it at start-up")
	}
	have := map[int]bool{}
	for _, n := range nodeNumaNodes {
		have[n] = true
	}
	requested = strings.TrimSpace(requested)
	if requested == "" {
		// "": the data NIC's node.
		if have[nicNode] {
			return strconv.Itoa(nicNode), strconv.Itoa(nicNode), nil
		}
		if len(nodeNumaNodes) == 1 {
			// One node: the NIC is on it, and it is the whole topology.
			n := nodeNumaNodes[0]
			return strconv.Itoa(n), strconv.Itoa(n), nil
		}
		sorted := make([]int, 0, len(nodeNumaNodes))
		sorted = append(sorted, nodeNumaNodes...)
		sort.Ints(sorted)
		return "", "", grantErr("numa",
			"numa.ergon.io: \"\" means the data NIC's NUMA node, and this node has none recorded (has %v); set numa.ergon.io/<container>: \"<node>\" to name it", sorted)
	}
	var pick []int
	for _, p := range strings.Split(requested, ",") {
		p = strings.TrimSpace(p)
		if p == "" {
			continue
		}
		n, convErr := strconv.Atoi(p)
		if convErr != nil {
			return "", "", grantErr("numa", "numa.ergon.io: %q is not a NUMA node number", requested)
		}
		if !have[n] {
			sorted := make([]int, 0, len(nodeNumaNodes))
			sorted = append(sorted, nodeNumaNodes...)
			sort.Ints(sorted)
			return "", "", grantErr("numa", "numa.ergon.io: node %d is not on this machine (has %v)", n, sorted)
		}
		pick = append(pick, n)
	}
	if len(pick) == 0 {
		return "", "", grantErr("numa", "numa.ergon.io: %q names no NUMA node", requested)
	}
	list := formatIntList(pick)
	return list, list, nil
}

func formatIntList(ns []int) string {
	parts := make([]string, len(ns))
	for i, n := range ns {
		parts[i] = strconv.Itoa(n)
	}
	return strings.Join(parts, ",")
}

// ---------------------------------------------------------------------------
// seccomp
// ---------------------------------------------------------------------------

// Seccomp modes an annotation may name.
const (
	ModeIOUring        = "io-uring"
	ModeRuntimeDefault = "runtime-default"
	ModeUnconfined     = "unconfined"
)

// SeccompArg is one argument filter of a rule (NRI api.LinuxSeccompArg, kept
// free of the NRI import so this package stays pure).
type SeccompArg struct {
	Index    uint32
	Value    uint64
	ValueTwo uint64
	Op       string
}

// SeccompSyscall is one rule of a seccomp policy (NRI api.LinuxSyscall, kept
// free of the NRI import so this package stays pure). Every field of the NRI
// type is here: a round trip through this package must not drop a filter.
type SeccompSyscall struct {
	Names  []string
	Action string
	// ErrnoRet is the errno SCMP_ACT_ERRNO returns (nil: the policy default).
	ErrnoRet *uint32
	Args     []SeccompArg
}

// SeccompPolicy is the container's seccomp policy (NRI api.LinuxSeccomp).
// Every field of the NRI type is here: a round trip through this package must
// not drop a restriction.
type SeccompPolicy struct {
	DefaultAction string
	// DefaultErrno is the errno SCMP_ACT_ERRNO returns (nil: the default).
	DefaultErrno     *uint32
	Architectures    []string
	Flags            []string
	ListenerPath     string
	ListenerMetadata string
	Syscalls         []SeccompSyscall
}

// ioUringSyscalls is the small deliberate widening. Aeron's RTT path does not
// use io_uring; the lab records "no change expected" for latency and proves
// enablement.
var ioUringSyscalls = []string{"io_uring_setup", "io_uring_enter", "io_uring_register", "io_uring_register2"}

// cloneSeccomp copies a policy whole, every rule and every argument filter
// included, so appending a rule never shares storage with the request.
func cloneSeccomp(req *SeccompPolicy) *SeccompPolicy {
	if req == nil {
		return nil
	}
	out := *req
	out.Architectures = append([]string(nil), req.Architectures...)
	out.Flags = append([]string(nil), req.Flags...)
	out.Syscalls = make([]SeccompSyscall, 0, len(req.Syscalls))
	for _, sc := range req.Syscalls {
		c := sc
		c.Names = append([]string(nil), sc.Names...)
		if sc.ErrnoRet != nil {
			v := *sc.ErrnoRet
			c.ErrnoRet = &v
		}
		c.Args = append([]SeccompArg(nil), sc.Args...)
		out.Syscalls = append(out.Syscalls, c)
	}
	return &out
}

// WithIOUring returns the policy to apply. ModeIOUring clones the request's
// policy whole and appends one allow rule for the io_uring syscalls; never
// Unconfined. ModeRuntimeDefault returns the request's policy unchanged.
// ModeUnconfined is refused by default (PSS Baseline forbids it).
func WithIOUring(req *SeccompPolicy, mode string) (*SeccompPolicy, error) {
	switch mode {
	case ModeRuntimeDefault, "":
		if req == nil {
			return &SeccompPolicy{DefaultAction: "SCMP_ACT_ERRNO"}, nil
		}
		return req, nil
	case ModeUnconfined:
		return nil, grantErr("seccomp", "seccomp.ergon.io: %q is refused; use %q (RuntimeDefault plus io_uring)", ModeUnconfined, ModeIOUring)
	case ModeIOUring:
		base := cloneSeccomp(req)
		if base == nil {
			base = &SeccompPolicy{DefaultAction: "SCMP_ACT_ERRNO"}
		}
		base.Syscalls = append(base.Syscalls, SeccompSyscall{
			Names:  append([]string(nil), ioUringSyscalls...),
			Action: "SCMP_ACT_ALLOW",
		})
		return base, nil
	default:
		return nil, grantErr("seccomp", "seccomp.ergon.io: %q is not a known mode; use %q or %q", mode, ModeIOUring, ModeRuntimeDefault)
	}
}

// ---------------------------------------------------------------------------
// netns sysctls (candidate 7)
// ---------------------------------------------------------------------------

// SysctlVerdict is where a net.* setting can be written.
type SysctlVerdict int

const (
	// SysctlPodOK: writable inside a pod netns.
	SysctlPodOK SysctlVerdict = iota
	// SysctlNodeOnly: per-netns but read-only outside init_net, or registered
	// only for init_net. Write it on the node (sysctl.d), never in a pod.
	SysctlNodeOnly
	// SysctlNetdevAttr: not a sysctl at all; a per-netdev sysfs attribute
	// (gro_flush_timeout, napi_defer_hard_irqs). A node unit writes it.
	SysctlNetdevAttr
)

// nodeOnlySysctls are read-only in a non-init netns (netns_core_table_dup
// clears the write bits) or registered only for init_net (net_core_table).
var nodeOnlySysctls = map[string]SysctlVerdict{
	"net.core.rmem_max":             SysctlNodeOnly,
	"net.core.wmem_max":             SysctlNodeOnly,
	"net.core.rmem_default":         SysctlNodeOnly,
	"net.core.wmem_default":         SysctlNodeOnly,
	"net.core.busy_poll":            SysctlNodeOnly,
	"net.core.busy_read":            SysctlNodeOnly,
	"net.core.gro_flush_timeout":    SysctlNetdevAttr,
	"net.core.napi_defer_hard_irqs": SysctlNetdevAttr,
}

// SysctlVerdictFor reports where a named net setting belongs.
func SysctlVerdictFor(name string) SysctlVerdict {
	if v, ok := nodeOnlySysctls[name]; ok {
		return v
	}
	return SysctlPodOK
}

// RefuseSysctl is the message for a pod that asked for a node-only setting.
// It names the node sysctl so the fix is obvious.
func RefuseSysctl(name string) string {
	v := SysctlVerdictFor(name)
	switch v {
	case SysctlNetdevAttr:
		return fmt.Sprintf("%s is a per-netdev sysfs attribute, not a sysctl; write it on the node (see nri-latency-pods/node-checks.md)", name)
	default:
		return fmt.Sprintf("%s is a node-wide sysctl: write it in the node's sysctl.d before the kubelet starts (see nri-latency-pods/node-checks.md)", name)
	}
}

// ---------------------------------------------------------------------------
// PHC device
// ---------------------------------------------------------------------------

// PickPtp chooses the NIC PHC to hand a latency pod. hostDevices is what the
// node has (e.g. /dev/ptp0). requested is the `ptp.ergon.io/<container>`
// annotation: "off", "auto" or a path. Returns "" when no device is granted.
func PickPtp(hostDevices []string, requested string) (string, error) {
	requested = strings.TrimSpace(requested)
	switch requested {
	case "off", "":
		return "", nil
	case "auto":
		if len(hostDevices) == 0 {
			return "", grantErr("ptp", "ptp.ergon.io: this node has no PHC (/dev/ptp*); use ptp.ergon.io/%s: \"off\" or drop the request", "container")
		}
		return hostDevices[0], nil
	default:
		for _, d := range hostDevices {
			if d == requested {
				return d, nil
			}
		}
		return "", grantErr("ptp", "ptp.ergon.io: %q is not on this node (has %v)", requested, hostDevices)
	}
}

// ---------------------------------------------------------------------------
// PM QoS (/dev/cpu_dma_latency, x86 only)
// ---------------------------------------------------------------------------

// PickCpuQos validates a `cpuqos.ergon.io/<container>` request and returns the
// device path to grant. The value is the target latency in microseconds. arch
// is the node's GOARCH; on anything but amd64 the annotation is ignored and no
// device is granted (Graviton has no deep C-state to cap). On amd64 the node
// must have the device: without it the grant fails closed and names the fix.
func PickCpuQos(requested, arch, device string) (string, error) {
	if arch != "amd64" {
		// Graviton and friends: the annotation is ignored, never a refusal.
		return "", nil
	}
	requested = strings.TrimSpace(requested)
	n, err := strconv.Atoi(requested)
	if err != nil || n <= 0 {
		return "", grantErr("cpuqos", "cpuqos.ergon.io: %q is not a positive latency in microseconds", requested)
	}
	if device == "" {
		return "", grantErr("cpuqos", "cpuqos.ergon.io: this node has no /dev/cpu_dma_latency; drop the annotation or run on a node that exposes it")
	}
	return device, nil
}

// ---------------------------------------------------------------------------
// sized hugetlbfs mount for AERON_DIR
// ---------------------------------------------------------------------------

// HugepagePageSize is the OCI/NRI page-size spelling for 2 MiB pages.
const HugepagePageSize = "2MB"

// HugepageSizeBytes is the 2 MiB page size in bytes (ERGON_HUGEPAGE_SIZE).
const HugepageSizeBytes = 2097152

// HugepageMountPoint is where the sized hugetlbfs is mounted in the pod.
const HugepageMountPoint = "/dev/hugepages"

// HugepageSpec is a sized hugetlbfs mount. The size is the container's
// hugepages-2Mi limit: Kubernetes isolates hugepages at container scope, so
// two containers cannot overcommit one mount.
type HugepageSpec struct {
	// Type is the mount type ("hugetlbfs").
	Type string
	// Source is the mount source ("none").
	Source string
	// Destination is where it appears in the pod.
	Destination string
	// Options are the mount options, including pagesize= and size=.
	Options []string
	// LimitBytes is the size= value: the container's hugepages-2Mi limit.
	LimitBytes int64
	// PageSize is the page-size label ("2MB", the OCI/NRI spelling). The
	// cgroup hugepage limit is NOT sent over NRI: Kubernetes already set it
	// from the pod spec, and re-sending it made containerd SEGV.
	PageSize string
}

// HugepageMount returns the sized hugetlbfs mount and the env the app reads.
// limitBytes is the container's hugepages-2Mi limit in bytes. A zero or
// negative limit is a typed refusal: the annotation asked for a mount the
// container cannot hold, and Aeron refuses an unsized hugetlbfs.
func HugepageMount(limitBytes int64) (HugepageSpec, []KV, error) {
	if limitBytes <= 0 {
		return HugepageSpec{}, nil, grantErr("hugepages",
			"hugepages.ergon.io: the pod asks for a sized hugetlbfs but the container has no hugepages-2Mi limit; add one (requests = limits)")
	}
	spec := HugepageSpec{
		Type:        "hugetlbfs",
		Source:      "none",
		Destination: HugepageMountPoint,
		Options: []string{
			"pagesize=2M",
			fmt.Sprintf("size=%d", limitBytes),
		},
		LimitBytes: limitBytes,
		PageSize:   HugepagePageSize,
	}
	// Page size and non-sparse terms match this 2 MiB mount. AERON_DIR stays
	// unset: the process creates $ERGON_HUGEPAGES_DIR/aeron and points it
	// there. Address, gateway and PCI are not set either.
	envs := []KV{
		{EnvHugepagesDir, HugepageMountPoint},
		{EnvHugepageSize, strconv.Itoa(HugepageSizeBytes)},
		{EnvAeronPageSize, strconv.Itoa(HugepageSizeBytes)},
		{EnvAeronSparse, "false"},
	}
	return spec, envs, nil
}

// CheckHugepageAnnotation checks the `hugepages.ergon.io/<container>` value.
// The only accepted value is "2M", spelled exactly: the plugin mounts 2 MiB
// pages only.
func CheckHugepageAnnotation(v string) error {
	if v != "2M" {
		return grantErr("hugepages", "hugepages.ergon.io: %q is not supported; the only value is \"2M\" (2 MiB pages)", v)
	}
	return nil
}

// CheckCpuPartitionValue checks the `cpu-partition.ergon.io/<container>`
// value. The only accepted value is "isolated", spelled exactly.
func CheckCpuPartitionValue(v string) error {
	if v != "isolated" {
		return grantErr("partition", "cpu-partition.ergon.io: %q is not supported; the only value is \"isolated\"", v)
	}
	return nil
}

// The trio the ENA device plugin injects with rusteron.io/dpdk-ena-pair.
// DpdkEnaPair reads these three and never fills any of them in.
const (
	senderPCI     = "RUSTERON_DPDK_SENDER_PCI"
	senderCIDR    = "RUSTERON_DPDK_SENDER_IPV4_CIDR"
	senderGateway = "RUSTERON_DPDK_SENDER_GATEWAY"
)

// DpdkEnaPair is the env the plugin exports for
// `dpdk.ergon.io/<container>: "ena-pair"`: the two shortest ergon-io lines
// built from the sender trio the ENA device plugin already put in this
// container's env, plus the parsed pair. ask is the annotation value, env
// the container's environment. A bad value, a missing or empty key or an
// unparsable value is a typed refusal (fail closed) that names the key.
// The plugin copies only: it never scans sysfs, never calls dpdk-devbind
// and never fills a value from the node's default route (that is the
// management NIC), so it sets no AERON_DIR, no pre-touch and no
// RUSTERON_DPDK_HUGE_DIR.
func DpdkEnaPair(ask string, env map[string]string) ([]KV, error) {
	if ask != "ena-pair" {
		return nil, grantErr("dpdk", "dpdk.ergon.io: %q is not supported; the only value is \"ena-pair\"", ask)
	}
	// trimmed, so "  " counts as missing like "".
	pci := strings.TrimSpace(env[senderPCI])
	cidr := strings.TrimSpace(env[senderCIDR])
	gateway := strings.TrimSpace(env[senderGateway])
	var missing []string
	for _, kv := range [][2]string{{senderPCI, pci}, {senderCIDR, cidr}, {senderGateway, gateway}} {
		if kv[1] == "" {
			missing = append(missing, kv[0])
		}
	}
	if len(missing) > 0 {
		return nil, grantErr("dpdk",
			"dpdk.ergon.io: ena-pair copies the NIC pair the ENA device plugin granted this container, but %s is missing or empty; the plugin never guesses a value. Fix: give the pod the rusteron.io/dpdk-ena-pair resource",
			strings.Join(missing, ", "))
	}
	addr, prefix, hasSlash := strings.Cut(cidr, "/")
	n, convErr := strconv.Atoi(prefix)
	if !hasSlash || convErr != nil || n < 0 || n > 32 || !isIPv4(addr) {
		return nil, grantErr("dpdk",
			"dpdk.ergon.io: %s=%q is not an IPv4 cidr a.b.c.d/len with a prefix of 0 to 32", senderCIDR, cidr)
	}
	if !isIPv4(gateway) {
		return nil, grantErr("dpdk", "dpdk.ergon.io: %s=%q is not an IPv4 address", senderGateway, gateway)
	}
	if strings.ContainsAny(pci, " \t,=") {
		return nil, grantErr("dpdk", "dpdk.ergon.io: %s=%q is not a PCI address or a DPDK device name", senderPCI, pci)
	}
	return []KV{
		{EnvIOFstack, "fstack-spin,fstack_addr=" + addr + ",fstack_gateway=" + gateway + ",fstack_pci=" + pci},
		{EnvIOSmoltcp, "port=" + pci + ",ip=" + cidr + ",eal=-a " + pci},
		{EnvDpdkPCI, pci},
		{EnvDpdkAddr, addr},
		{EnvDpdkGateway, gateway},
	}, nil
}

// isIPv4 reports whether s is a dotted-quad IPv4 address.
func isIPv4(s string) bool {
	a, err := netip.ParseAddr(s)
	return err == nil && a.Is4()
}

// PartitionGrant is the child-cgroup isolated partition the createRuntime hook
// set up for the container's exclusive CPUs.
type PartitionGrant struct {
	// Child is the absolute path of the child cgroup.
	Child string
	// Exclusive is the CPU list the child holds (ERGON_ISOLATED_CPUS).
	Exclusive string
}

// ---------------------------------------------------------------------------
// env contract
// ---------------------------------------------------------------------------

// Env names this package sets. They are the contract; changing one is a
// breaking change (see names.go).
const (
	EnvRlimitMemlock  = "ERGON_RLIMIT_MEMLOCK"
	EnvRlimitNofile   = "ERGON_RLIMIT_NOFILE"
	EnvRlimitRtPrio   = "ERGON_RLIMIT_RTPRIO"
	EnvSchedFifo      = "ERGON_SCHED_FIFO"
	EnvSeccomp        = "ERGON_SECCOMP"
	EnvNumaNodes      = "ERGON_NUMA_NODES"
	EnvPtpDevice      = "ERGON_PTP_DEVICE"
	EnvIsolatedCgroup = "ERGON_ISOLATED_CGROUP"
	EnvHugepagesDir   = "ERGON_HUGEPAGES_DIR"
	EnvHugepageSize   = "ERGON_HUGEPAGE_SIZE"
	EnvCpuDmaLatency  = "ERGON_CPU_DMA_LATENCY"
	EnvNodeFacts      = "ERGON_NODE_FACTS"
	EnvAeronDir       = "AERON_DIR"
	EnvAeronPageSize  = "AERON_FILE_PAGE_SIZE"
	EnvAeronSparse    = "AERON_TERM_BUFFER_SPARSE_FILE"
	EnvIOFstack       = "ERGON_IO_FSTACK"
	EnvIOSmoltcp      = "ERGON_IO_SMOLTCP"
	EnvDpdkPCI        = "ERGON_DPDK_PCI"
	EnvDpdkAddr       = "ERGON_DPDK_ADDR"
	EnvDpdkGateway    = "ERGON_DPDK_GATEWAY"
)

// EnvGrant is the grant set one container got. A zero field means "not
// granted": its key is omitted so the app can tell "not offered" from "off".
type EnvGrant struct {
	// Rlimits are the POSIX rlimits that were added.
	Rlimits []Rlimit
	// SchedFifo is the SCHED_FIFO priority the hot thread may take, or 0.
	SchedFifo int
	// SeccompMode is the seccomp mode that was applied, or "".
	SeccompMode string
	// NumaNodes is the cpuset.mems string, or "".
	NumaNodes string
	// PtpDevice is the /dev/ptp* granted, or "".
	PtpDevice string
	// CpuqosDevice is /dev/cpu_dma_latency granted, or "".
	CpuqosDevice string
	// Partition is the isolated child cgroup, or nil.
	Partition *PartitionGrant
	// NodeFacts is the mounted /run/ergon/node.json, or "".
	NodeFacts string
	// HugepageEnvs are the env pairs from HugepageMount, or nil.
	HugepageEnvs []KV
	// DpdkEnvs are the env pairs from DpdkEnaPair, or nil.
	DpdkEnvs []KV
}

// GrantEnv builds the environment the plugin exports for the grants it gave.
func GrantEnv(g EnvGrant) []KV {
	var out []KV
	for _, r := range g.Rlimits {
		switch r.Type {
		case "RLIMIT_MEMLOCK":
			out = append(out, KV{EnvRlimitMemlock, FormatRlimit(r.Soft)})
		case "RLIMIT_NOFILE":
			out = append(out, KV{EnvRlimitNofile, FormatRlimit(r.Soft)})
		case "RLIMIT_RTPRIO":
			out = append(out, KV{EnvRlimitRtPrio, FormatRlimit(r.Soft)})
		}
	}
	if g.SchedFifo > 0 {
		out = append(out, KV{EnvSchedFifo, strconv.Itoa(g.SchedFifo)})
	}
	if g.SeccompMode != "" {
		out = append(out, KV{EnvSeccomp, g.SeccompMode})
	}
	if g.NumaNodes != "" {
		out = append(out, KV{EnvNumaNodes, g.NumaNodes})
	}
	if g.PtpDevice != "" {
		out = append(out, KV{EnvPtpDevice, g.PtpDevice})
	}
	if g.CpuqosDevice != "" {
		out = append(out, KV{EnvCpuDmaLatency, g.CpuqosDevice})
	}
	if g.Partition != nil && g.Partition.Child != "" {
		out = append(out, KV{EnvIsolatedCgroup, g.Partition.Child})
	}
	if g.NodeFacts != "" {
		out = append(out, KV{EnvNodeFacts, g.NodeFacts})
	}
	out = append(out, g.HugepageEnvs...)
	out = append(out, g.DpdkEnvs...)
	return out
}
