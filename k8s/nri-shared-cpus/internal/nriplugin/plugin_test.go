// SPDX-License-Identifier: Apache-2.0

package nriplugin

import (
	"context"
	"log/slog"
	"os"
	"strconv"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/containerd/nri/pkg/api"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/config"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/latency"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/metrics"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

type testPluginResult struct {
	*Plugin
	m *metrics.Metrics
}

func testPlugin(t *testing.T, shared string, opts ...func(*config.File)) testPluginResult {
	t.Helper()
	return testPluginSiblings(t, shared, testSiblings(), opts...)
}

// testPluginSiblings is testPlugin on a topology of the caller's choosing.
// The n/n+4 map in testSiblings is eight CPUs and four cores, which is not
// always enough: a test that needs three whole exclusive cores beside two
// shared CPUs has to bring a wider node.
func testPluginSiblings(t *testing.T, shared string, siblings map[int]cpuset.Set, opts ...func(*config.File)) testPluginResult {
	t.Helper()
	text := "sharedCpus: \"" + shared + "\"\nnamespaces: [trading]\n"
	// Options are applied to the YAML text through the File shape the config
	// package accepts.
	f := &config.File{SharedCPUs: &shared, Namespaces: []string{"trading"}}
	for _, o := range opts {
		o(f)
	}
	if f.Oversubscription != nil {
		text += "oversubscription: " + strconv.Itoa(*f.Oversubscription) + "\n"
	}
	if f.AnnotationOptIn {
		text += "annotationOptIn: true\n"
	}
	if f.DefaultAffinity != "" {
		text += "defaultAffinity: " + f.DefaultAffinity + "\n"
	}
	cfg, err := config.Parse([]byte(text))
	if err != nil {
		t.Fatal(err)
	}
	m := metrics.New()
	p := New(cfg, siblings, slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError})), m)
	p.ResendDelay = 0
	p.After = func(_ time.Duration, fn func()) { go fn() }
	return testPluginResult{Plugin: p, m: m}
}

// testSiblings is the node's core topology for the tests: x86 with the two
// threads of a core at n and n+4, the shape the cpuset tests use. So 1 and 5
// are one physical core and 0 and 4 are another. A real node has
// thread_siblings_list for every online CPU; an empty map is not a node, it is
// a topology the plugin must refuse to guess from.
func testSiblings() map[int]cpuset.Set {
	out := map[int]cpuset.Set{}
	for c := 0; c < 4; c++ {
		out[c] = cpuset.New(c, c+4)
		out[c+4] = cpuset.New(c, c+4)
	}
	return out
}

func pod() *api.PodSandbox {
	return &api.PodSandbox{
		Id:          "sandbox-1",
		Name:        "trader",
		Namespace:   "trading",
		Uid:         "uid-1",
		Annotations: map[string]string{},
	}
}

func ctr(id string, cpus string, env map[string]string, ann map[string]string) *api.Container {
	envList := make([]string, 0, len(env))
	for k, v := range env {
		envList = append(envList, k+"="+v)
	}
	q := int64(-1)
	s := uint64(2048)
	return &api.Container{
		Id:           id,
		Name:         "app",
		PodSandboxId: "sandbox-1",
		Annotations:  ann,
		Env:          envList,
		State:        api.ContainerState_CONTAINER_CREATED,
		Linux: &api.LinuxContainer{
			Resources: &api.LinuxResources{
				Cpu: &api.LinuxCPU{
					Cpus:   cpus,
					Quota:  &api.OptionalInt64{Value: q},
					Shares: &api.OptionalUInt64{Value: s},
				},
			},
		},
	}
}

// applyAdjustment folds a create adjustment into the container the way the
// runtime does: the env it added and the cpuset it set are the live state a
// later UpdateContainer or Synchronize reads. A test that creates a container
// and then updates it needs this, or the update sees the create request rather
// than what the container became — a cpuset that was never widened and an env
// that never gained ERGON_ISOLATED_CGROUP.
func applyAdjustment(t *testing.T, c *api.Container, adj *api.ContainerAdjustment) {
	t.Helper()
	if adj == nil {
		return
	}
	for _, e := range adj.GetEnv() {
		replaced := false
		for i, cur := range c.Env {
			if k, _, ok := strings.Cut(cur, "="); ok && k == e.Key {
				c.Env[i] = e.Key + "=" + e.Value
				replaced = true
				break
			}
		}
		if !replaced {
			c.Env = append(c.Env, e.Key+"="+e.Value)
		}
	}
	l := adj.GetLinux()
	if l == nil {
		return
	}
	res := l.GetResources()
	if res == nil {
		return
	}
	cpu := res.GetCpu()
	if cpu == nil || cpu.GetCpus() == "" {
		return
	}
	c.Linux.Resources.Cpu.Cpus = cpu.GetCpus()
}

func TestCreateAdjustment(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil {
		t.Fatal(err)
	}
	if adj == nil {
		t.Fatal("no adjustment")
	}
	cpu := adj.GetLinux().GetResources().GetCpu()
	// E is 1,5 and M=250 needs one shared CPU, so the union has 3 CPUs:
	// 2 exclusive + the chosen shared CPU.
	got, err := cpuset.Parse(cpu.GetCpus())
	if err != nil {
		t.Fatalf("cpuset %q: %v", cpu.GetCpus(), err)
	}
	if got.Len() != 3 {
		t.Errorf("cpuset is %q, want 3 CPUs (2 exclusive + 1 shared)", cpu.GetCpus())
	}
	if !cpuset.New(1, 5).IsSubsetOf(got) {
		t.Errorf("cpuset %q does not contain the exclusive CPUs 1,5", cpu.GetCpus())
	}
	shared := got.Difference(cpuset.New(1, 5))
	if shared.Len() != 1 || !shared.IsSubsetOf(cpuset.New(0, 4)) {
		t.Errorf("shared part is %q, want one of 0,4", shared)
	}
	env := map[string]string{}
	for _, e := range adj.GetEnv() {
		env[e.Key] = e.Value
	}
	if env[names.EnvIsolated] != "1,5" {
		t.Errorf("%s = %q, want 1,5", names.EnvIsolated, env[names.EnvIsolated])
	}
	if env[names.EnvShared] != "0" && env[names.EnvShared] != "4" {
		t.Errorf("%s = %q, want one shared CPU", names.EnvShared, env[names.EnvShared])
	}
	if env[names.EnvHelper] != env[names.EnvShared] {
		t.Errorf("%s = %q, %s = %q; they must match", names.EnvHelper, env[names.EnvHelper], names.EnvShared, env[names.EnvShared])
	}
	if env[names.EnvMilli] != "250" {
		t.Errorf("%s = %q, want 250", names.EnvMilli, env[names.EnvMilli])
	}
	hooks := adj.GetHooks()
	if hooks == nil || len(hooks.GetCreateRuntime()) != 1 {
		t.Fatalf("hooks are %v, want one createRuntime hook", hooks)
	}
	h := hooks.GetCreateRuntime()[0]
	if h.GetPath() != p.cfg.HookPath {
		t.Errorf("hook path is %q, want %q", h.GetPath(), p.cfg.HookPath)
	}
	if len(h.GetArgs()) != 4 || h.GetArgs()[2] != "--shared" || h.GetArgs()[3] != env[names.EnvShared] {
		t.Errorf("hook args are %v, want hook --shared <S>", h.GetArgs())
	}
	// Shared-only: there is no isolated child to take back down.
	if got := hooks.GetPoststop(); len(got) != 0 {
		t.Errorf("poststop hooks are %v, want none without a partition", got)
	}
}

