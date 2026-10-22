// SPDX-License-Identifier: Apache-2.0

// Package names holds the contract of this module: the resource name, the
// annotation keys and the environment variable names. Changing one is a
// breaking change.
package names

import "regexp"

// requiredPluginToken is the RE2 match the validating policy uses. A
// substring of the plugin name (not-ergon-shared-cpus, ergon-shared-cpus-extra)
// does not count. The annotation value is a JSON list of quoted names.
var requiredPluginToken = regexp.MustCompile(`(^|\[|,)\s*"` + PluginName + `"\s*(,|\]|$)`)

// RequiredPluginListed reports whether annotation, the
// required-plugins.noderesource.dev value, names this plugin as one token.
func RequiredPluginListed(annotation string) bool {
	return requiredPluginToken.MatchString(annotation)
}

const (
	// PluginName is the NRI plugin name. Pods list it in the
	// required-plugins.noderesource.dev annotation.
	PluginName = "ergon-shared-cpus"
	// PluginIdx is the NRI plugin index.
	PluginIdx = "50"

	// Resource is the kubelet extended resource, in millicpus.
	Resource = "ergon.io/shared-millicpus"
	// MilliAnnotation is the container annotation the device plugin sets.
	// Pod authors cannot set container annotations, so the NRI plugin trusts it.
	MilliAnnotation = "ergon.io/shared-millicpus"
	// OptInPrefix + container name is the fallback pod annotation
	// (config annotationOptIn); the value is millicpus.
	OptInPrefix = "cpu-shared.ergon.io/"
	// AffinityPrefix + container name is the pod annotation that turns the
	// start-affinity hook off for one container ("all") or keeps it ("shared").
	// A hot container cannot select "all": the hook stays on.
	AffinityPrefix = "default-affinity.ergon.io/"
	// RequiredPluginsAnnotation is containerd's default validator annotation.
	RequiredPluginsAnnotation = "required-plugins.noderesource.dev"
	// LatencyLabel marks a pod that must meet the hot-node contract.
	LatencyLabel = "ergon.io/latency"
	// LatencyHot is the LatencyLabel value for a trading container.
	LatencyHot = "hot"

	// RlimitsPrefix + container name is the pod annotation that asks for
	// rlimits: a comma list of memlock, nofile, rt-prio, nice. Values come
	// from the plugin config, never from the pod.
	RlimitsPrefix = "rlimits.ergon.io/"
	// SeccompPrefix + container name is the pod annotation that asks for a
	// seccomp widening: "io-uring" (RuntimeDefault plus io_uring) or
	// "runtime-default". "unconfined" is refused by default.
	SeccompPrefix = "seccomp.ergon.io/"
	// NumaPrefix + container name is the pod annotation that binds memory to
	// NUMA nodes ("0", "0,1"). Empty means the NIC's node.
	NumaPrefix = "numa.ergon.io/"
	// PtpPrefix + container name is the pod annotation that hands the pod the
	// NIC PHC: "off", "auto" or a /dev/ptp* path.
	PtpPrefix = "ptp.ergon.io/"
	// SchedFifoPrefix + container name is the pod annotation that grants
	// SCHED_FIFO on the hot thread only ("thread") or refuses ("off").
	SchedFifoPrefix = "sched-fifo.ergon.io/"
	// HugepagesPrefix + container name is the pod annotation that asks for a
	// sized hugetlbfs mount at /dev/hugepages for AERON_DIR. The only value
	// is "2M". The size comes from the container's hugepages-2Mi limit.
	HugepagesPrefix = "hugepages.ergon.io/"
	// CpuPartitionPrefix + container name is the pod annotation that asks for
	// cpuset.cpus.partition=isolated on the exclusive CPUs. The only value is
	// "isolated".
	CpuPartitionPrefix = "cpu-partition.ergon.io/"
	// CpuqosPrefix + container name is the pod annotation that hands the pod
	// /dev/cpu_dma_latency (PM QoS). The value is the target latency in
	// microseconds. x86 only: ignored on arm64.
	CpuqosPrefix = "cpuqos.ergon.io/"

	// EnvIsolated is the container's exclusive CPUs, a sorted comma list.
	EnvIsolated = "ERGON_ISOLATED_CPUS"
	// EnvIsolatedCores is EnvIsolated grouped by physical core: "1,5;2,6".
	EnvIsolatedCores = "ERGON_ISOLATED_CORES"
	// EnvShared is the container's shared CPUs, a sorted comma list.
	EnvShared = "ERGON_SHARED_CPUS"
	// EnvMilli is the millicpus asked for.
	EnvMilli = "ERGON_SHARED_MILLICPUS"
	// EnvHelper is ergon-io's name for EnvShared.
	EnvHelper = "ERGON_HELPER_CPUS"
	// EnvOpenShiftIsolated is CRI-O's name for the exclusive CPUs (with ranges).
	EnvOpenShiftIsolated = "OPENSHIFT_ISOLATED_CPUS"
	// EnvOpenShiftShared is CRI-O's name for the shared CPUs (with ranges).
	EnvOpenShiftShared = "OPENSHIFT_SHARED_CPUS"

	// EnvIsolatedCgroup is the child cgroup that holds the exclusive CPUs as
	// an isolated partition (cpu-partition.ergon.io). It is a threaded child,
	// so both kinds of move work here: a whole process with
	// $ERGON_ISOLATED_CGROUP/cgroup.procs, and one thread of a multi-threaded
	// program with $ERGON_ISOLATED_CGROUP/cgroup.threads. The kernel then
	// confines that thread to the exclusive CPUs and keeps the helpers on the
	// container's shared CPUs. A thread moves across the threaded subtree
	// only once its whole process is inside it: put the process in with
	// cgroup.procs first, then move the threads.
	//
	// In a threaded child, cgroup.procs is write-only: reading it is
	// EOPNOTSUPP, because the processes belong to the thread root. Read
	// cgroup.threads for the membership list.
	EnvIsolatedCgroup = "ERGON_ISOLATED_CGROUP"
	// EnvHugepagesDir is the sized hugetlbfs mount the plugin gave the pod.
	EnvHugepagesDir = "ERGON_HUGEPAGES_DIR"
	// EnvHugepageSize is the hugepage size in bytes (2097152 for 2 MiB).
	EnvHugepageSize = "ERGON_HUGEPAGE_SIZE"
	// EnvPtpDevice is the NIC PHC handed to the pod (read-only).
	EnvPtpDevice = "ERGON_PTP_DEVICE"
	// EnvCpuDmaLatency is /dev/cpu_dma_latency (PM QoS, x86 only).
	EnvCpuDmaLatency = "ERGON_CPU_DMA_LATENCY"
	// EnvNodeFacts is the mounted node facts file (/run/ergon/node.json).
	EnvNodeFacts = "ERGON_NODE_FACTS"

	// HugepageMountPoint is where the plugin mounts the sized hugetlbfs.
	HugepageMountPoint = "/dev/hugepages"
	// NodeFactsPath is the node facts file the plugin bind-mounts read-only.
	NodeFactsPath = "/run/ergon/node.json"
	// NodeFactsMountPoint is where the node facts file appears in the pod.
	NodeFactsMountPoint = "/run/ergon/node.json"
	// CpuDmaLatencyPath is the PM QoS device (x86 only).
	CpuDmaLatencyPath = "/dev/cpu_dma_latency"
	// CpuPartitionValue is the only accepted cpu-partition.ergon.io value.
	CpuPartitionValue = "isolated"
	// CpuPartitionChild is the child cgroup name under the container cgroup.
	CpuPartitionChild = "ergon-isolated"
)
