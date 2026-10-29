// SPDX-License-Identifier: Apache-2.0

package nriplugin

import (
	"errors"
	"strings"
	"testing"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/config"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/latency"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

// grantConfig is a config with every grant knob turned on.
func grantConfig(t *testing.T) *config.Config {
	t.Helper()
	shared := "0"
	cfg, err := config.Parse([]byte(`sharedCpus: "0"
namespaces: [trading]
annotationOptIn: true
rlimits:
  memlock: "inf"
  nofile: "65536"
  rtPrio: "50"
seccomp:
  ioUring: allow
schedFifo: thread
ptp: auto
`))
	if err != nil {
		t.Fatal(err)
	}
	cfg.NodeNumaNodes = []int{0, 1}
	cfg.NicNumaNode = 1
	cfg.HostPtp = []string{"/dev/ptp0"}
	_ = shared
	return cfg
}

func TestResolveGrantsRlimits(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.MilliAnnotation:             "250",
		names.RlimitsPrefix + r.Container: "memlock,rt-prio,nofile",
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g == nil {
		t.Fatal("want grants, got nil")
	}
	if len(g.Rlimits) != 3 {
		t.Fatalf("got %d rlimits, want 3: %v", len(g.Rlimits), g.Rlimits)
	}
	kinds := map[string]bool{}
	for _, rl := range g.Rlimits {
		kinds[rl.Type] = true
	}
	for _, want := range []string{"RLIMIT_MEMLOCK", "RLIMIT_RTPRIO", "RLIMIT_NOFILE"} {
		if !kinds[want] {
			t.Errorf("missing %s in %v", want, g.Rlimits)
		}
	}
}

func TestResolveGrantsRlimitsRefused(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.RlimitsPrefix + r.Container: "memlock,bogus",
	}
	_, err := resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("want a refusal for an unknown rlimit kind")
	}
	var ref *Refusal
	if !errors.As(err, &ref) && !errors.As(err, new(*latency.GrantError)) {
		t.Errorf("want a typed error, got %T: %v", err, err)
	}
	if !strings.Contains(err.Error(), "bogus") {
		t.Errorf("error %q must name the bad kind", err)
	}
}

func TestResolveGrantsSeccompIOUring(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.SeccompPolicy = &latency.SeccompPolicy{DefaultAction: "SCMP_ACT_ERRNO"}
	r.PodAnnotations = map[string]string{
		names.SeccompPrefix + r.Container: "io-uring",
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g.SeccompMode != latency.ModeIOUring {
		t.Errorf("mode %q, want %q", g.SeccompMode, latency.ModeIOUring)
	}
	if g.Seccomp == nil {
		t.Fatal("want a widened policy")
	}
	found := false
	for _, s := range g.Seccomp.Syscalls {
		for _, n := range s.Names {
			if strings.HasPrefix(n, "io_uring") && s.Action == "SCMP_ACT_ALLOW" {
				found = true
			}
		}
	}
	if !found {
		t.Error("io_uring must be allowed")
	}
}

func TestResolveGrantsSeccompUnconfinedRefused(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.SeccompPrefix + r.Container: "unconfined",
	}
	_, err := resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("unconfined must be refused")
	}
	if !strings.Contains(err.Error(), "unconfined") {
		t.Errorf("error %q must name unconfined", err)
	}
}

func TestResolveGrantsSeccompOff(t *testing.T) {
	// ioUring: off in the config: asking for io-uring is a fail-closed refusal.
	shared := "0"
	_ = shared
	cfg, err := config.Parse([]byte("sharedCpus: \"0\"\nnamespaces: [trading]\nseccomp:\n  ioUring: off\n"))
	if err != nil {
		t.Fatal(err)
	}
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.SeccompPrefix + r.Container: "io-uring",
	}
	_, err = resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("want a refusal when ioUring is off")
	}
	if !strings.Contains(err.Error(), "ioUring") {
		t.Errorf("error %q must name the config knob", err)
	}
}

func TestResolveGrantsNuma(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.NumaPrefix + r.Container: "0",
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g.NumaMems != "0" {
		t.Errorf("mems %q, want 0", g.NumaMems)
	}
}

func TestResolveGrantsNumaDefaultNic(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.NumaPrefix + r.Container: "",
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g.NumaMems != "1" {
		t.Errorf("mems %q, want the NIC node 1", g.NumaMems)
	}
}

