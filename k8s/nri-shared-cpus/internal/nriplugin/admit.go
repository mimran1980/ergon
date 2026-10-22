// SPDX-License-Identifier: Apache-2.0

package nriplugin

import (
	"fmt"
	"strconv"

	"github.com/containerd/nri/pkg/api"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/assign"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/config"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/latency"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

// MaxMilli is the most millicpus one container may ask for.
const MaxMilli = config.MaxDeviceIDs

// Request is what admission needs from one container create.
type Request struct {
	Namespace      string
	PodName        string
	PodUID         string
	Container      string
	PodAnnotations map[string]string
	Annotations    map[string]string
	Env            map[string]string
	Cpus           string
	Quota          *int64
	Shares         *uint64
	// SeccompPolicy is the container's request policy, or nil. The io_uring
	// widening clones it.
	SeccompPolicy *latency.SeccompPolicy
	// HugepageLimitBytes is the container's hugepages-2Mi limit in bytes
	// (0 when it asks for none). The sized hugetlbfs mount is exactly this
	// size.
	HugepageLimitBytes int64
	// Hot is true when the pod is labelled ergon.io/latency=hot. A hot
	// container cannot turn the start-affinity hook off.
	Hot bool
}

// Refusal is why a container may not be created. The kubelet shows Error()
// in the pod's events as CreateContainerError.
type Refusal struct {
	// Rule is the admission rule number in the README (0: malformed input).
	Rule int
	// Reason is the metric label.
	Reason string
	// Msg is the text for the pod's events.
	Msg string
}

func (r *Refusal) Error() string { return names.PluginName + ": " + r.Msg }

func refuse(rule int, reason, format string, args ...any) *Refusal {
	return &Refusal{Rule: rule, Reason: reason, Msg: fmt.Sprintf(format, args...)}
}

// Decision is an accepted opt-in.
type Decision struct {
	// Milli is the millicpus asked for.
	Milli int
	// Source is "resource" (device plugin annotation) or "annotation"
	// (the cpu-shared.ergon.io/<container> fallback).
	Source string
	// Exclusive is the kubelet's cpuset for the container.
	Exclusive cpuset.Set
	// Need is the number of shared CPUs to add.
	Need int
	// Hook is true when the start-affinity hook is injected.
	Hook bool
	// Grants is the latency-pod extension set: rlimits, seccomp widening,
	// NUMA binding, the PHC device and the hugepages hint. Nil fields mean
	// "not asked for".
	Grants *Grants
	// Warnings are logged, not refused.
	Warnings []string
}

// Grants is the latency-pod extension set for one container.
type Grants struct {
	// Rlimits are the POSIX rlimits to add (from the plugin config).
	Rlimits []latency.Rlimit
	// SchedFifo is the SCHED_FIFO priority the hot thread may take, or 0.
	// The plugin never sets a container-wide realtime policy.
	SchedFifo int
	// SeccompMode is latency.ModeIOUring or latency.ModeRuntimeDefault.
	SeccompMode string
	// Seccomp is the policy to apply, or nil for "leave the request's".
	Seccomp *latency.SeccompPolicy
	// NumaMems is the cpuset.mems string, or "" for no binding.
	NumaMems string
	// PtpDevice is the /dev/ptp* path to add, or "" for none.
	PtpDevice string
	// CpuqosDevice is /dev/cpu_dma_latency (x86 only), or "" for none.
	CpuqosDevice string
	// Hugepage is the sized hugetlbfs mount, or nil for none.
	Hugepage *latency.HugepageSpec
	// HugepageEnvs are the env pairs that go with Hugepage.
	HugepageEnvs []latency.KV
	// CpuPartition asks the createRuntime hook for cpuset.cpus.partition=
	// isolated on the exclusive CPUs.
	CpuPartition bool
	// Partition is filled in at apply time: the child cgroup the hook will
	// create. Nil when CpuPartition is false.
	Partition *latency.PartitionGrant
	// NodeFacts is the /run/ergon/node.json bind mount, or "" for none.
	NodeFacts string
}

// Admit applies the admission rules. It returns (nil, nil) for a container
// that does not opt in, a *Refusal for one that breaks a rule, and a
// Decision otherwise.
func Admit(cfg *config.Config, r Request) (*Decision, error) {
	d := &Decision{}
	devAnn, fromDevice := r.Annotations[names.MilliAnnotation]
	podAnn, fromPod := r.PodAnnotations[names.OptInPrefix+r.Container]
	envMilli, envSet := r.Env[names.EnvMilli]
	switch {
	case fromDevice:
		m, err := parseMilli(devAnn)
		if err != nil {
			return nil, refuse(0, "bad-annotation", "container annotation %s: %v", names.MilliAnnotation, err)
		}
		d.Milli, d.Source = m, "resource"
		if fromPod {
			d.Warnings = append(d.Warnings, fmt.Sprintf("pod annotation %s%s ignored: the resource %s wins", names.OptInPrefix, r.Container, names.Resource))
		}
		if envSet && envMilli != devAnn {
			d.Warnings = append(d.Warnings, fmt.Sprintf("pod env %s=%q replaced by %s", names.EnvMilli, envMilli, devAnn))
		}
	case fromPod && cfg.AnnotationOptIn:
		m, err := parseMilli(podAnn)
		if err != nil {
			return nil, refuse(0, "bad-annotation", "pod annotation %s%s: %v", names.OptInPrefix, r.Container, err)
		}
		d.Milli, d.Source = m, "annotation"
		// The pod may set EnvMilli itself here: then ergon-cpus refuses to
		// run when this plugin did not set the container up (fail closed).
		if envSet && envMilli != podAnn {
			d.Warnings = append(d.Warnings, fmt.Sprintf("pod env %s=%q replaced by %s", names.EnvMilli, envMilli, podAnn))
		}
	case fromPod:
		return nil, refuse(0, "annotation-off", "pod annotation %s%s: the annotation opt-in is off on this node; request %s", names.OptInPrefix, r.Container, names.Resource)
	case envSet:
		return nil, refuse(7, "env-by-hand", "%s is set by hand; request %s (the device plugin annotation is missing; containerd >= 2.0 is needed)", names.EnvMilli, names.Resource)
	default:
		// No shared-CPU opt-in. The latency-pod grants (rlimits, seccomp,
		// NUMA, ptp, sched-fifo, hugepages, cpu-partition, cpuqos) are opt-in
		// by their own annotations and do not need the millicpu resource.
		// Honour them, and refuse the ones this node cannot give (fail closed).
		g, err := resolveGrants(cfg, &r)
		if err != nil {
			return nil, err
		}
		if g == nil {
			return nil, nil
		}
		if !cfg.NamespaceAllowed(r.Namespace) {
			return nil, refuse(1, "namespace", "namespace %s may not use the ergon latency grants", r.Namespace)
		}
		if g.CpuPartition {
			// The child cgroup holds exactly the kubelet's cpuset: the pod is
			// a Guaranteed integer-CPU container whose exclusive set is known.
			if r.Cpus == "" {
				return nil, refuse(2, "no-cpuset", "no cpuset from the kubelet: static CPU manager needed")
			}
			e, err := cpuset.Parse(r.Cpus)
			if err != nil {
				return nil, refuse(2, "no-cpuset", "cpuset from the kubelet does not parse: %v", err)
			}
			if both := e.Intersection(cfg.Shared); !both.IsEmpty() {
				return nil, refuse(5, "overlap", "CPU %s is both exclusive and shared: sharedCpus must be inside reservedSystemCPUs", both)
			}
			d.Exclusive = e
		}
		d.Grants = g
		return d, nil
	}

	// Rule 1.
	if !cfg.NamespaceAllowed(r.Namespace) {
		return nil, refuse(1, "namespace", "namespace %s may not use shared CPUs", r.Namespace)
	}
	// Rule 2.
	if r.Cpus == "" {
		return nil, refuse(2, "no-cpuset", "no cpuset from the kubelet: static CPU manager needed")
	}
	e, err := cpuset.Parse(r.Cpus)
	if err != nil {
		return nil, refuse(2, "no-cpuset", "cpuset from the kubelet does not parse: %v", err)
	}
	// Rule 3.
	if r.Quota != nil && *r.Quota > 0 {
		return nil, refuse(3, "cpu-quota", "CPU quota is set: needs a Guaranteed pod with integer CPUs on Kubernetes >= 1.33")
	}
	// Rule 4.
	if r.Shares == nil || *r.Shares == 0 || *r.Shares%1024 != 0 || uint64(e.Len()) != *r.Shares/1024 {
		shares := "unset"
		if r.Shares != nil {
			shares = strconv.FormatUint(*r.Shares, 10)
		}
		return nil, refuse(4, "not-exclusive", "cpuset is not exclusive (Guaranteed pod, integer cpu): cpuset %s, cpu shares %s", e, shares)
	}
	// Rule 5.
	if both := e.Intersection(cfg.Shared); !both.IsEmpty() {
		return nil, refuse(5, "overlap", "CPU %s is both exclusive and shared: sharedCpus must be inside reservedSystemCPUs", both)
	}
	// Rule 6.
	d.Need = assign.Need(d.Milli)
	if d.Need > cfg.Shared.Len() {
		return nil, refuse(6, "too-many", "%d millicpus need %d shared CPUs; this node has %d", d.Milli, d.Need, cfg.Shared.Len())
	}
	d.Exclusive = e

	d.Hook = cfg.DefaultAffinity == config.AffinityShared
	if v, ok := r.PodAnnotations[names.AffinityPrefix+r.Container]; ok {
		switch v {
		case config.AffinityAll:
			if r.Hot {
				return nil, refuse(0, "hook-opt-out", "hot containers cannot select %s%s: all; the start-affinity hook is required", names.AffinityPrefix, r.Container)
			}
			d.Hook = false
		case config.AffinityShared:
			d.Hook = true
		default:
			return nil, refuse(0, "bad-annotation", "pod annotation %s%s is %q; use %q or %q", names.AffinityPrefix, r.Container, v, config.AffinityShared, config.AffinityAll)
		}
	}
	if r.Hot {
		d.Hook = true
	}
	for _, k := range []string{names.EnvIsolated, names.EnvIsolatedCores, names.EnvShared, names.EnvHelper, names.EnvOpenShiftIsolated, names.EnvOpenShiftShared} {
		if v, ok := r.Env[k]; ok {
			d.Warnings = append(d.Warnings, fmt.Sprintf("pod env %s=%q replaced", k, v))
		}
	}
	g, err := resolveGrants(cfg, &r)
	if err != nil {
		return nil, err
	}
	d.Grants = g
	return d, nil
}

func parseMilli(s string) (int, error) {
	m, err := strconv.Atoi(s)
	if err != nil || m < 1 || m > MaxMilli || strconv.Itoa(m) != s {
		return 0, fmt.Errorf("%q is not a whole number of millicpus in 1..%d", s, MaxMilli)
	}
	return m, nil
}

// KV is one environment variable.
type KV struct{ Key, Value string }

// Env is the environment the plugin sets: exclusive CPUs e, shared CPUs s,
// e grouped by core, and the millicpus asked for.
func Env(e, s cpuset.Set, cores []cpuset.Set, milli int) []KV {
	return []KV{
		{names.EnvIsolated, e.List()},
		{names.EnvIsolatedCores, cpuset.FormatCores(cores)},
		{names.EnvShared, s.List()},
		{names.EnvMilli, strconv.Itoa(milli)},
		{names.EnvHelper, s.List()},
		{names.EnvOpenShiftIsolated, e.String()},
		{names.EnvOpenShiftShared, s.String()},
	}
}

// HookTimeoutSeconds bounds the createRuntime hook.
const HookTimeoutSeconds = 5

// Hook is the createRuntime hook. It starts the container on shared when that
// set is not empty, and builds the isolated child cgroup when part is not nil.
// At least one of the two must be set.
func Hook(path string, shared cpuset.Set, part *latency.PartitionGrant) *api.Hook {
	args := []string{path, "hook"}
	if !shared.IsEmpty() {
		args = append(args, "--shared", shared.List())
	}
	if part != nil {
		args = append(args, "--isolated", part.Exclusive, "--partition-child", part.Child)
	}
	return &api.Hook{
		Path:    path,
		Args:    args,
		Timeout: &api.OptionalInt{Value: HookTimeoutSeconds},
	}
}

// ReleaseHook is the poststop hook. It takes the isolated child cgroup and
// its ancestor reservations back down once the container's processes are
// gone. NRI delivers poststop outside the plugin's request handlers, where
// the filesystem work cannot eat into the plugin's request timeout.
func ReleaseHook(path string, part *latency.PartitionGrant) *api.Hook {
	args := []string{path, "hook", "--isolated", part.Exclusive, "--release-child", part.Child}
	return &api.Hook{
		Path:    path,
		Args:    args,
		Timeout: &api.OptionalInt{Value: HookTimeoutSeconds},
	}
}

// key names a container across restarts.
func key(r Request) assign.Key { return assign.Key{PodUID: r.PodUID, Container: r.Container} }
