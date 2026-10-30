// SPDX-License-Identifier: Apache-2.0

// Package nriplugin is the NRI side of ergon-shared-cpus: it adds the
// node's shared CPUs to an opted-in container's exclusive cpuset, sets the
// CPU env and the start-affinity hook, and keeps the union when the kubelet
// writes the cpuset again.
//
// Handlers do no I/O besides logging: NRI drops a plugin that takes longer
// than its request timeout (2 s by default) and goes on without its changes.
package nriplugin

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/containerd/nri/pkg/api"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/assign"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/config"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/latency"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/metrics"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

// DefaultResendDelay is how long after Synchronize the re-widen updates are
// sent again: containerd only logs a failed sync update.
const DefaultResendDelay = 2 * time.Second

// Resender sends unsolicited container updates and returns the failed ones
// (the NRI stub's UpdateContainers).
type Resender func([]*api.ContainerUpdate) ([]*api.ContainerUpdate, error)

// Plugin implements the NRI stub's handler interfaces.
type Plugin struct {
	cfg      *config.Config
	siblings map[int]cpuset.Set
	log      *slog.Logger
	metrics  *metrics.Metrics

	// ResendDelay and After are fields so tests can run the resend at once.
	ResendDelay time.Duration
	After       func(time.Duration, func())

	mu        sync.Mutex
	table     *assign.Table
	resend    Resender
	onSynced  func()
	onConnect func()
	// contract is re-read on every hot create. Nil refuses hot creates and
	// leaves every other create alone.
	contract func(shared, exclusive cpuset.Set) error
	// touched holds the containers whose state moved on after a Synchronize.
	// Synchronize sends its re-widen updates again after ResendDelay against a
	// copy taken at sync time; a container that was resized, stopped or removed
	// in between must not have that copy replayed onto it.
	touched map[string]bool
}

// New returns a plugin. siblings maps each CPU to its core's CPUs (read
// from sysfs once, at start).
func New(cfg *config.Config, siblings map[int]cpuset.Set, log *slog.Logger, m *metrics.Metrics) *Plugin {
	return &Plugin{
		cfg:         cfg,
		siblings:    siblings,
		log:         log,
		metrics:     m,
		ResendDelay: DefaultResendDelay,
		After:       func(d time.Duration, f func()) { time.AfterFunc(d, f) },
		table:       assign.New(cfg.Shared),
		touched:     map[string]bool{},
	}
}

// SetResender sets the function used to re-send sync updates; call it for
// each new NRI connection.
func (p *Plugin) SetResender(r Resender) {
	p.mu.Lock()
	p.resend = r
	p.mu.Unlock()
}

// OnSynchronized sets a function called after each Synchronize.
func (p *Plugin) OnSynchronized(f func()) {
	p.mu.Lock()
	p.onSynced = f
	p.mu.Unlock()
}

// SetHostContract installs the hot-create node check. The function is called
// with the configured helper set and the container's exclusive cpuset. A nil
// function refuses hot creates. It does not affect pods without the hot label.
func (p *Plugin) SetHostContract(fn func(shared, exclusive cpuset.Set) error) {
	p.mu.Lock()
	p.contract = fn
	p.mu.Unlock()
}

// OnConnected sets a function called on every new NRI connection (Configure).
// The device plugin uses it to mark its devices healthy again.
func (p *Plugin) OnConnected(f func()) {
	p.mu.Lock()
	p.onConnect = f
	p.mu.Unlock()
}

// Configure logs the runtime, marks NRI connected and subscribes to every
// implemented event.
func (p *Plugin) Configure(_ context.Context, _, runtime, version string) (api.EventMask, error) {
	p.log.Info("nri configure", "runtime", runtime, "runtime_version", version)
	p.metrics.SetNRIConnected(true)
	p.mu.Lock()
	f := p.onConnect
	p.mu.Unlock()
	if f != nil {
		f()
	}
	return 0, nil
}

func envMap(env []string) map[string]string {
	m := make(map[string]string, len(env))
	for _, e := range env {
		k, v, _ := strings.Cut(e, "=")
		m[k] = v
	}
	return m
}