// A pod that asks for a NUMA binding is refused rather than granted nothing
// when the node's topology was never read.
func TestResolveGrantsNumaUnreadTopology(t *testing.T) {
	cfg := grantConfig(t)
	cfg.NodeNumaNodes = nil
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.NumaPrefix + r.Container: "0",
	}
	_, err := resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("want a refusal when the node's NUMA topology is unknown")
	}
	if !strings.Contains(err.Error(), "NUMA topology") {
		t.Errorf("error %q must name the missing topology", err)
	}
}

// "": the NIC's node. With several nodes and none recorded, node 0 would be
// a guess, so the pod is told to name the node itself.
func TestResolveGrantsNumaDefaultNicUnknown(t *testing.T) {
	cfg := grantConfig(t)
	cfg.NicNumaNode = -1
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.NumaPrefix + r.Container: "",
	}
	_, err := resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("want a refusal when the data NIC's node is unknown on a multi-node machine")
	}
	if !strings.Contains(err.Error(), "numa.ergon.io") {
		t.Errorf("error %q must name the annotation to set", err)
	}
}

func TestResolveGrantsPtp(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.PtpPrefix + r.Container: "auto",
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g.PtpDevice != "/dev/ptp0" {
		t.Errorf("ptp %q, want /dev/ptp0", g.PtpDevice)
	}
}

func TestResolveGrantsPtpMissing(t *testing.T) {
	cfg := grantConfig(t)
	cfg.HostPtp = nil
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.PtpPrefix + r.Container: "auto",
	}
	_, err := resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("want a refusal when the node has no PHC")
	}
}

func TestResolveGrantsSchedFifo(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.RlimitsPrefix + r.Container:   "rt-prio",
		names.SchedFifoPrefix + r.Container: "thread",
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g.SchedFifo != 50 {
		t.Errorf("fifo prio %d, want 50 from the rtPrio config", g.SchedFifo)
	}
}

func TestResolveGrantsSchedFifoOffByConfig(t *testing.T) {
	cfg := grantConfig(t)
	cfg.SchedFifo = config.SchedFifoOff
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.SchedFifoPrefix + r.Container: "thread",
	}
	_, err := resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("want a refusal when schedFifo is off")
	}
}

func TestResolveGrantsNothing(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g != nil {
		t.Fatalf("want no grants without annotations, got %+v", g)
	}
}

func TestResolveGrantsSeccompDefaultMode(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	// One real grant forces a non-nil Grants so we can read SeccompMode.
	r.PodAnnotations = map[string]string{
		names.RlimitsPrefix + r.Container: "memlock",
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g == nil {
		t.Fatal("want grants")
	}
	if g.SeccompMode != latency.ModeRuntimeDefault {
		t.Errorf("mode %q, want runtime-default when nothing is asked for", g.SeccompMode)
	}
}

func TestSysctlRefusalNamesNode(t *testing.T) {
	msg := SysctlRefusal("net.core.rmem_max")
	if !strings.Contains(msg, "net.core.rmem_max") || !strings.Contains(msg, "node") {
		t.Errorf("message %q must name the sysctl and say node", msg)
	}
}

// --- item 1: sized hugetlbfs mount ---

func TestResolveGrantsHugepageMount(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.HugepagesPrefix + r.Container: "2M",
	}
	r.HugepageLimitBytes = 256 * 1024 * 1024
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g == nil {
		t.Fatal("want grants, got nil")
	}
	if g.Hugepage == nil {
		t.Fatal("want a hugepage mount")
	}
	if g.Hugepage.LimitBytes != 256*1024*1024 {
		t.Errorf("limit = %d, want the container's hugepages-2Mi limit", g.Hugepage.LimitBytes)
	}
	if g.Hugepage.Destination != "/dev/hugepages" {
		t.Errorf("destination = %q", g.Hugepage.Destination)
	}
	opts := strings.Join(g.Hugepage.Options, ",")
	if !strings.Contains(opts, "pagesize=2M") || !strings.Contains(opts, "size=268435456") {
		t.Errorf("options %v must carry pagesize=2M and size=<limit>", g.Hugepage.Options)
	}
	env := kvMapOf(GrantEnv(g, cfg))
	if env["ERGON_HUGEPAGES_DIR"] != "/dev/hugepages" {
		t.Errorf("ERGON_HUGEPAGES_DIR = %q", env["ERGON_HUGEPAGES_DIR"])
	}
	if env["ERGON_HUGEPAGE_SIZE"] != "2097152" {
		t.Errorf("ERGON_HUGEPAGE_SIZE = %q", env["ERGON_HUGEPAGE_SIZE"])
	}
	if env["AERON_FILE_PAGE_SIZE"] != "2097152" {
		t.Errorf("AERON_FILE_PAGE_SIZE = %q", env["AERON_FILE_PAGE_SIZE"])
	}
	if env["AERON_TERM_BUFFER_SPARSE_FILE"] != "false" {
		t.Errorf("AERON_TERM_BUFFER_SPARSE_FILE = %q", env["AERON_TERM_BUFFER_SPARSE_FILE"])
	}
	if _, ok := env["ERGON_HUGEPAGES"]; ok {
		t.Error("ERGON_HUGEPAGES is replaced by ERGON_HUGEPAGES_DIR/_SIZE")
	}
	if _, ok := env["AERON_DIR"]; ok {
		t.Error("the app points AERON_DIR at $ERGON_HUGEPAGES_DIR/aeron itself; the plugin must not set it")
	}
}