// TestCreateGroupsSMTSiblingsIntoOneCore is the regression for the topology
// map being built from the shared CPUs alone. E is disjoint from S, so that
// map had no entry for any exclusive CPU and Cores called each of them a
// physical core of its own: two threads of one core were published as two
// cores, and an app placing two "separate" hot cores from
// ERGON_ISOLATED_CORES put both on the same execution units.
func TestCreateGroupsSMTSiblingsIntoOneCore(t *testing.T) {
	p := testPlugin(t, "0")
	c := ctr("ctr-1", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil {
		t.Fatal(err)
	}
	env := map[string]string{}
	for _, e := range adj.GetEnv() {
		env[e.Key] = e.Value
	}
	if env[names.EnvIsolatedCores] != "1,5" {
		t.Errorf("%s = %q, want 1,5: 1 and 5 are the two threads of one core", names.EnvIsolatedCores, env[names.EnvIsolatedCores])
	}
}

// TestCreateRefusesACoreSplitAcrossExclusiveAndShared: an exclusive CPU whose
// SMT sibling is shared is not isolated. The hot thread and the helpers still
// time-share one physical core, which is the contention the partition exists
// to remove. Refuse it; a container must never start claiming an isolation it
// does not have.
func TestCreateRefusesACoreSplitAcrossExclusiveAndShared(t *testing.T) {
	p := testPlugin(t, "5")
	// 1 and 5 are the two threads of one core. The container keeps 1 and 2;
	// 5 is in the shared pool, so core 1,5 is split between them.
	c := ctr("ctr-1", "1,2", nil, map[string]string{names.MilliAnnotation: "250"})
	_, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err == nil {
		t.Fatal("a core split between exclusive 1 and shared 5 must be refused")
	}
	if !strings.Contains(err.Error(), "core") || !strings.Contains(err.Error(), "1,5") {
		t.Errorf("refusal %q must name the split core", err)
	}
}

// TestCreateRefusesAnIncompleteTopology: without thread_siblings_list the core
// of a CPU is unknown, so the split check cannot run. Cores would guess "a
// core of its own", which is exactly the guess that published two siblings as
// two cores. Refuse rather than guess.
func TestCreateRefusesAnIncompleteTopology(t *testing.T) {
	p := testPlugin(t, "0")
	p.siblings = map[int]cpuset.Set{}
	c := ctr("ctr-1", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	_, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err == nil {
		t.Fatal("a container with no core topology must be refused")
	}
	if !strings.Contains(err.Error(), "thread_siblings_list") {
		t.Errorf("refusal %q must name the missing file", err)
	}
}

func TestCreateAdjustmentNoHookOnOptOut(t *testing.T) {
	p := testPlugin(t, "0,4")
	podObj := pod()
	podObj.Annotations = map[string]string{names.AffinityPrefix + "app": "all"}
	c := ctr("ctr-1", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	adj, _, err := p.CreateContainer(context.Background(), podObj, c)
	if err != nil {
		t.Fatal(err)
	}
	if adj.GetHooks() != nil {
		t.Errorf("hooks are %v, want none", adj.GetHooks())
	}
}

func TestCreateRefusesAndCounts(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	podObj := pod()
	podObj.Namespace = "default"
	_, _, err := p.CreateContainer(context.Background(), podObj, c)
	if err == nil {
		t.Fatal("create in a non-allowlisted namespace was accepted")
	}
	if got := p.m.Counter("refused/namespace"); got != 1 {
		t.Errorf("refused/namespace = %d, want 1", got)
	}
}

func TestCreateNonOptedInIsUntouched(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", nil, nil)
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil || adj != nil {
		t.Fatalf("got (%v, %v), want (nil, nil)", adj, err)
	}
}

func TestUpdateRewidens(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "0,1,4,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
	}, map[string]string{names.MilliAnnotation: "250"})
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{Cpus: "1,5"}} // kubelet narrowed it
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 1 {
		t.Fatalf("got %d updates, want 1", len(updates))
	}
	if !sameCPUs(t, updates[0].GetLinux().GetResources().GetCpu().GetCpus(), "0,1,5") {
		t.Errorf("union is %q, want 0,1,5", updates[0].GetLinux().GetResources().GetCpu().GetCpus())
	}
	if got := p.m.Counter("rewidened/update"); got != 1 {
		t.Errorf("rewidened/update = %d, want 1", got)
	}
}

func TestUpdateMemoryOnlyIsUntouched(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "0,1,4,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
	}, map[string]string{names.MilliAnnotation: "250"})
	res := &api.LinuxResources{Memory: &api.LinuxMemory{Limit: &api.OptionalInt64{Value: 1 << 30}}}
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates for a memory-only update, want 0", len(updates))
	}
}

func TestUpdateNonOursIsUntouched(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{"PATH": "/bin"}, nil)
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{Cpus: "1"}}
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates for a container that is not ours, want 0", len(updates))
	}
}