func requestFrom(pod *api.PodSandbox, ctr *api.Container) Request {
	cpu := ctr.GetLinux().GetResources().GetCpu()
	r := Request{
		Namespace:          pod.GetNamespace(),
		PodName:            pod.GetName(),
		PodUID:             pod.GetUid(),
		Container:          ctr.GetName(),
		PodAnnotations:     pod.GetAnnotations(),
		Annotations:        ctr.GetAnnotations(),
		Env:                envMap(ctr.GetEnv()),
		Cpus:               cpu.GetCpus(),
		SeccompPolicy:      requestSeccomp(ctr),
		HugepageLimitBytes: requestHugepageLimitBytes(ctr),
		Hot:                pod.GetLabels()[names.LatencyLabel] == names.LatencyHot,
	}
	if q := cpu.GetQuota(); q != nil {
		v := q.GetValue()
		r.Quota = &v
	}
	if s := cpu.GetShares(); s != nil {
		v := s.GetValue()
		r.Shares = &v
	}
	return r
}

// hotContract runs the mounted-fact check for one hot container. The plugin
// stays up either way: the caller returns the refusal to this create only.
func (p *Plugin) hotContract(r Request) error {
	p.mu.Lock()
	fn := p.contract
	p.mu.Unlock()
	if fn == nil {
		return errors.New("node contract is not configured")
	}
	exclusive := cpuset.Set{}
	if r.Cpus != "" {
		e, err := cpuset.Parse(r.Cpus)
		if err != nil {
			return fmt.Errorf("exclusive cpuset: %w", err)
		}
		exclusive = e
	}
	return fn(p.cfg.Shared, exclusive)
}

// marker is the opt-in marker of a container: the device plugin's container
// annotation (pod authors cannot set container annotations) or the pod
// opt-in annotation, which only counts when annotationOptIn is on. Env alone
// is never a marker: a pod can set env by hand.
func marker(cfg *config.Config, pod *api.PodSandbox, ctr *api.Container) (value, source string) {
	if v, ok := ctr.GetAnnotations()[names.MilliAnnotation]; ok {
		return v, "resource"
	}
	if cfg != nil && cfg.AnnotationOptIn {
		if v, ok := pod.GetAnnotations()[names.OptInPrefix+ctr.GetName()]; ok {
			return v, "annotation"
		}
	}
	return "", ""
}

// setup is what the plugin knows about a container it set up.
type setup struct {
	shared cpuset.Set
	milli  int
	source string
}

// setupOf returns the container's shared CPUs when this plugin set it up.
// The millicpu request comes from the marker, not from the env, and must
// match the number of shared CPUs in the env this plugin wrote at create.
// A namespace outside the allowlist is never managed: a marker alone grants
// nothing.
//
//	ours=false           no marker: not ours, never touched
//	ours=true, ok=false  the marker is there but the state is not ours
//	                     (created without the plugin, env set by hand, or
//	                     the namespace may no longer use shared CPUs)
//	ours=true, ok=true   managed: s is trusted
func setupOf(cfg *config.Config, pod *api.PodSandbox, ctr *api.Container, env map[string]string) (ours, ok bool, s setup) {
	value, source := marker(cfg, pod, ctr)
	if value == "" {
		return false, false, setup{}
	}
	milli, err := parseMilli(value)
	if err != nil {
		return true, false, setup{}
	}
	if !cfg.NamespaceAllowed(pod.GetNamespace()) {
		return true, false, setup{}
	}
	sharedText, hasShared := env[names.EnvShared]
	if !hasShared {
		return true, false, setup{}
	}
	shared, err := cpuset.Parse(sharedText)
	if err != nil || shared.Len() != assign.Need(milli) {
		return true, false, setup{}
	}
	return true, true, setup{shared: shared, milli: milli, source: source}
}

// ineligible reports whether the CPU resources show a container that is no
// longer an exclusive integer-CPU container (a CPU quota appeared, or the cpu
// shares do not match the cpuset). Only exclusive integer-CPU containers may
// hold shared CPUs. A nil field means "unchanged"; so does a zero value,
// because the kubelet's cpuset-only update carries shares and quota as Go
// zero values rather than as absent. Linux cpu.shares is never 0 (minimum 2).
// The shares test needs a cpuset to compare against.
func ineligible(cpus string, q *api.OptionalInt64, shares *api.OptionalUInt64) bool {
	if q != nil && q.GetValue() > 0 {
		return true
	}
	if shares == nil || shares.GetValue() == 0 || cpus == "" {
		return false
	}
	n, err := cpuset.Parse(cpus)
	if err != nil {
		return true
	}
	return shares.GetValue() != uint64(n.Len())*1024
}

func (p *Plugin) publishTable() {
	p.metrics.SetTable(p.table.Len(), p.table.Load())
}

