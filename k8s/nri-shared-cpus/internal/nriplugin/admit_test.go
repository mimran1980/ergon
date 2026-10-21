// SPDX-License-Identifier: Apache-2.0

package nriplugin

import (
	"errors"
	"strconv"
	"strings"
	"testing"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/config"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/latency"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

func testConfig(t *testing.T, shared string, opts ...func(*config.File)) *config.Config {
	t.Helper()
	f := config.File{SharedCPUs: &shared, Namespaces: []string{"trading"}}
	for _, o := range opts {
		o(&f)
	}
	data := mustYAML(t, f)
	cfg, err := config.Parse(data)
	if err != nil {
		t.Fatal(err)
	}
	return cfg
}

func mustYAML(t *testing.T, f config.File) []byte {
	t.Helper()
	// Build the YAML by hand: the File struct is tagged for json.
	var b strings.Builder
	b.WriteString("sharedCpus: \"" + *f.SharedCPUs + "\"\n")
	if f.Oversubscription != nil {
		b.WriteString("oversubscription: " + itoa(*f.Oversubscription) + "\n")
	}
	if len(f.Namespaces) > 0 {
		b.WriteString("namespaces: [" + strings.Join(f.Namespaces, ", ") + "]\n")
	}
	if f.DefaultAffinity != "" {
		b.WriteString("defaultAffinity: " + f.DefaultAffinity + "\n")
	}
	if f.HookPath != "" {
		b.WriteString("hookPath: " + f.HookPath + "\n")
	}
	if f.AnnotationOptIn {
		b.WriteString("annotationOptIn: true\n")
	}
	if f.MetricsAddress != "" {
		b.WriteString("metricsAddress: " + f.MetricsAddress + "\n")
	}
	return []byte(b.String())
}

func itoa(n int) string { return strconv.Itoa(n) }

func baseRequest() Request {
	q := int64(-1)
	s := uint64(2048) // 2 CPUs x 1024
	return Request{
		Namespace:   "trading",
		PodName:     "trader",
		PodUID:      "uid-1",
		Container:   "app",
		Annotations: map[string]string{names.MilliAnnotation: "250"},
		Env:         map[string]string{},
		Cpus:        "1,5",
		Quota:       &q,
		Shares:      &s,
	}
}

func wantRefusal(t *testing.T, err error, rule int, text string) {
	t.Helper()
	var ref *Refusal
	if !errors.As(err, &ref) {
		t.Fatalf("got %v, want a *Refusal", err)
	}
	if ref.Rule != rule {
		t.Errorf("rule = %d, want %d (msg %q)", ref.Rule, rule, ref.Msg)
	}
	if !strings.Contains(ref.Msg, text) {
		t.Errorf("msg = %q, want it to contain %q", ref.Msg, text)
	}
}

func TestAdmitNotOptedIn(t *testing.T) {
	cfg := testConfig(t, "4")
	r := baseRequest()
	r.Annotations = map[string]string{}
	d, err := Admit(cfg, r)
	if err != nil || d != nil {
		t.Fatalf("plain container: got (%v, %v), want (nil, nil)", d, err)
	}
}

func TestAdmitResourceOptIn(t *testing.T) {
	cfg := testConfig(t, "4")
	d, err := Admit(cfg, baseRequest())
	if err != nil {
		t.Fatal(err)
	}
	if d == nil || d.Milli != 250 || d.Source != "resource" || d.Need != 1 {
		t.Fatalf("decision is %+v", d)
	}
	if !d.Hook {
		t.Error("hook is off; defaultAffinity is shared")
	}
	if !d.Exclusive.Equals(cpuset.New(1, 5)) {
		t.Errorf("exclusive is %s, want 1,5", d.Exclusive)
	}
}

func TestAdmitPodAnnotationFallback(t *testing.T) {
	cfg := testConfig(t, "4", func(f *config.File) { f.AnnotationOptIn = true })
	r := baseRequest()
	r.Annotations = map[string]string{}
	r.PodAnnotations = map[string]string{names.OptInPrefix + "app": "250"}
	d, err := Admit(cfg, r)
	if err != nil {
		t.Fatal(err)
	}
	if d.Source != "annotation" || d.Milli != 250 {
		t.Fatalf("decision is %+v", d)
	}
}

func TestAdmitPodAnnotationWhenOff(t *testing.T) {
	cfg := testConfig(t, "4")
	r := baseRequest()
	r.Annotations = map[string]string{}
	r.PodAnnotations = map[string]string{names.OptInPrefix + "app": "250"}
	_, err := Admit(cfg, r)
	wantRefusal(t, err, 0, "annotation opt-in is off")
}

func TestAdmitRule1Namespace(t *testing.T) {
	cfg := testConfig(t, "4")
	r := baseRequest()
	r.Namespace = "default"
	_, err := Admit(cfg, r)
	wantRefusal(t, err, 1, "namespace default may not use shared CPUs")
}

func TestAdmitRule2NoCpuset(t *testing.T) {
	cfg := testConfig(t, "4")
	r := baseRequest()
	r.Cpus = ""
	_, err := Admit(cfg, r)
	wantRefusal(t, err, 2, "no cpuset from the kubelet")
}

func TestAdmitRule3Quota(t *testing.T) {
	cfg := testConfig(t, "4")
	r := baseRequest()
	q := int64(200000)
	r.Quota = &q
	_, err := Admit(cfg, r)
	wantRefusal(t, err, 3, "CPU quota is set")
}

func TestAdmitRule4NotExclusive(t *testing.T) {
	cfg := testConfig(t, "4")

	// shares not a multiple of 1024
	r := baseRequest()
	s := uint64(1500)
	r.Shares = &s
	_, err := Admit(cfg, r)
	wantRefusal(t, err, 4, "cpuset is not exclusive")

	// shares say 2 CPUs, cpuset has 1
	r = baseRequest()
	s2 := uint64(2048)
	r.Shares = &s2
	r.Cpus = "1"
	_, err = Admit(cfg, r)
	wantRefusal(t, err, 4, "cpuset is not exclusive")

	// Burstable: shares not a whole multiple
	r = baseRequest()
	s3 := uint64(512)
	r.Shares = &s3
	r.Cpus = "1"
	_, err = Admit(cfg, r)
	wantRefusal(t, err, 4, "cpuset is not exclusive")

	// shares unset
	r = baseRequest()
	r.Shares = nil
	_, err = Admit(cfg, r)
	wantRefusal(t, err, 4, "cpuset is not exclusive")
}

func TestAdmitRule5Overlap(t *testing.T) {
	cfg := testConfig(t, "0,1,4")
	r := baseRequest()
	r.Cpus = "1,5" // CPU 1 is shared
	_, err := Admit(cfg, r)
	wantRefusal(t, err, 5, "both exclusive and shared")
}

func TestAdmitRule6TooManyShared(t *testing.T) {
	cfg := testConfig(t, "4") // only one shared CPU
	r := baseRequest()
	r.Annotations = map[string]string{names.MilliAnnotation: "2001"} // needs 2
	_, err := Admit(cfg, r)
	wantRefusal(t, err, 6, "this node has 1")
}

func TestAdmitRule7EnvByHand(t *testing.T) {
	cfg := testConfig(t, "4")
	r := baseRequest()
	r.Annotations = map[string]string{}
	r.Env = map[string]string{names.EnvMilli: "250"}
	_, err := Admit(cfg, r)
	wantRefusal(t, err, 7, "is set by hand")
}

func TestAdmitOptOutAnnotation(t *testing.T) {
	cfg := testConfig(t, "4")
	r := baseRequest()
	r.PodAnnotations = map[string]string{names.AffinityPrefix + "app": "all"}
	d, err := Admit(cfg, r)
	if err != nil {
		t.Fatal(err)
	}
	if d.Hook {
		t.Error("hook is on despite the opt-out annotation")
	}
}

func TestAdmitForceHookAnnotation(t *testing.T) {
	cfg := testConfig(t, "4", func(f *config.File) { f.DefaultAffinity = config.AffinityAll })
	r := baseRequest()
	r.PodAnnotations = map[string]string{names.AffinityPrefix + "app": "shared"}
	d, err := Admit(cfg, r)
	if err != nil {
		t.Fatal(err)
	}
	if !d.Hook {
		t.Error("hook is off despite the opt-in annotation")
	}
}

func TestAdmitBadAffinityAnnotation(t *testing.T) {
	cfg := testConfig(t, "4")
	r := baseRequest()
	r.PodAnnotations = map[string]string{names.AffinityPrefix + "app": "yes"}
	_, err := Admit(cfg, r)
	wantRefusal(t, err, 0, "use \"shared\" or \"all\"")
}

func TestAdmitEnvReplacementWarns(t *testing.T) {
	cfg := testConfig(t, "4")
	r := baseRequest()
	r.Env = map[string]string{
		names.EnvShared:   "9",
		names.EnvIsolated: "3",
	}
	d, err := Admit(cfg, r)
	if err != nil {
		t.Fatal(err)
	}
	if len(d.Warnings) == 0 {
		t.Fatal("no warning for a pod-set env the plugin replaces")
	}
	joined := strings.Join(d.Warnings, "\n")
	if !strings.Contains(joined, names.EnvShared) {
		t.Errorf("warnings %q do not name %s", joined, names.EnvShared)
	}
}

func TestAdmitBothOptInsResourceWins(t *testing.T) {
	cfg := testConfig(t, "4", func(f *config.File) { f.AnnotationOptIn = true })
	r := baseRequest()
	r.Annotations = map[string]string{names.MilliAnnotation: "250"}
	r.PodAnnotations = map[string]string{names.OptInPrefix + "app": "500"}
	d, err := Admit(cfg, r)
	if err != nil {
		t.Fatal(err)
	}
	if d.Milli != 250 || d.Source != "resource" {
		t.Fatalf("decision is %+v, want the resource to win", d)
	}
	if len(d.Warnings) == 0 {
		t.Error("no warning that the pod annotation was ignored")
	}
}

func TestAdmitBadAnnotationValue(t *testing.T) {
	cfg := testConfig(t, "4")
	r := baseRequest()
	r.Annotations = map[string]string{names.MilliAnnotation: "250m"}
	_, err := Admit(cfg, r)
	wantRefusal(t, err, 0, "not a whole number of millicpus")
}

func TestEnvFormats(t *testing.T) {
	e := cpuset.New(1, 2, 5, 6)
	s := cpuset.New(4)
	cores := []cpuset.Set{cpuset.New(1, 5), cpuset.New(2, 6)}
	kv := Env(e, s, cores, 250)
	got := map[string]string{}
	for _, x := range kv {
		got[x.Key] = x.Value
	}
	want := map[string]string{
		names.EnvIsolated:          "1,2,5,6",
		names.EnvIsolatedCores:     "1,5;2,6",
		names.EnvShared:            "4",
		names.EnvMilli:             "250",
		names.EnvHelper:            "4",
		names.EnvOpenShiftIsolated: "1-2,5-6",
		names.EnvOpenShiftShared:   "4",
	}
	for k, v := range want {
		if got[k] != v {
			t.Errorf("%s = %q, want %q", k, got[k], v)
		}
	}
	if len(got) != len(want) {
		t.Errorf("env has %d keys, want %d: %v", len(got), len(want), got)
	}
}

func TestHookShape(t *testing.T) {
	h := Hook("/opt/ergon/bin/ergon-cpus", cpuset.New(4, 0), nil)
	if h.Path != "/opt/ergon/bin/ergon-cpus" {
		t.Errorf("hook path is %q", h.Path)
	}
	wantArgs := []string{"/opt/ergon/bin/ergon-cpus", "hook", "--shared", "0,4"}
	for i := range wantArgs {
		if h.Args[i] != wantArgs[i] {
			t.Errorf("hook arg %d is %q, want %q", i, h.Args[i], wantArgs[i])
		}
	}
	if h.Timeout == nil || h.Timeout.Value != HookTimeoutSeconds {
		t.Errorf("hook timeout is %v, want %d", h.Timeout, HookTimeoutSeconds)
	}
}

func TestHookShapeWithPartition(t *testing.T) {
	part := &latency.PartitionGrant{Child: "/sys/fs/cgroup/pod/ctr/ergon-isolated", Exclusive: "1,5"}
	h := Hook("/opt/ergon/bin/ergon-cpus", cpuset.Set{}, part)
	wantArgs := []string{
		"/opt/ergon/bin/ergon-cpus", "hook",
		"--isolated", "1,5",
		"--partition-child", "/sys/fs/cgroup/pod/ctr/ergon-isolated",
	}
	if len(h.Args) != len(wantArgs) {
		t.Fatalf("hook args are %v, want %v", h.Args, wantArgs)
	}
	for i := range wantArgs {
		if h.Args[i] != wantArgs[i] {
			t.Errorf("hook arg %d is %q, want %q", i, h.Args[i], wantArgs[i])
		}
	}
}

func TestHookShapeWithBoth(t *testing.T) {
	part := &latency.PartitionGrant{Child: "/c/ergon-isolated", Exclusive: "1,5"}
	h := Hook("/opt/ergon/bin/ergon-cpus", cpuset.New(0), part)
	joined := strings.Join(h.Args, " ")
	if !strings.Contains(joined, "--shared 0") || !strings.Contains(joined, "--isolated 1,5") {
		t.Errorf("hook args %v must carry both --shared and --isolated", h.Args)
	}
}

// A latency pod that asks only for the grants (no shared-CPU resource) must
// still get them. The grants are opt-in by their own annotations and are not
// tied to the millicpu resource.
func TestAdmitGrantsWithoutSharedCPU(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.Annotations = map[string]string{} // no ergon.io/shared-millicpus
	r.PodAnnotations = map[string]string{
		names.RlimitsPrefix + r.Container: "memlock,rt-prio",
	}
	d, err := Admit(cfg, r)
	if err != nil {
		t.Fatal(err)
	}
	if d == nil {
		t.Fatal("a container with grant annotations and no shared-CPU request must still be adjusted")
	}
	if d.Grants == nil || len(d.Grants.Rlimits) != 2 {
		t.Fatalf("grants = %+v, want two rlimits", d.Grants)
	}
	if d.Milli != 0 {
		t.Errorf("milli = %d, want 0 (no shared-CPU opt-in)", d.Milli)
	}
}

// An unknown grant kind must be refused even when the pod did not opt into
// shared CPUs: fail closed, never silently drop the grant.
func TestAdmitGrantRefusedWithoutSharedCPU(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.Annotations = map[string]string{}
	r.PodAnnotations = map[string]string{
		names.RlimitsPrefix + r.Container: "bogus",
	}
	_, err := Admit(cfg, r)
	if err == nil {
		t.Fatal("want a refusal for an unknown rlimit kind")
	}
	if !strings.Contains(err.Error(), "bogus") {
		t.Errorf("error %q must name the bad kind", err)
	}
}

// A partition-only container (no shared-CPU opt-in) still needs its exclusive
// CPUs: they are the kubelet's cpuset.
func TestAdmitCpuPartitionOnlySetsExclusive(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.Annotations = map[string]string{}
	r.PodAnnotations = map[string]string{
		names.CpuPartitionPrefix + r.Container: names.CpuPartitionValue,
	}
	d, err := Admit(cfg, r)
	if err != nil {
		t.Fatal(err)
	}
	if d == nil || d.Grants == nil || !d.Grants.CpuPartition {
		t.Fatalf("want the cpu-partition grant, got %+v", d)
	}
	if d.Exclusive.String() != "1,5" {
		t.Errorf("exclusive = %q, want the kubelet's cpuset 1,5", d.Exclusive)
	}
	if d.Milli != 0 {
		t.Errorf("milli = %d, want 0 (no shared-CPU opt-in)", d.Milli)
	}
}

// The exclusive CPUs of a partition must never overlap the node's shared
// CPUs: an isolated shared CPU would take load balancing away from helpers.
func TestAdmitCpuPartitionSharedOverlapRefused(t *testing.T) {
	cfg := grantConfig(t)
	cfg.Shared = cpuset.New(5)
	r := baseRequest()
	r.Annotations = map[string]string{}
	r.PodAnnotations = map[string]string{
		names.CpuPartitionPrefix + r.Container: names.CpuPartitionValue,
	}
	_, err := Admit(cfg, r)
	if err == nil {
		t.Fatal("CPU 5 is both exclusive and shared; the partition must be refused")
	}
	if !strings.Contains(err.Error(), "shared") {
		t.Errorf("refusal %q must name the shared CPUs", err)
	}
}

// A partition with no cpuset from the kubelet is refused: the hook has
// nothing to put in the child.
func TestAdmitCpuPartitionNoCpusetRefused(t *testing.T) {
	cfg := grantConfig(t)
	r := baseRequest()
	r.Annotations = map[string]string{}
	r.Cpus = ""
	r.PodAnnotations = map[string]string{
		names.CpuPartitionPrefix + r.Container: names.CpuPartitionValue,
	}
	_, err := Admit(cfg, r)
	if err == nil {
		t.Fatal("a partition without a cpuset must be refused")
	}
}