func TestUpdateResizeMarksStale(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "0,1,4,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
	}, map[string]string{names.MilliAnnotation: "250"})
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{Cpus: "1,2,5"}} // resized
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 1 {
		t.Fatalf("got %d updates, want 1", len(updates))
	}
	if !sameCPUs(t, updates[0].GetLinux().GetResources().GetCpu().GetCpus(), "0,1,2,5") {
		t.Errorf("union is %q, want 0,1,2,5", updates[0].GetLinux().GetResources().GetCpu().GetCpus())
	}
	if got := p.m.Counter("stale_env"); got != 1 {
		t.Errorf("stale_env = %d, want 1", got)
	}
}

func TestSynchronizeRewidensAndRebuildsLoad(t *testing.T) {
	p := testPlugin(t, "0,4")
	narrow := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	narrow.State = api.ContainerState_CONTAINER_RUNNING
	other := ctr("ctr-2", "2", nil, nil)
	other.State = api.ContainerState_CONTAINER_RUNNING

	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{narrow, other})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 1 {
		t.Fatalf("got %d updates, want 1", len(updates))
	}
	if !sameCPUs(t, updates[0].GetLinux().GetResources().GetCpu().GetCpus(), "0,1,5") {
		t.Errorf("union is %q, want 0,1,5", updates[0].GetLinux().GetResources().GetCpu().GetCpus())
	}
	load, held := p.Table()
	if held != 1 {
		t.Errorf("held = %d, want 1", held)
	}
	if load[0] != 250 {
		t.Errorf("load on CPU 0 is %d, want 250", load[0])
	}
	if got := p.m.Counter("rewidened/sync"); got != 1 {
		t.Errorf("rewidened/sync = %d, want 1", got)
	}
}

func TestSynchronizeUnhandled(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates, want 0", len(updates))
	}
	if got := p.m.Counter("unhandled"); got != 1 {
		t.Errorf("unhandled = %d, want 1", got)
	}
}

func TestFullSetMillicpuGrant(t *testing.T) {
	one := 1
	p := testPlugin(t, "0,4", func(f *config.File) { f.Oversubscription = &one })
	c := ctr("ctr-full", "1,5", nil, map[string]string{names.MilliAnnotation: "2000"})
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil {
		t.Fatal(err)
	}
	got := adj.GetLinux().GetResources().GetCpu().GetCpus()
	if !sameCPUs(t, got, "0,1,4,5") {
		t.Fatalf("cpuset %q, want the exclusive CPUs plus the whole helper set 0,4", got)
	}
	env := map[string]string{}
	for _, e := range adj.GetEnv() {
		env[e.Key] = e.Value
	}
	if env[names.EnvShared] != "0,4" {
		t.Fatalf("%s = %q, want 0,4 (M = 1000 * |S| grants every helper CPU)", names.EnvShared, env[names.EnvShared])
	}
	if adj.GetLinux().GetResources().GetCpu().GetQuota() != nil {
		t.Fatal("create wrote a CPU quota; the plugin must not clear or set one")
	}
	t.Log("full-set millicpu grant")
}

func TestReconnectKeepsRecordedHelperCPUs(t *testing.T) {
	p := testPlugin(t, "0,4")
	recorded := ctr("ctr-old", "1,5,4", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "4",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	recorded.State = api.ContainerState_CONTAINER_RUNNING
	if _, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{recorded}); err != nil {
		t.Fatal(err)
	}
	freshPod := pod()
	freshPod.Id = "sandbox-2"
	freshPod.Uid = "uid-2"
	fresh := ctr("ctr-new", "2,6", nil, map[string]string{names.MilliAnnotation: "250"})
	fresh.PodSandboxId = freshPod.Id
	adj, _, err := p.CreateContainer(context.Background(), freshPod, fresh)
	if err != nil {
		t.Fatal(err)
	}
	env := map[string]string{}
	for _, e := range adj.GetEnv() {
		env[e.Key] = e.Value
	}
	if env[names.EnvShared] != "0" {
		t.Fatalf("new container got shared %q, want the least-loaded CPU 0, not the recorded helper 4", env[names.EnvShared])
	}
	load, _ := p.Table()
	if load[4] != 250 {
		t.Fatalf("recorded helper CPU 4 has load %d, want 250 (reconnect must not replace it)", load[4])
	}
	t.Log("reconnect keeps recorded helper CPUs")
}

func TestProfileChangeKeepsRunningGrant(t *testing.T) {
	data := []byte(`
nodeProfileLabel: ergon.io/node-profile
profiles:
  wide:
    sharedCpus: "0,4"
    namespaces: [trading]
  moved:
    sharedCpus: "0"
    namespaces: [trading]
`)
	moved, err := config.Resolve(data, "moved")
	if err != nil {
		t.Fatal(err)
	}
	m := metrics.New()
	p := New(moved, testSiblings(), slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError})), m)
	recorded := ctr("ctr-old", "1,5,4", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "4",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	recorded.State = api.ContainerState_CONTAINER_RUNNING
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{recorded})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("profile change rewrote a running cpuset: %+v", updates)
	}
	load, _ := p.Table()
	if load[4] != 250 {
		t.Fatalf("recorded helper CPU 4 has load %d after the profile changed", load[4])
	}
	freshPod := pod()
	freshPod.Id = "sandbox-2"
	freshPod.Uid = "uid-2"
	fresh := ctr("ctr-new", "2,6", nil, map[string]string{names.MilliAnnotation: "250"})
	fresh.PodSandboxId = freshPod.Id
	adj, _, err := p.CreateContainer(context.Background(), freshPod, fresh)
	if err != nil {
		t.Fatal(err)
	}
	env := map[string]string{}
	for _, e := range adj.GetEnv() {
		env[e.Key] = e.Value
	}
	if env[names.EnvShared] != "0" {
		t.Fatalf("new container shared %q, want the new profile's CPU 0", env[names.EnvShared])
	}
	t.Log("profile change does not alter grants of already-running containers")
}