// CreateContainer widens an opted-in container's cpuset, sets its env and
// injects the start-affinity hook. A container that breaks a rule is not
// created.
func (p *Plugin) CreateContainer(_ context.Context, pod *api.PodSandbox, ctr *api.Container) (*api.ContainerAdjustment, []*api.ContainerUpdate, error) {
	r := requestFrom(pod, ctr)
	if pod.GetLabels()[names.LatencyLabel] == names.LatencyHot {
		if err := p.hotContract(r); err != nil {
			p.metrics.SetContractOK(false)
			p.metrics.Refused("node-contract")
			p.log.Warn("create refused", "pod", r.Namespace+"/"+r.PodName, "container", r.Container,
				"reason", "node-contract", "error", err.Error())
			return nil, nil, refuse(9, "node-contract", "%v", err)
		}
		p.metrics.SetContractOK(true)
	}
	// Kubernetes writes the hugepages-2Mi limit on the pod cgroup and never
	// on the OCI/NRI container, so read it from there when NRI has none.
	if r.HugepageLimitBytes == 0 {
		r.HugepageLimitBytes = hugepageLimitFromCgroup(p.cfg.CgroupRoot,
			pod.GetLinux().GetCgroupsPath(), ctr.GetLinux().GetCgroupsPath())
	}
	d, err := Admit(p.cfg, r)
	logArgs := []any{"pod", r.Namespace + "/" + r.PodName, "container", r.Container,
		"huge_limit", r.HugepageLimitBytes}
	// What the runtime actually sent for hugepages. A zero here with a
	// hugepages.ergon.io annotation means the limit never reached NRI.
	if len(ctr.GetLinux().GetResources().GetHugepageLimits()) > 0 {
		logArgs = append(logArgs, "huge_raw", fmt.Sprintf("%v", ctr.GetLinux().GetResources().GetHugepageLimits()))
	}
	if err != nil {
		var ref *Refusal
		reason := "other"
		if errors.As(err, &ref) {
			reason = ref.Reason
		}
		p.metrics.Refused(reason)
		p.log.Warn("create refused", append(logArgs, "reason", reason, "error", err.Error())...)
		return nil, nil, err
	}
	if d == nil {
		return nil, nil, nil
	}
	// The exclusive CPUs have to be able to keep their own physical cores.
	// On SMT a core split between exclusive and shared runs the hot thread and
	// the helpers on the same execution units, which is the contention a
	// partition exists to remove. This is checked here rather than in Admit:
	// Admit is the policy over the request, and this is the request against
	// the node's topology. A container with no exclusive CPUs has nothing to
	// keep, so it is left alone.
	if !d.Exclusive.IsEmpty() {
		if err := cpuset.CheckCoreLayout(d.Exclusive, p.cfg.Shared, p.siblings); err != nil {
			p.metrics.Refused("core-layout")
			p.log.Warn("create refused", append(logArgs, "reason", "core-layout", "error", err.Error())...)
			return nil, nil, refuse(8, "core-layout", "%v", err)
		}
	}
	for _, w := range d.Warnings {
		p.log.Warn("create warning", append(logArgs, "warning", w)...)
	}

	adj := &api.ContainerAdjustment{}
	shared := cpuset.Set{}
	if d.Source != "" {
		// Shared-CPU opt-in: widen the cpuset and set the env.
		p.mu.Lock()
		var err error
		shared, err = p.table.Choose(key(r), d.Need)
		if err != nil {
			p.mu.Unlock()
			p.metrics.Refused("too-many")
			return nil, nil, refuse(6, "too-many", "%v", err)
		}
		p.table.Hold(ctr.GetId(), key(r), shared, d.Milli)
		p.publishTable()
		p.mu.Unlock()

		adj.SetLinuxCPUSetCPUs(d.Exclusive.Union(shared).String())
		for _, kv := range Env(d.Exclusive, shared, cpuset.Cores(d.Exclusive, p.siblings), d.Milli) {
			adj.AddEnv(kv.Key, kv.Value)
		}
	}
	if d.Grants != nil {
		if err := p.applyGrants(adj, d.Grants, d, ctr, logArgs); err != nil {
			p.metrics.Refused("partition")
			return nil, nil, err
		}
	}
	// The hook starts the container on the shared CPUs (d.Hook) and builds the
	// isolated child cgroup (cpu-partition). Either one alone is enough to
	// inject it: a partition is never skipped because the affinity was left
	// alone. A partition also gets a poststop hook: the create's ancestor
	// reservations must come back down when the container goes, or the next
	// one that wants those CPUs is refused.
	needHook := d.Hook || (d.Grants != nil && d.Grants.CpuPartition)
	if needHook {
		var part *latency.PartitionGrant
		if d.Grants != nil {
			part = d.Grants.Partition
		}
		hooks := &api.Hooks{CreateRuntime: []*api.Hook{Hook(p.cfg.HookPath, shared, part)}}
		if part != nil {
			hooks.Poststop = []*api.Hook{ReleaseHook(p.cfg.HookPath, part)}
		}
		adj.AddHooks(hooks)
	}
	p.log.Info("create", append(logArgs, "exclusive", d.Exclusive.String(), "shared", shared.String(),
		"millicpus", d.Milli, "source", d.Source, "hook", needHook)...)
	return adj, nil, nil
}