func TestResolveGrantsHugepageNoLimitRefused(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.HugepagesPrefix + r.Container: "2M",
	}
	r.HugepageLimitBytes = 0
	_, err := resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("annotation without a hugepages-2Mi limit must be refused (fail closed)")
	}
	if !strings.Contains(err.Error(), "hugepages-2Mi") {
		t.Errorf("refusal %q must name the fix", err)
	}
}

func TestResolveGrantsHugepageBadValue(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.HugepagesPrefix + r.Container: "1G",
	}
	r.HugepageLimitBytes = 256 * 1024 * 1024
	_, err := resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("only \"2M\" is accepted")
	}
	if !strings.Contains(err.Error(), "2M") {
		t.Errorf("refusal %q must name the accepted value", err)
	}
}

// --- item 2: cpu-partition ---

func TestResolveGrantsCpuPartition(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.CpuPartitionPrefix + r.Container: names.CpuPartitionValue,
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g == nil || !g.CpuPartition {
		t.Fatalf("want the cpu-partition grant, got %+v", g)
	}
}

func TestResolveGrantsCpuPartitionBadValue(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.CpuPartitionPrefix + r.Container: "root",
	}
	_, err := resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("only \"isolated\" is accepted")
	}
	if !strings.Contains(err.Error(), "isolated") {
		t.Errorf("refusal %q must name the accepted value", err)
	}
}

// --- item 5: cpuqos (x86 only) ---

func TestResolveGrantsCpuqosArm64Ignored(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.CpuqosPrefix + r.Container: "1",
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatalf("arm64 must ignore the cpuqos annotation, got %v", err)
	}
	if g != nil && g.CpuqosDevice != "" {
		t.Errorf("arm64 must grant no cpu_dma_latency, got %q", g.CpuqosDevice)
	}
}

func TestResolveGrantsCpuqosAMD64NoDevice(t *testing.T) {
	cfg := grantConfig(t)
	cfg.GOARCH = "amd64"
	cfg.HostCpuDmaLatency = ""
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.CpuqosPrefix + r.Container: "1",
	}
	_, err := resolveGrants(cfg, &r)
	if err == nil {
		t.Fatal("on amd64 a missing /dev/cpu_dma_latency must fail closed")
	}
	if !strings.Contains(err.Error(), "cpu_dma_latency") {
		t.Errorf("refusal %q must name the device", err)
	}
}

func TestResolveGrantsCpuqosAMD64Grants(t *testing.T) {
	cfg := grantConfig(t)
	cfg.GOARCH = "amd64"
	cfg.HostCpuDmaLatency = "/dev/cpu_dma_latency"
	r := baseRequest()
	r.PodAnnotations = map[string]string{
		names.CpuqosPrefix + r.Container: "1",
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		t.Fatal(err)
	}
	if g == nil || g.CpuqosDevice != "/dev/cpu_dma_latency" {
		t.Fatalf("want /dev/cpu_dma_latency, got %+v", g)
	}
	env := kvMapOf(GrantEnv(g, cfg))
	if env["ERGON_CPU_DMA_LATENCY"] != "/dev/cpu_dma_latency" {
		t.Errorf("ERGON_CPU_DMA_LATENCY = %q", env["ERGON_CPU_DMA_LATENCY"])
	}
}

// kvMapOf is the grants_test env helper.
func kvMapOf(kvs []latency.KV) map[string]string {
	m := make(map[string]string, len(kvs))
	for _, kv := range kvs {
		m[kv.Key] = kv.Value
	}
	return m
}