func TestUpdatePreservesQuota(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "4",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING
	res := c.GetLinux().GetResources()
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 1 {
		t.Fatalf("got %d updates, want the shared CPU added back", len(updates))
	}
	cpu := updates[0].GetLinux().GetResources().GetCpu()
	if cpu.GetQuota() != nil || cpu.GetShares() != nil {
		t.Fatalf("update wrote quota/shares (%v / %v); the plugin must not clear a quota", cpu.GetQuota(), cpu.GetShares())
	}
	if !sameCPUs(t, cpu.GetCpus(), "1,4,5") {
		t.Fatalf("cpuset %q, want 1,4,5", cpu.GetCpus())
	}
	t.Log("quota-preserving update")
}

func TestSynchronizeKeepsOwnShared(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5,4", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "4",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates for a container that already has its shared CPU, want 0", len(updates))
	}
	load, _ := p.Table()
	if load[4] != 250 {
		t.Errorf("load on CPU 4 is %d, want 250", load[4])
	}
}

func TestSynchronizeResend(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING

	resent := make(chan []*api.ContainerUpdate, 1)
	p.SetResender(func(updates []*api.ContainerUpdate) ([]*api.ContainerUpdate, error) {
		resent <- updates
		return nil, nil
	})
	if _, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c}); err != nil {
		t.Fatal(err)
	}
	select {
	case got := <-resent:
		if len(got) != 1 {
			t.Fatalf("resent %d updates, want 1", len(got))
		}
		if cpus := got[0].GetLinux().GetResources().GetCpu().GetCpus(); !sameCPUs(t, cpus, "0,1,5") {
			t.Errorf("resent cpuset is %q, want 0,1,5", cpus)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("the 2 s resend never ran")
	}
}

func TestStopAndRemoveReleaseLoad(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	if _, _, err := p.CreateContainer(context.Background(), pod(), c); err != nil {
		t.Fatal(err)
	}
	if _, held := p.Table(); held != 1 {
		t.Fatalf("held = %d after create, want 1", held)
	}
	if _, err := p.StopContainer(context.Background(), pod(), c); err != nil {
		t.Fatal(err)
	}
	if _, held := p.Table(); held != 0 {
		t.Fatalf("held = %d after stop, want 0", held)
	}
}

func TestRemovePodSandboxForgetsSticky(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	if _, _, err := p.CreateContainer(context.Background(), pod(), c); err != nil {
		t.Fatal(err)
	}
	// Release the load but keep the sticky mapping.
	if _, err := p.StopContainer(context.Background(), pod(), c); err != nil {
		t.Fatal(err)
	}
	// Same pod + container name: sticky would give the same CPUs back.
	c2 := ctr("ctr-2", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	if _, _, err := p.CreateContainer(context.Background(), pod(), c2); err != nil {
		t.Fatal(err)
	}
	load, _ := p.Table()
	if load[0] != 250 {
		t.Errorf("load on CPU 0 is %d after a sticky restart, want 250", load[0])
	}
	if _, err := p.StopContainer(context.Background(), pod(), c2); err != nil {
		t.Fatal(err)
	}
	if err := p.RemovePodSandbox(context.Background(), pod()); err != nil {
		t.Fatal(err)
	}
	// After the pod is gone the sticky mapping is too: the next create may
	// land on the other shared CPU (the load is equal, so the lower CPU wins).
	c3 := ctr("ctr-3", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	if _, _, err := p.CreateContainer(context.Background(), pod(), c3); err != nil {
		t.Fatal(err)
	}
}

func TestBalanceOverSharedCPUs(t *testing.T) {
	// sharedCpus "0,1", the README example: A (250) -> 0, B (250) -> 1,
	// C (250) -> 0 (tie 250/250, lower CPU number wins). Every exclusive
	// container takes a whole physical core, because a core left half
	// unmanaged is refused: under n/n+8 SMT those cores are 2,10 / 3,11 /
	// 4,12, none of them touching the shared cores 0,8 and 1,9. The wider
	// node is needed because the eight-CPU test topology has only two cores
	// to spare beside two shared ones.
	sibs := map[int]cpuset.Set{}
	for c := 0; c < 8; c++ {
		sibs[c] = cpuset.New(c, c+8)
		sibs[c+8] = cpuset.New(c, c+8)
	}
	p := testPluginSiblings(t, "0,1", sibs)
	mk := func(name, uid, cpus string, shares uint64) (*api.PodSandbox, *api.Container) {
		pd := pod()
		pd.Uid, pd.Name = uid, name
		c := ctr(name, cpus, nil, map[string]string{names.MilliAnnotation: "250"})
		c.Name = name
		c.Linux.Resources.Cpu.Shares = &api.OptionalUInt64{Value: shares}
		return pd, c
	}
	pdA, a := mk("a", "ua", "2,10", 2048)
	if _, _, err := p.CreateContainer(context.Background(), pdA, a); err != nil {
		t.Fatal(err)
	}
	pdB, b := mk("b", "ub", "3,11", 2048)
	if _, _, err := p.CreateContainer(context.Background(), pdB, b); err != nil {
		t.Fatal(err)
	}
	pdC, c := mk("c", "uc", "4,12", 2048)
	if _, _, err := p.CreateContainer(context.Background(), pdC, c); err != nil {
		t.Fatal(err)
	}
	load, held := p.Table()
	if held != 3 {
		t.Fatalf("held = %d, want 3", held)
	}
	if load[0] != 500 || load[1] != 250 {
		t.Fatalf("load is %v, want CPU 0 = 500 (A+C) and CPU 1 = 250 (B)", load)
	}
}

func TestConfigureMarksNRIConnected(t *testing.T) {
	// Configure is the first call on a new NRI connection. It must flip the
	// connected gauge (so /readyz can open) and run the on-connect callback
	// (so the device plugin marks its devices healthy again).
	p := testPlugin(t, "0,4")
	connected := false
	p.OnConnected(func() { connected = true })
	if p.m.Ready() {
		t.Fatal("ready before anything happened")
	}
	mask, err := p.Configure(context.Background(), "", "containerd", "v2.3.4")
	if err != nil {
		t.Fatal(err)
	}
	_ = mask
	if !connected {
		t.Fatal("OnConnected did not run")
	}
	p.m.SetRegistered(true)
	if !p.m.Ready() {
		t.Fatal("not ready after Configure and device-plugin registration")
	}
}

// A pod author can set container env by hand. Without our marker that must
// never be treated as ours: otherwise the pod could grow its cpuset onto
// other pods' exclusive CPUs at the next kubelet cpuset write (kubelet
// restart, resize, reconcile).
func TestUpdateHandSetEnvWithoutMarkerIsUntouched(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0,4",
		names.EnvMilli:    "250",
	}, nil) // no marker: not ours
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{Cpus: "1,5"}}
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates for a hand-set env, want 0", len(updates))
	}
}