// applyGrants writes the latency-pod extension set into the container
// adjustment: rlimits, the seccomp widening, the NUMA binding, the PHC and
// PM QoS devices, the sized hugetlbfs mount and the env contract. Every grant
// is opt-in by annotation. A grant that cannot be applied is a typed refusal
// (fail closed): the container is not created.
func (p *Plugin) applyGrants(adj *api.ContainerAdjustment, g *Grants, d *Decision, ctr *api.Container, logArgs []any) error {
	for _, r := range g.Rlimits {
		adj.AddRlimit(r.Type, r.Hard, r.Soft)
		p.metrics.Grant("rlimit", r.Type)
	}
	if g.Seccomp != nil {
		adj.SetLinuxSeccompPolicy(toAPISeccomp(g.Seccomp))
		p.metrics.Grant("seccomp", g.SeccompMode)
	} else if g.SeccompMode != "" {
		p.metrics.Grant("seccomp", g.SeccompMode)
	}
	if g.NumaMems != "" {
		adj.SetLinuxCPUSetMems(g.NumaMems)
		// MPOL_BIND so the memory actually lands on those nodes. The mode
		// constant is in the NRI proto; set it via the string form the API
		// exposes.
		adj.SetLinuxMemoryPolicy(api.MpolMode_MPOL_BIND, g.NumaMems)
		p.metrics.Grant("numa", g.NumaMems)
	}
	if g.PtpDevice != "" {
		major, minor := p.ptpNumbers(g.PtpDevice)
		adj.AddDevice(&api.LinuxDevice{
			Path:     g.PtpDevice,
			Type:     "c",
			Major:    major,
			Minor:    minor,
			FileMode: &api.OptionalFileMode{Value: 0o444},
		})
		p.metrics.Grant("ptp", g.PtpDevice)
	}
	if g.NodeFacts != "" {
		adj.AddMount(&api.Mount{
			Destination: names.NodeFactsMountPoint,
			Source:      names.NodeFactsPath,
			Type:        "bind",
			Options:     []string{"ro", "bind"},
		})
		p.metrics.Grant("node-facts", names.NodeFactsMountPoint)
	}
	if g.CpuqosDevice != "" {
		// The app writes its PM QoS request here, so it needs write access.
		major, minor := p.ptpNumbers(g.CpuqosDevice)
		adj.AddDevice(&api.LinuxDevice{
			Path:     g.CpuqosDevice,
			Type:     "c",
			Major:    major,
			Minor:    minor,
			FileMode: &api.OptionalFileMode{Value: 0o666},
		})
		p.metrics.Grant("cpuqos", g.CpuqosDevice)
	}
	if g.Hugepage != nil {
		// Only the sized hugetlbfs mount. Kubernetes already put the hugetlb
		// cgroup limit on this container from the pod spec's hugepages-2Mi, and
		// re-sending it as an NRI hugepage limit made containerd SEGV on apply.
		adj.AddMount(&api.Mount{
			Destination: g.Hugepage.Destination,
			Type:        g.Hugepage.Type,
			Source:      g.Hugepage.Source,
			Options:     g.Hugepage.Options,
		})
		p.metrics.Grant("hugepages", g.Hugepage.PageSize)
	}
	if g.CpuPartition {
		if d.Exclusive.IsEmpty() {
			return refuse(2, "no-cpuset", "cpu-partition.ergon.io: the container has no exclusive CPUs to isolate")
		}
		child, err := partitionChild(p.cfg.CgroupRoot, ctr.GetLinux().GetCgroupsPath())
		if err != nil {
			return err
		}
		g.Partition = &latency.PartitionGrant{Child: child, Exclusive: d.Exclusive.List()}
		p.metrics.Grant("partition", names.CpuPartitionValue)
	}
	for _, kv := range GrantEnv(g, p.cfg) {
		adj.AddEnv(kv.Key, kv.Value)
	}
	p.log.Info("grants", append(logArgs,
		"rlimits", len(g.Rlimits), "seccomp", g.SeccompMode, "numa", g.NumaMems,
		"ptp", g.PtpDevice, "sched_fifo", g.SchedFifo, "cpuqos", g.CpuqosDevice,
		"hugepages", g.Hugepage != nil, "partition", g.CpuPartition,
		"dpdk_pair", len(g.DpdkEnvs) > 0)...)
	return nil
}

