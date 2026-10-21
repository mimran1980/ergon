// SPDX-License-Identifier: Apache-2.0

package nriplugin

import (
	"strconv"
	"strings"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/config"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/latency"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

// resolveGrants turns the opt-in annotations into the grants to apply. It
// returns (nil, nil) when the pod asked for nothing beyond shared CPUs. A
// grant that cannot be given is a typed refusal (fail closed): the container
// is not created, and the pod's events name the fix.
//
// Annotation contract (pod authors name kinds and modes; values come from the
// plugin config or the node's facts, never from the pod):
//
//	rlimits.ergon.io/<container>:      "memlock,rt-prio,nofile"
//	seccomp.ergon.io/<container>:      "io-uring" | "runtime-default"
//	numa.ergon.io/<container>:         "0" | "0,1" | "" (the NIC's node)
//	ptp.ergon.io/<container>:          "off" | "auto" | /dev/ptpN
//	sched-fifo.ergon.io/<container>:   "thread" | "off"
//	hugepages.ergon.io/<container>:    "2M"
//	cpu-partition.ergon.io/<container>:"isolated"
//	cpuqos.ergon.io/<container>:       "<microseconds>" (x86 only)
func resolveGrants(cfg *config.Config, r *Request) (*Grants, error) {
	g := &Grants{}

	if ask, ok := lookup(r, names.RlimitsPrefix+r.Container); ok {
		rls, err := latency.Table(cfg.Rlimits, ask)
		if err != nil {
			return nil, err
		}
		g.Rlimits = rls
	}

	seccompAsk, hasSeccomp := lookup(r, names.SeccompPrefix+r.Container)
	if hasSeccomp {
		if !cfg.SeccompIOUring && seccompAsk == latency.ModeIOUring {
			return nil, refuse(0, "seccomp-off",
				"seccomp.ergon.io: %q is asked for but this node has seccomp.ioUring: off", latency.ModeIOUring)
		}
		policy, err := latency.WithIOUring(r.SeccompPolicy, seccompAsk)
		if err != nil {
			return nil, err
		}
		g.SeccompMode = seccompAsk
		if seccompAsk == latency.ModeIOUring {
			g.Seccomp = policy
		}
	} else {
		g.SeccompMode = latency.ModeRuntimeDefault
	}

	numaAsk, hasNuma := lookup(r, names.NumaPrefix+r.Container)
	if hasNuma {
		mems, _, err := latency.PickMems(numaAsk, cfg.NodeNumaNodes, cfg.NicNumaNode)
		if err != nil {
			return nil, err
		}
		g.NumaMems = mems
	}

	ptpAsk, hasPtp := lookup(r, names.PtpPrefix+r.Container)
	if hasPtp {
		dev, err := latency.PickPtp(cfg.HostPtp, ptpAsk)
		if err != nil {
			return nil, err
		}
		g.PtpDevice = dev
		// The node facts travel with the PHC: the app reads the NIC's node
		// and the PHC name from one file.
		g.NodeFacts = names.NodeFactsMountPoint
	}

	if fifoAsk, hasFifo := lookup(r, names.SchedFifoPrefix+r.Container); hasFifo {
		switch fifoAsk {
		case config.SchedFifoOff:
			g.SchedFifo = 0
		case config.SchedFifoThread:
			if cfg.SchedFifo != config.SchedFifoThread {
				return nil, refuse(0, "fifo-off",
					"sched-fifo.ergon.io: %q is asked for but this node has schedFifo: off", config.SchedFifoThread)
			}
			// Priority comes from the rt-prio rlimit when present, else 50.
			prio := 50
			for _, rl := range g.Rlimits {
				if rl.Type == "RLIMIT_RTPRIO" && rl.Soft != latency.Inf {
					prio = int(rl.Soft)
				}
			}
			g.SchedFifo = prio
		default:
			return nil, refuse(0, "bad-annotation", "sched-fifo.ergon.io: %q must be %q or %q",
				fifoAsk, config.SchedFifoOff, config.SchedFifoThread)
		}
	}

	// Item 1: a sized hugetlbfs mount for AERON_DIR. The size is the
	// container's hugepages-2Mi limit, so two containers cannot overcommit.
	if hugeAsk, hasHuge := lookup(r, names.HugepagesPrefix+r.Container); hasHuge {
		if err := latency.CheckHugepageAnnotation(hugeAsk); err != nil {
			return nil, err
		}
		spec, envs, err := latency.HugepageMount(r.HugepageLimitBytes)
		if err != nil {
			return nil, err
		}
		g.Hugepage = &spec
		g.HugepageEnvs = envs
	}

	// Item 2: cpuset.cpus.partition=isolated on the exclusive CPUs. The
	// createRuntime hook creates the child cgroup and writes the files.
	if partAsk, hasPart := lookup(r, names.CpuPartitionPrefix+r.Container); hasPart {
		if err := latency.CheckCpuPartitionValue(partAsk); err != nil {
			return nil, err
		}
		g.CpuPartition = true
	}

	// Item 5: PM QoS via /dev/cpu_dma_latency. x86 only.
	if qosAsk, hasQos := lookup(r, names.CpuqosPrefix+r.Container); hasQos {
		dev, err := latency.PickCpuQos(qosAsk, cfg.GOARCH, cfg.HostCpuDmaLatency)
		if err != nil {
			return nil, err
		}
		g.CpuqosDevice = dev
	}

	if len(g.Rlimits) == 0 && g.SchedFifo == 0 && g.Seccomp == nil &&
		g.NumaMems == "" && g.PtpDevice == "" && g.CpuqosDevice == "" &&
		g.Hugepage == nil && !g.CpuPartition {
		return nil, nil
	}
	return g, nil
}

// lookup finds an annotation on the container first (a pod author cannot set
// container annotations, so the device plugin can override), then on the pod.
func lookup(r *Request, key string) (string, bool) {
	if v, ok := r.Annotations[key]; ok {
		return v, true
	}
	if v, ok := r.PodAnnotations[key]; ok {
		return v, true
	}
	return "", false
}

// GrantEnv is the env the plugin exports for the grants it gave.
func GrantEnv(g *Grants, _ *config.Config) []latency.KV {
	if g == nil {
		return nil
	}
	return latency.GrantEnv(latency.EnvGrant{
		Rlimits:      g.Rlimits,
		SchedFifo:    g.SchedFifo,
		SeccompMode:  g.SeccompMode,
		NumaNodes:    g.NumaMems,
		PtpDevice:    g.PtpDevice,
		CpuqosDevice: g.CpuqosDevice,
		Partition:    g.Partition,
		NodeFacts:    g.NodeFacts,
		HugepageEnvs: g.HugepageEnvs,
	})
}

// FormatSchedFifo renders the priority for ERGON_SCHED_FIFO, or "" for none.
func FormatSchedFifo(prio int) string {
	if prio <= 0 {
		return ""
	}
	return strconv.Itoa(prio)
}

// SysctlRefusal names the node setting for a candidate-7 request. The plugin
// refuses these with this message so nobody expects a pod to raise rmem_max.
func SysctlRefusal(name string) string { return latency.RefuseSysctl(name) }

// SplitCommaList is a tiny helper for the hook argument parser and tests.
func SplitCommaList(s string) []string {
	var out []string
	for _, p := range strings.Split(s, ",") {
		p = strings.TrimSpace(p)
		if p != "" {
			out = append(out, p)
		}
	}
	return out
}