func TestSynchronizeHandSetEnvWithoutMarkerIsUntouched(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0,4",
		names.EnvMilli:    "250",
	}, nil) // no marker: not ours
	c.State = api.ContainerState_CONTAINER_RUNNING
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates for a hand-set env, want 0", len(updates))
	}
	if _, held := p.Table(); held != 0 {
		t.Fatalf("held = %d for a hand-set env, want 0", held)
	}
}

// The only CPUs this plugin may add to a cpuset are the node's shared
// housekeeping CPUs. A set outside config sharedCpus (forged env, or a
// config change) must never be widened back in.
func TestUpdateForeignSharedIsNotWidened(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "2", // CPU 2 is not in sharedCpus 0,4
	}, map[string]string{names.MilliAnnotation: "250"})
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{Cpus: "1,5"}}
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates that add a foreign CPU, want 0", len(updates))
	}
}

func TestSynchronizeForeignSharedIsNotWidened(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "2", // CPU 2 is not in sharedCpus 0,4
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates that add a foreign CPU, want 0", len(updates))
	}
}

// A marker plus env that does not add up (more shared CPUs than it asked for)
// was not set up by this plugin. It must not be widened.
func TestSynchronizeMarkerEnvMismatchIsUnhandled(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0,4", // 2 CPUs, but the marker asked for 250 (1)
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates, want 0", len(updates))
	}
	if got := p.m.Counter("unhandled"); got != 1 {
		t.Errorf("unhandled = %d, want 1", got)
	}
}

// A pod that opted in by annotation while the plugin was down was created
// without it: no env, or env the pod wrote. It must be counted as unhandled
// (recreate), not silently ignored.
func TestSynchronizeAnnotationMarkerWithoutEnvIsUnhandled(t *testing.T) {
	p := testPlugin(t, "0,4", func(f *config.File) { f.AnnotationOptIn = true })
	podObj := pod()
	podObj.Annotations = map[string]string{names.OptInPrefix + "app": "250"}
	c := ctr("ctr-1", "1,5", nil, nil) // created while the plugin was down
	c.State = api.ContainerState_CONTAINER_RUNNING
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{podObj}, []*api.Container{c})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates, want 0", len(updates))
	}
	if got := p.m.Counter("unhandled"); got != 1 {
		t.Errorf("unhandled = %d, want 1", got)
	}
}

// An update that shows the container is no longer an exclusive integer-CPU
// container (a CPU quota appeared) must not keep the shared set.
func TestUpdateIneligibleQuotaIsNotWidened(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "0,1,4,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
	}, map[string]string{names.MilliAnnotation: "250"})
	q := int64(200000)
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{Cpus: "1,5", Quota: &api.OptionalInt64{Value: q}}}
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates for a container with a CPU quota, want 0", len(updates))
	}
	if got := p.m.Counter("ineligible"); got != 1 {
		t.Errorf("ineligible = %d, want 1", got)
	}
}

// The kubelet's CPU manager writes a cpuset with only cpuset_cpus set. The
// other CPU fields arrive as Go zero values (shares 0, quota 0), not as nil.
// That is "unchanged", not "shares were set to zero": the container is still
// an exclusive integer-CPU container and must keep (and get back) its shared
// CPUs. Regression: the shares test fired on 0 and dropped the shared CPUs on
// every kubelet reconcile.
func TestUpdateKubeletCpusetOnlyIsNotIneligible(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "0,1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
	}, map[string]string{names.MilliAnnotation: "250"})
	// Exactly what a kubelet cpuset-only update looks like in NRI: the cpuset
	// is the exclusive CPUs, shares and quota are zero values.
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{
		Cpus:   "1,5",
		Quota:  &api.OptionalInt64{Value: 0},
		Shares: &api.OptionalUInt64{Value: 0},
	}}
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 1 {
		t.Fatalf("got %d updates, want 1 (re-widen to 0,1,5)", len(updates))
	}
	if cpus := updates[0].GetLinux().GetResources().GetCpu().GetCpus(); !sameCPUs(t, cpus, "0,1,5") {
		t.Errorf("cpuset is %q, want 0,1,5", cpus)
	}
	if got := p.m.Counter("ineligible"); got != 0 {
		t.Errorf("ineligible = %d for a kubelet cpuset-only update, want 0", got)
	}
	if got := p.m.Counter("rewidened/update"); got != 1 {
		t.Errorf("rewidened/update = %d, want 1", got)
	}
}

// A live container that is no longer an exclusive integer-CPU container (a
// CPU quota appeared) must lose its shared CPUs at sync, not keep them.
func TestSynchronizeIneligibleQuotaDropsShared(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "0,1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING
	c.Linux.Resources.Cpu.Quota = &api.OptionalInt64{Value: 200000}
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 1 {
		t.Fatalf("got %d updates, want 1 (shrink to the exclusive CPUs)", len(updates))
	}
	if cpus := updates[0].GetLinux().GetResources().GetCpu().GetCpus(); !sameCPUs(t, cpus, "1,5") {
		t.Errorf("cpuset is %q, want 1,5 (the shared CPU dropped)", cpus)
	}
	if got := p.m.Counter("ineligible"); got != 1 {
		t.Errorf("ineligible = %d, want 1", got)
	}
	if _, held := p.Table(); held != 0 {
		t.Errorf("held = %d for an ineligible container, want 0", held)
	}
}