// partitionChild is the absolute path of the isolated child cgroup the hook
// will create under the container's own cgroup. cgroupsPath is the OCI/NRI
// cgroups path: a cgroupfs path, or a systemd slice[:cri-containerd:id] name
// (kind, EKS). The child name is names.CpuPartitionChild, and the same string
// goes into ERGON_ISOLATED_CGROUP.
func partitionChild(cgroupRoot, cgroupsPath string) (string, error) {
	dir, err := cgroupFsPath(cgroupRoot, cgroupsPath)
	if err != nil {
		return "", refuse(0, "bad-cgroup",
			"cpu-partition.ergon.io: the container cgroup %q cannot be placed under %s: %v",
			cgroupsPath, cgroupRoot, err)
	}
	return filepath.Join(dir, names.CpuPartitionChild), nil
}

// toAPISeccomp converts the pure policy to the NRI type. Every field is
// carried across: what went in through requestSeccomp comes back out
// unchanged apart from the one rule the plugin appends.
func toAPISeccomp(s *latency.SeccompPolicy) *api.LinuxSeccomp {
	if s == nil {
		return nil
	}
	out := &api.LinuxSeccomp{
		DefaultAction:    s.DefaultAction,
		Architectures:    append([]string(nil), s.Architectures...),
		Flags:            append([]string(nil), s.Flags...),
		ListenerPath:     s.ListenerPath,
		ListenerMetadata: s.ListenerMetadata,
	}
	if s.DefaultErrno != nil {
		out.DefaultErrno = &api.OptionalUInt32{Value: *s.DefaultErrno}
	}
	for _, sc := range s.Syscalls {
		rule := &api.LinuxSyscall{
			Names:  append([]string(nil), sc.Names...),
			Action: sc.Action,
		}
		if sc.ErrnoRet != nil {
			rule.ErrnoRet = &api.OptionalUInt32{Value: *sc.ErrnoRet}
		}
		for _, a := range sc.Args {
			rule.Args = append(rule.Args, &api.LinuxSeccompArg{
				Index:    a.Index,
				Value:    a.Value,
				ValueTwo: a.ValueTwo,
				Op:       a.Op,
			})
		}
		out.Syscalls = append(out.Syscalls, rule)
	}
	return out
}

// ptpNumbers reads the device numbers of a host device node (the PHC or the PM
// QoS device). The granted path is the container's /dev/... name; the plugin
// stats it under cfg.DevRoot. A missing file gives 0,0: runc still creates the
// node, and the open fails in the pod with a clear errno.
func (p *Plugin) ptpNumbers(path string) (int64, int64) {
	return deviceNumbers(config.HostDevicePath(p.cfg.DevRoot, path))
}

// UpdateContainer adds the shared CPUs back when the kubelet writes a
// cpuset (reconcile after a kubelet restart, or a resize). The union goes
// out in the same write as the kubelet's. Only containers this plugin set
// up (see setupOf) are touched, and only with shared CPUs from sharedCpus.
func (p *Plugin) UpdateContainer(_ context.Context, pod *api.PodSandbox, ctr *api.Container, res *api.LinuxResources) ([]*api.ContainerUpdate, error) {
	env := envMap(ctr.GetEnv())
	ours, ok, s := setupOf(p.cfg, pod, ctr, env)
	if !ours {
		return nil, nil
	}
	logArgs := []any{"pod", pod.GetNamespace() + "/" + pod.GetName(), "container", ctr.GetName()}
	if !ok {
		p.metrics.Unhandled()
		p.log.Warn("update: container was not set up by this plugin (created without it, or env set by hand); recreate it", logArgs...)
		p.markTouched(ctr.GetId())
		return nil, nil
	}
	cpu := res.GetCpu()
	if ineligible(cpu.GetCpus(), cpu.GetQuota(), cpu.GetShares()) {
		p.metrics.Ineligible()
		p.log.Warn("update: container is no longer an exclusive integer-CPU container (quota or shares); the shared CPUs are dropped",
			append(logArgs, "cpus", cpu.GetCpus(), "quota", cpu.GetQuota().GetValue(), "shares", cpu.GetShares().GetValue())...)
		// The reservation goes with the shared CPUs: they are shared
		// capacity, and a later pod must not be refused capacity a dead
		// reservation is still counting.
		p.release(ctr.GetId())
		p.markTouched(ctr.GetId())
		// The kubelet is not writing a cpuset in this update: its write
		// would leave the shared CPUs in. Shrink to the exclusive CPUs.
		if cpu.GetCpus() == "" {
			current, err := cpuset.Parse(ctr.GetLinux().GetResources().GetCpu().GetCpus())
			if err == nil && !current.Difference(s.shared).Equals(current) {
				u := &api.ContainerUpdate{}
				u.SetContainerId(ctr.GetId())
				u.SetLinuxCPUSetCPUs(current.Difference(s.shared).String())
				return []*api.ContainerUpdate{u}, nil
			}
		}
		return nil, nil
	}
	if !s.shared.IsSubsetOf(p.cfg.Shared) {
		p.metrics.Unhandled()
		p.log.Warn("update: the container's shared CPUs are outside sharedCpus (changed since it started?); not re-widening; recreate the container",
			append(logArgs, "container_shared", s.shared.String(), "config_shared", p.cfg.Shared.String())...)
		// Hold what it still has, never a set outside today's sharedCpus.
		p.holdPresent(ctr.GetId(), ctr.GetName(), pod, s,
			ctr.GetLinux().GetResources().GetCpu().GetCpus())
		p.markTouched(ctr.GetId())
		return nil, nil
	}
	cpus := cpu.GetCpus()
	if cpus == "" {
		return nil, nil
	}
	requested, err := cpuset.Parse(cpus)
	if err != nil {
		p.log.Error("update: bad cpuset", append(logArgs, "error", err.Error())...)
		p.markTouched(ctr.GetId())
		return nil, nil
	}
	// From here the cpuset is being decided: any later re-widen from a sync
	// copy is stale.
	p.markTouched(ctr.GetId())
	// The reservation tracks what the container holds now, whatever the
	// cpuset turn out to be.
	p.hold(ctr.GetId(), ctr.GetName(), pod, s)
	if s.shared.IsSubsetOf(requested) {
		return nil, nil
	}
	iso, isoErr := cpuset.Parse(env[names.EnvIsolated])
	if isoErr != nil || !requested.Difference(s.shared).Equals(iso) {
		p.metrics.StaleEnv()
		p.log.Warn("update: exclusive CPUs changed; the container's CPU env is stale (use resizePolicy RestartContainer for cpu)",
			append(logArgs, "env_isolated", env[names.EnvIsolated], "new", requested.String())...)
		// The cpu-partition hooks carry the exclusive set as it was at
		// create. NRI cannot re-run a createRuntime hook, so the poststop
		// would release CPUs the container no longer owns. Say so.
		if env[names.EnvIsolatedCgroup] != "" {
			p.metrics.StalePartition()
			p.log.Warn("update: exclusive CPUs changed; the cpu-partition still isolates the create-time set; recreate the container",
				append(logArgs, "partition", env[names.EnvIsolatedCgroup], "was", env[names.EnvIsolated], "now", requested.Difference(s.shared).String())...)
		}
	}
	union := requested.Union(s.shared)
	u := &api.ContainerUpdate{}
	u.SetContainerId(ctr.GetId())
	u.SetLinuxCPUSetCPUs(union.String())
	p.metrics.Rewidened("update")
	p.log.Info("update", append(logArgs, "requested", requested.String(), "cpuset", union.String())...)
	return []*api.ContainerUpdate{u}, nil
}

// markTouched records that a container's state moved on after the last
// Synchronize, so a delayed re-widen from that sync is not replayed onto it.
func (p *Plugin) markTouched(id string) {
	p.mu.Lock()
	p.touched[id] = true
	p.mu.Unlock()
}

// hold records the container's reservation: its shared CPUs and its millicpu
// request. It is what Synchronize rebuilds, and an update must leave the same
// picture behind. The key matches Synchronize's and create's.
func (p *Plugin) hold(id, name string, pod *api.PodSandbox, s setup) {
	p.mu.Lock()
	p.table.Hold(id, assign.Key{PodUID: pod.GetUid(), Container: name}, s.shared, s.milli)
	p.publishTable()
	p.mu.Unlock()
}