// The same for cpu shares that no longer match the exclusive cpuset.
func TestSynchronizeIneligibleSharesDropsShared(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "0,1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING
	c.Linux.Resources.Cpu.Shares = &api.OptionalUInt64{Value: 100} // not 2 x 1024
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 1 {
		t.Fatalf("got %d updates, want 1 (shrink to the exclusive CPUs)", len(updates))
	}
	if cpus := updates[0].GetLinux().GetResources().GetCpu().GetCpus(); !sameCPUs(t, cpus, "1,5") {
		t.Errorf("cpuset is %q, want 1,5 (the shared CPU dropped)", cpus)
	}
	if got := p.m.Counter("ineligible"); got != 1 {
		t.Errorf("ineligible = %d, want 1", got)
	}
}

// A marker in a namespace outside the allowlist grants nothing: the plugin
// must not touch that container at update time.
func TestUpdateNamespaceNotAllowedIsUntouched(t *testing.T) {
	p := testPlugin(t, "0,4")
	podObj := pod()
	podObj.Namespace = "kube-system"
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{Cpus: "1,5"}}
	updates, err := p.UpdateContainer(context.Background(), podObj, c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates for a namespace outside the allowlist, want 0", len(updates))
	}
}

func TestSynchronizeNamespaceNotAllowedIsUntouched(t *testing.T) {
	p := testPlugin(t, "0,4", func(f *config.File) { f.AnnotationOptIn = true })
	podObj := pod()
	podObj.Namespace = "kube-system"
	podObj.Annotations = map[string]string{names.OptInPrefix + "app": "250"}
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, nil)
	c.State = api.ContainerState_CONTAINER_RUNNING
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{podObj}, []*api.Container{c})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates for a namespace outside the allowlist, want 0", len(updates))
	}
	if _, held := p.Table(); held != 0 {
		t.Fatalf("held = %d for a namespace outside the allowlist, want 0", held)
	}
}

// The pod annotation fallback is an opt-in, but only when annotationOptIn is
// on. With it off the annotation must not make a container ours.
func TestSynchronizeAnnotationIgnoredWhenOptInOff(t *testing.T) {
	p := testPlugin(t, "0,4") // annotationOptIn off
	podObj := pod()
	podObj.Annotations = map[string]string{names.OptInPrefix + "app": "250"}
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, nil)
	c.State = api.ContainerState_CONTAINER_RUNNING
	updates, err := p.Synchronize(context.Background(), []*api.PodSandbox{podObj}, []*api.Container{c})
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 0 {
		t.Fatalf("got %d updates with annotationOptIn off, want 0", len(updates))
	}
	if _, held := p.Table(); held != 0 {
		t.Fatalf("held = %d with annotationOptIn off, want 0", held)
	}
}

// sameCPUs reports whether a kernel cpulist holds exactly the CPUs of want.
func sameCPUs(t *testing.T, got, want string) bool {
	t.Helper()
	g, err := cpuset.Parse(got)
	if err != nil {
		t.Fatalf("parse %q: %v", got, err)
	}
	w, err := cpuset.Parse(want)
	if err != nil {
		t.Fatalf("parse %q: %v", want, err)
	}
	return g.Equals(w)
}

// A seccomp policy that goes in through requestSeccomp and comes back out
// through toAPISeccomp must lose nothing: an argument filter or a per-rule
// errno that is dropped turns a restricted syscall into an unconditional one.
func TestSeccompRoundTripKeepsEveryField(t *testing.T) {
	e := func(v uint32) *api.OptionalUInt32 { return &api.OptionalUInt32{Value: v} }
	in := &api.LinuxSeccomp{
		DefaultAction:    "SCMP_ACT_ERRNO",
		DefaultErrno:     e(1),
		Architectures:    []string{"SCMP_ARCH_X86_64", "SCMP_ARCH_AARCH64"},
		Flags:            []string{"SECCOMP_FILTER_FLAG_TSYNC"},
		ListenerPath:     "/run/nri/seccomp.bpf",
		ListenerMetadata: "meta",
		Syscalls: []*api.LinuxSyscall{
			{
				Names:    []string{"clone"},
				Action:   "SCMP_ACT_ERRNO",
				ErrnoRet: e(38),
				Args: []*api.LinuxSeccompArg{
					{Index: 0, Value: 2147483648, Op: "SCMP_CMP_MASKED_EQ", ValueTwo: 2147483648},
					{Index: 1, Value: 3, Op: "SCMP_CMP_EQ"},
				},
			},
			{Names: []string{"read", "write"}, Action: "SCMP_ACT_ALLOW"},
		},
	}
	container := &api.Container{Linux: &api.LinuxContainer{SeccompPolicy: in}}
	got := toAPISeccomp(requestSeccomp(container))
	if got.GetDefaultAction() != in.DefaultAction {
		t.Errorf("default action %q, want %q", got.GetDefaultAction(), in.DefaultAction)
	}
	if got.GetDefaultErrno().GetValue() != in.DefaultErrno.GetValue() {
		t.Errorf("default errno %v, want %v", got.GetDefaultErrno(), in.DefaultErrno)
	}
	if strings.Join(got.GetArchitectures(), ",") != strings.Join(in.Architectures, ",") {
		t.Errorf("architectures %v, want %v", got.GetArchitectures(), in.Architectures)
	}
	if strings.Join(got.GetFlags(), ",") != strings.Join(in.Flags, ",") {
		t.Errorf("flags %v, want %v", got.GetFlags(), in.Flags)
	}
	if got.GetListenerPath() != in.ListenerPath {
		t.Errorf("listener path %q, want %q", got.GetListenerPath(), in.ListenerPath)
	}
	if got.GetListenerMetadata() != in.ListenerMetadata {
		t.Errorf("listener metadata %q, want %q", got.GetListenerMetadata(), in.ListenerMetadata)
	}
	if len(got.GetSyscalls()) != len(in.Syscalls) {
		t.Fatalf("syscalls %d, want %d", len(got.GetSyscalls()), len(in.Syscalls))
	}
	for i, want := range in.Syscalls {
		have := got.GetSyscalls()[i]
		if strings.Join(have.GetNames(), ",") != strings.Join(want.Names, ",") {
			t.Errorf("syscall %d names %v, want %v", i, have.GetNames(), want.Names)
		}
		if have.GetAction() != want.Action {
			t.Errorf("syscall %d action %q, want %q", i, have.GetAction(), want.Action)
		}
		if (have.GetErrnoRet() == nil) != (want.ErrnoRet == nil) {
			t.Fatalf("syscall %d errno presence %v, want %v", i, have.GetErrnoRet(), want.ErrnoRet)
		}
		if want.ErrnoRet != nil && have.GetErrnoRet().GetValue() != want.ErrnoRet.GetValue() {
			t.Errorf("syscall %d errno %v, want %v", i, have.GetErrnoRet(), want.ErrnoRet)
		}
		if len(have.GetArgs()) != len(want.Args) {
			t.Fatalf("syscall %d args %d, want %d", i, len(have.GetArgs()), len(want.Args))
		}
		for j, a := range want.Args {
			b := have.GetArgs()[j]
			if b.GetIndex() != a.Index || b.GetValue() != a.Value ||
				b.GetValueTwo() != a.ValueTwo || b.GetOp() != a.Op {
				t.Errorf("syscall %d arg %d = %+v, want %+v", i, j, b, a)
			}
		}
	}
}