// holdPresent records the shared CPUs the container actually has, the way
// Synchronize does: its recorded set clipped to what is in its cpuset now.
// Used when that recorded set is outside today's sharedCpus — it keeps what it
// holds and is never widened back in, but the load it does carry still counts.
func (p *Plugin) holdPresent(id, name string, pod *api.PodSandbox, s setup, cpus string) {
	current, err := cpuset.Parse(cpus)
	if err != nil {
		// Without a readable cpuset there is nothing to account for; drop
		// the reservation rather than count CPUs the container may not have.
		p.release(id)
		return
	}
	present := s.shared.Intersection(current)
	if present.IsEmpty() {
		p.release(id)
		return
	}
	p.mu.Lock()
	p.table.Hold(id, assign.Key{PodUID: pod.GetUid(), Container: name}, present, s.milli)
	p.publishTable()
	p.mu.Unlock()
}

func live(c *api.Container) bool {
	s := c.GetState()
	return s == api.ContainerState_CONTAINER_CREATED || s == api.ContainerState_CONTAINER_RUNNING
}

// Synchronize rebuilds the load table from the markers of every live
// container and widens any managed container whose cpuset lost its shared
// CPUs (the kubelet wrote it while the plugin was down). Only containers
// this plugin set up (see setupOf) are held or widened. It sends the same
// updates again after ResendDelay, because containerd only logs a failed one.
func (p *Plugin) Synchronize(_ context.Context, pods []*api.PodSandbox, ctrs []*api.Container) ([]*api.ContainerUpdate, error) {
	podByID := make(map[string]*api.PodSandbox, len(pods))
	for _, pod := range pods {
		podByID[pod.GetId()] = pod
	}
	table := assign.New(p.cfg.Shared)
	var updates []*api.ContainerUpdate
	held, unhandled := 0, 0
	for _, c := range ctrs {
		if !live(c) {
			continue
		}
		pod := podByID[c.GetPodSandboxId()]
		logArgs := []any{"pod", pod.GetNamespace() + "/" + pod.GetName(), "container", c.GetName()}
		env := envMap(c.GetEnv())
		ours, ok, s := setupOf(p.cfg, pod, c, env)
		if !ours {
			continue
		}
		if !ok {
			unhandled++
			p.metrics.Unhandled()
			p.log.Warn("sync: container asked for shared CPUs but its state is not ours (created without the plugin, or env set by hand); recreate it", logArgs...)
			continue
		}
		current, err := cpuset.Parse(c.GetLinux().GetResources().GetCpu().GetCpus())
		if err != nil {
			p.log.Error("sync: container has no cpuset", append(logArgs, "error", err.Error())...)
			continue
		}
		cpuNow := c.GetLinux().GetResources().GetCpu()
		// The container may still hold the shared CPUs, so test eligibility
		// against the exclusive part: shares == 1024 * len(exclusive).
		exclusive := current.Difference(s.shared)
		if ineligible(exclusive.String(), cpuNow.GetQuota(), cpuNow.GetShares()) {
			p.metrics.Ineligible()
			p.log.Warn("sync: container is no longer an exclusive integer-CPU container (quota or shares); the shared CPUs are dropped",
				append(logArgs, "quota", cpuNow.GetQuota().GetValue(), "shares", cpuNow.GetShares().GetValue())...)
			if !exclusive.Equals(current) {
				u := &api.ContainerUpdate{}
				u.SetContainerId(c.GetId())
				u.SetLinuxCPUSetCPUs(exclusive.String())
				updates = append(updates, u)
			}
			continue
		}
		// Hold only the shared CPUs the container actually has. A set outside
		// sharedCpus (config changed since it started, or a forged env) is
		// never widened back in.
		present := s.shared.Intersection(current)
		if !s.shared.IsSubsetOf(p.cfg.Shared) {
			if !present.IsEmpty() {
				table.Hold(c.GetId(), assign.Key{PodUID: pod.GetUid(), Container: c.GetName()}, present, s.milli)
				held++
			}
			p.log.Warn("sync: the container's shared CPUs are outside sharedCpus (config changed since it started, or the env is not ours); it keeps what it has and is not re-widened",
				append(logArgs, "container_shared", s.shared.String(), "config_shared", p.cfg.Shared.String())...)
			continue
		}
		if s.shared.IsSubsetOf(current) {
			table.Hold(c.GetId(), assign.Key{PodUID: pod.GetUid(), Container: c.GetName()}, s.shared, s.milli)
			held++
			continue
		}
		union := current.Union(s.shared)
		u := &api.ContainerUpdate{}
		u.SetContainerId(c.GetId())
		u.SetLinuxCPUSetCPUs(union.String())
		updates = append(updates, u)
		table.Hold(c.GetId(), assign.Key{PodUID: pod.GetUid(), Container: c.GetName()}, s.shared, s.milli)
		held++
		p.metrics.Rewidened("sync")
		p.log.Info("sync: re-widen", append(logArgs, "current", current.String(), "cpuset", union.String())...)
	}

	p.mu.Lock()
	p.table = table
	// This sync is the fresh picture; anything that moves on from here is
	// newer than the updates below and must cancel its own resend.
	p.touched = map[string]bool{}
	p.publishTable()
	resend, onSynced := p.resend, p.onSynced
	p.mu.Unlock()

	p.log.Info("sync", "containers", len(ctrs), "held", held, "rewidened", len(updates), "unhandled", unhandled)
	if len(updates) > 0 && resend != nil {
		again := make([]*api.ContainerUpdate, len(updates))
		for i, u := range updates {
			c := &api.ContainerUpdate{}
			c.SetContainerId(u.GetContainerId())
			c.SetLinuxCPUSetCPUs(u.GetLinux().GetResources().GetCpu().GetCpus())
			again[i] = c
		}
		p.After(p.ResendDelay, func() { p.resendUpdates(resend, again) })
	}
	if onSynced != nil {
		onSynced()
	}
	return updates, nil
}