// The io-uring widening appends one rule and keeps every restriction the
// runtime put on the rest of the policy.
func TestSeccompIOUringKeepsEveryRestriction(t *testing.T) {
	e := func(v uint32) *uint32 { return &v }
	in := &latency.SeccompPolicy{
		DefaultAction:    "SCMP_ACT_ERRNO",
		DefaultErrno:     e(1),
		Architectures:    []string{"SCMP_ARCH_X86_64"},
		Flags:            []string{"SECCOMP_FILTER_FLAG_TSYNC"},
		ListenerPath:     "/run/nri/seccomp.bpf",
		ListenerMetadata: "meta",
		Syscalls: []latency.SeccompSyscall{{
			Names:    []string{"clone"},
			Action:   "SCMP_ACT_ERRNO",
			ErrnoRet: e(38),
			Args:     []latency.SeccompArg{{Index: 0, Value: 1, Op: "SCMP_CMP_EQ"}},
		}},
	}
	out, err := latency.WithIOUring(in, latency.ModeIOUring)
	if err != nil {
		t.Fatal(err)
	}
	if out.DefaultErrno == nil || *out.DefaultErrno != 1 {
		t.Errorf("default errno %v, want 1", out.DefaultErrno)
	}
	if len(out.Flags) != 1 || out.Flags[0] != "SECCOMP_FILTER_FLAG_TSYNC" {
		t.Errorf("flags %v, want the request's", out.Flags)
	}
	if out.ListenerPath != in.ListenerPath || out.ListenerMetadata != in.ListenerMetadata {
		t.Errorf("listener %q/%q, want %q/%q",
			out.ListenerPath, out.ListenerMetadata, in.ListenerPath, in.ListenerMetadata)
	}
	var clone *latency.SeccompSyscall
	for i := range out.Syscalls {
		if len(out.Syscalls[i].Names) > 0 && out.Syscalls[i].Names[0] == "clone" {
			clone = &out.Syscalls[i]
		}
	}
	if clone == nil {
		t.Fatal("the clone rule must still be there")
	}
	if clone.ErrnoRet == nil || *clone.ErrnoRet != 38 {
		t.Errorf("clone errno %v, want 38", clone.ErrnoRet)
	}
	if len(clone.Args) != 1 || clone.Args[0] != (latency.SeccompArg{Index: 0, Value: 1, Op: "SCMP_CMP_EQ"}) {
		t.Errorf("clone args %v, want the request's filter", clone.Args)
	}
	// The input is untouched, including its slices.
	if in.Syscalls[0].ErrnoRet == nil || *in.Syscalls[0].ErrnoRet != 38 {
		t.Error("the request's rule was mutated")
	}
	if len(in.Syscalls) != 1 {
		t.Errorf("the request's rules were mutated: %v", in.Syscalls)
	}
}

// --- defect 3: UpdateContainer reconciles partition/reservation; Synchronize
// does not replay a stale widen ---

// A container that loses its exclusive integer-CPU status gives up the shared
// CPUs. It must also give up the reservation: the millicpus it held are shared
// capacity, and a later pod is refused capacity it should get while a dead
// reservation still counts. Both shapes of update matter: one where the
// kubelet writes the cpuset (then there is nothing to send back) and one where
// it does not (then the plugin shrinks the cpuset itself).
func TestUpdateIneligibleReleasesTheReservation(t *testing.T) {
	for _, tc := range []struct {
		name string
		res  *api.LinuxResources
		want int
	}{
		{
			name: "kubelet writes the cpuset",
			res: &api.LinuxResources{Cpu: &api.LinuxCPU{
				Cpus:   "1,5",
				Quota:  &api.OptionalInt64{Value: 100000},
				Shares: &api.OptionalUInt64{Value: 2048},
			}},
			want: 0,
		},
		{
			name: "kubelet does not write the cpuset",
			res: &api.LinuxResources{Cpu: &api.LinuxCPU{
				Quota:  &api.OptionalInt64{Value: 100000},
				Shares: &api.OptionalUInt64{Value: 2048},
			}},
			want: 1,
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			p := testPlugin(t, "0,4")
			c := ctr("ctr-1", "1,5", map[string]string{
				names.EnvIsolated: "1,5",
				names.EnvShared:   "0",
				names.EnvMilli:    "250",
			}, map[string]string{names.MilliAnnotation: "250"})
			adj, _, err := p.CreateContainer(context.Background(), pod(), c)
			if err != nil {
				t.Fatal(err)
			}
			applyAdjustment(t, c, adj)
			if _, held := p.Table(); held != 1 {
				t.Fatalf("held after create = %d, want 1", held)
			}
			updates, err := p.UpdateContainer(context.Background(), pod(), c, tc.res)
			if err != nil {
				t.Fatal(err)
			}
			if len(updates) != tc.want {
				t.Fatalf("got %d updates, want %d", len(updates), tc.want)
			}
			for _, u := range updates {
				if !sameCPUs(t, u.GetLinux().GetResources().GetCpu().GetCpus(), "1,5") {
					t.Errorf("cpuset is %q, want 1,5", u.GetLinux().GetResources().GetCpu().GetCpus())
				}
			}
			if _, held := p.Table(); held != 0 {
				t.Errorf("held after the shared CPUs were dropped = %d, want 0", held)
			}
			if got := p.m.Counter("ineligible"); got != 1 {
				t.Errorf("ineligible = %d, want 1", got)
			}
		})
	}
}

// A managed update that keeps the container eligible must leave the
// reservation matching what the container holds: same shared CPU, same
// millicpus. It must not be dropped, and it must not be left stale.
func TestUpdateRewidenKeepsTheReservation(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil {
		t.Fatal(err)
	}
	applyAdjustment(t, c, adj)
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{Cpus: "1,5"}}
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 1 {
		t.Fatalf("got %d updates, want 1", len(updates))
	}
	load, held := p.Table()
	if held != 1 {
		t.Fatalf("held after the update = %d, want 1", held)
	}
	if load[0] != 250 {
		t.Errorf("load on CPU 0 is %d, want 250", load[0])
	}
}

// The cpu-partition's createRuntime and poststop hooks carry the exclusive
// set as it was at create. A resize makes them wrong: the poststop would
// release CPUs the container no longer owns and miss the ones it does. NRI
// cannot re-run a createRuntime hook, so the plugin must report it and the
// container has to be recreated. Silently leaving it is the bug.
func TestUpdateResizeReportsStalePartition(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := withCgroupsPath(ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, map[string]string{
		names.MilliAnnotation:            "250",
		names.CpuPartitionPrefix + "app": names.CpuPartitionValue,
	}), "kubepods/pod-1/ctr-1")
	adj, _, err := p.CreateContainer(context.Background(), pod(), c)
	if err != nil {
		t.Fatal(err)
	}
	applyAdjustment(t, c, adj)
	// The kubelet resizes the exclusive set: 2 joins the core pair 1,5.
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{Cpus: "1,2,5"}}
	updates, err := p.UpdateContainer(context.Background(), pod(), c, res)
	if err != nil {
		t.Fatal(err)
	}
	if len(updates) != 1 {
		t.Fatalf("got %d updates, want 1", len(updates))
	}
	if got := p.m.Counter("stale_partition"); got != 1 {
		t.Errorf("stale_partition = %d, want 1 (the create-time hooks still name 1,5)", got)
	}
}

// Synchronize sends its re-widen updates again after ResendDelay because
// containerd only logs a failed one. The copy is taken when Synchronize runs.
// If the container moves on before the resend fires — a resize, a stop — the
// copy is stale and must not be replayed: replaying it would put the container
// on CPUs it no longer owns.
func TestSynchronizeResendIsNotReplayedAfterUpdate(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING

	var pending []func()
	p.After = func(_ time.Duration, fn func()) { pending = append(pending, fn) }
	resent := make(chan []*api.ContainerUpdate, 1)
	p.SetResender(func(updates []*api.ContainerUpdate) ([]*api.ContainerUpdate, error) {
		resent <- updates
		return nil, nil
	})
	if _, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c}); err != nil {
		t.Fatal(err)
	}
	if len(pending) != 1 {
		t.Fatalf("Synchronize queued %d resends, want 1", len(pending))
	}
	// The container moves on before the resend: the kubelet resizes it and the
	// plugin widens it again from the new state.
	res := &api.LinuxResources{Cpu: &api.LinuxCPU{Cpus: "1,2,5"}}
	if _, err := p.UpdateContainer(context.Background(), pod(), c, res); err != nil {
		t.Fatal(err)
	}
	pending[0]() // the delayed resend fires
	select {
	case got := <-resent:
		t.Fatalf("the stale widen was replayed after the container moved on: %v",
			got[0].GetLinux().GetResources().GetCpu().GetCpus())
	default:
	}
}

// The same rule when the container is gone: a resend naming a stopped
// container is noise at best.
func TestSynchronizeResendIsNotReplayedAfterStop(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING

	var pending []func()
	p.After = func(_ time.Duration, fn func()) { pending = append(pending, fn) }
	resent := make(chan []*api.ContainerUpdate, 1)
	p.SetResender(func(updates []*api.ContainerUpdate) ([]*api.ContainerUpdate, error) {
		resent <- updates
		return nil, nil
	})
	if _, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c}); err != nil {
		t.Fatal(err)
	}
	if len(pending) != 1 {
		t.Fatalf("Synchronize queued %d resends, want 1", len(pending))
	}
	if _, err := p.StopContainer(context.Background(), pod(), c); err != nil {
		t.Fatal(err)
	}
	pending[0]()
	select {
	case got := <-resent:
		t.Fatalf("the widen was replayed for a stopped container: %v",
			got[0].GetLinux().GetResources().GetCpu().GetCpus())
	default:
	}
}

// resendUpdates used to publish the touched map header under the lock and
// index it after Unlock. StopContainer writes that same map. The race
// detector reports it when a delayed resend overlaps a stop. Run this test
// with -race: without the detector the bad read often still returns.
func TestResendUpdatesDoesNotRaceStopContainer(t *testing.T) {
	p := testPlugin(t, "0,4")
	c := ctr("ctr-1", "1,5", map[string]string{
		names.EnvIsolated: "1,5",
		names.EnvShared:   "0",
		names.EnvMilli:    "250",
	}, map[string]string{names.MilliAnnotation: "250"})
	c.State = api.ContainerState_CONTAINER_RUNNING

	var pending []func()
	p.After = func(_ time.Duration, fn func()) { pending = append(pending, fn) }
	p.SetResender(func(updates []*api.ContainerUpdate) ([]*api.ContainerUpdate, error) {
		return nil, nil
	})
	if _, err := p.Synchronize(context.Background(), []*api.PodSandbox{pod()}, []*api.Container{c}); err != nil {
		t.Fatal(err)
	}
	if len(pending) != 1 {
		t.Fatalf("Synchronize queued %d resends, want 1", len(pending))
	}

	const n = 200
	var wg sync.WaitGroup
	errCh := make(chan error, 1)
	wg.Add(2)
	go func() {
		defer wg.Done()
		for i := 0; i < n; i++ {
			pending[0]()
		}
	}()
	go func() {
		defer wg.Done()
		for i := 0; i < n; i++ {
			if _, err := p.StopContainer(context.Background(), pod(), c); err != nil {
				select {
				case errCh <- err:
				default:
				}
				return
			}
		}
	}()
	wg.Wait()
	select {
	case err := <-errCh:
		t.Fatal(err)
	default:
	}
}