// resendUpdates sends the sync-time re-widen again. The copy is from sync
// time: a container that was resized, stopped or removed since then must not
// have it replayed, because the copy names the CPUs as they were then and
// could put the container on CPUs it no longer owns. containerd only logs a
// failed update, so the ones that are still right are worth the second try.
func (p *Plugin) resendUpdates(resend Resender, updates []*api.ContainerUpdate) {
	live := make([]*api.ContainerUpdate, 0, len(updates))
	// Membership is observed only while p.mu is held. Copying the map header
	// and indexing it after Unlock races with markTouched: StopContainer and
	// RemoveContainer write that map, and a delayed resend runs beside them.
	skip := make([]bool, len(updates))
	p.mu.Lock()
	for i, u := range updates {
		skip[i] = p.touched[u.GetContainerId()]
	}
	p.mu.Unlock()
	for i, u := range updates {
		if skip[i] {
			p.log.Info("sync resend: skipped, the container moved on",
				"container_id", u.GetContainerId(),
				"cpuset", u.GetLinux().GetResources().GetCpu().GetCpus())
			continue
		}
		live = append(live, u)
	}
	if len(live) == 0 {
		p.log.Info("sync resend", "updates", 0, "skipped", len(updates))
		return
	}
	failed, err := resend(live)
	if err != nil {
		p.log.Error("sync resend failed", "error", err.Error(), "updates", len(live))
	}
	for _, f := range failed {
		p.log.Error("sync resend: update failed", "container_id", f.GetContainerId(),
			"cpuset", f.GetLinux().GetResources().GetCpu().GetCpus())
	}
	if err == nil {
		p.log.Info("sync resend", "updates", len(live), "skipped", len(updates)-len(live), "failed", len(failed))
	}
}

// StopContainer drops the container's load.
func (p *Plugin) StopContainer(_ context.Context, _ *api.PodSandbox, ctr *api.Container) ([]*api.ContainerUpdate, error) {
	p.markTouched(ctr.GetId())
	p.release(ctr.GetId())
	return nil, nil
}

// RemoveContainer drops the container's load.
func (p *Plugin) RemoveContainer(_ context.Context, _ *api.PodSandbox, ctr *api.Container) error {
	p.markTouched(ctr.GetId())
	p.release(ctr.GetId())
	return nil
}

func (p *Plugin) release(id string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if p.table.Release(id) {
		p.publishTable()
	}
}

// RemovePodSandbox forgets the shared CPUs remembered for the pod's
// containers.
func (p *Plugin) RemovePodSandbox(_ context.Context, pod *api.PodSandbox) error {
	p.mu.Lock()
	p.table.ForgetPod(pod.GetUid())
	p.mu.Unlock()
	return nil
}

// Table returns a copy of the per-CPU load and the number of containers
// held, for tests and logs.
func (p *Plugin) Table() (map[int]int, int) {
	p.mu.Lock()
	defer p.mu.Unlock()
	return p.table.Load(), p.table.Len()
}
