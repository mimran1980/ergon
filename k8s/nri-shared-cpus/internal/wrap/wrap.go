// SPDX-License-Identifier: Apache-2.0

// Package wrap implements the ergon-cpus exec wrapper: it turns index specs
// like "isolated:0" into real CPU numbers, starts the command on the shared
// CPUs and execs it. Use it as the image entrypoint when the OCI hook cannot
// run, and to give programs that want real CPU numbers (the Aeron media
// driver, Java affinity libraries) the ones this container got.
package wrap

import (
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"strconv"
	"strings"

	"golang.org/x/sys/unix"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

// CgroupCPUs is where the container's real cpuset lives (cgroup v2).
const CgroupCPUs = "/sys/fs/cgroup/cpuset.cpus.effective"

// Plan is the checked container CPU layout.
type Plan struct {
	// Isolated is the container's exclusive CPUs (ERGON_ISOLATED_CPUS).
	Isolated cpuset.Set
	// Shared is this container's shared CPUs (ERGON_SHARED_CPUS).
	Shared cpuset.Set
	// Cores groups Isolated by physical core (ERGON_ISOLATED_CORES).
	Cores []cpuset.Set
	// Allowed is the container's real cpuset (cpuset.cpus.effective).
	Allowed cpuset.Set
	// Milli is ERGON_SHARED_MILLICPUS, 0 when unset.
	Milli int
}

// FromEnv reads the plugin's env and checks both lists against the CPUs the
// kernel actually gave this container. Without a partition both are inside
// the container's cpuset. With one, the exclusive list is the partition
// child's cpuset exactly and the shared list is inside what is left of the
// container's: a valid partition takes the exclusive CPUs out of the
// container's cpuset.cpus.effective.
//
// sysRoot is usually /sys (cores are read from it when ERGON_ISOLATED_CORES
// is missing). env is usually os.Environ.
func FromEnv(env []string, cgroupCPUs, sysRoot string) (*Plan, error) {
	get := func(key string) (string, bool) {
		for _, e := range env {
			if k, v, ok := strings.Cut(e, "="); ok && k == key {
				return v, true
			}
		}
		return "", false
	}
	isoText, hasIso := get(names.EnvIsolated)
	sharedText, hasShared := get(names.EnvShared)
	if !hasIso {
		if v, ok := get(names.EnvOpenShiftIsolated); ok {
			isoText, hasIso = v, true
		}
	}
	if !hasShared {
		if v, ok := get(names.EnvOpenShiftShared); ok {
			sharedText, hasShared = v, true
		}
	}
	_, hasMilli := get(names.EnvMilli)
	if !hasShared {
		if hasMilli {
			return nil, fmt.Errorf("%s is set but %s is not: the NRI plugin did not set up this container", names.EnvMilli, names.EnvShared)
		}
		return nil, fmt.Errorf("%s and %s are not set: the NRI plugin did not set up this container", names.EnvIsolated, names.EnvShared)
	}
	if !hasIso {
		return nil, fmt.Errorf("%s is not set: the NRI plugin did not set up this container", names.EnvIsolated)
	}
	p := &Plan{}
	if v, ok := get(names.EnvMilli); ok {
		m, err := strconv.Atoi(v)
		if err != nil || m < 0 {
			return nil, fmt.Errorf("%s=%q is not a number of millicpus", names.EnvMilli, v)
		}
		p.Milli = m
	}
	iso, err := cpuset.Parse(isoText)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", names.EnvIsolated, err)
	}
	shared, err := cpuset.Parse(sharedText)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", names.EnvShared, err)
	}
	p.Isolated, p.Shared = iso, shared

	if coresText, ok := get(names.EnvIsolatedCores); ok && coresText != "" {
		p.Cores, err = cpuset.ParseCores(coresText)
		if err != nil {
			return nil, fmt.Errorf("%s: %w", names.EnvIsolatedCores, err)
		}
	} else {
		sibs, err := cpuset.ReadSiblings(sysRoot, iso)
		if err != nil {
			return nil, fmt.Errorf("core topology: %w", err)
		}
		p.Cores = cpuset.Cores(iso, sibs)
	}

	data, err := os.ReadFile(cgroupCPUs)
	if err != nil {
		return nil, fmt.Errorf("read %s: %w", cgroupCPUs, err)
	}
	allowed, err := cpuset.Parse(string(data))
	if err != nil {
		return nil, fmt.Errorf("%s: %w", cgroupCPUs, err)
	}
	p.Allowed = allowed
	// A valid isolated partition takes the exclusive CPUs out of the
	// container's cpuset.cpus.effective, so E is not in Allowed and cannot
	// be checked against it. E belongs to the partition child; S stays with
	// the container. Without a partition, both are in the container's set.
	part, hasPart := partitionCPUs(filepath.Dir(cgroupCPUs))
	if hasPart {
		if !iso.Equals(part) {
			return nil, fmt.Errorf("%s is %s, but the isolated partition %s holds %s",
				names.EnvIsolated, iso, filepath.Join(filepath.Dir(cgroupCPUs), names.CpuPartitionChild), part)
		}
		if missing := shared.Difference(allowed); !missing.IsEmpty() {
			return nil, fmt.Errorf("%s is %s, which is not in %s (%s)", names.EnvShared, missing, cgroupCPUs, allowed)
		}
	} else {
		if missing := iso.Difference(allowed); !missing.IsEmpty() {
			return nil, fmt.Errorf("%s is %s, which is not in %s (%s)", names.EnvIsolated, missing, cgroupCPUs, allowed)
		}
		if missing := shared.Difference(allowed); !missing.IsEmpty() {
			return nil, fmt.Errorf("%s is %s, which is not in %s (%s)", names.EnvShared, missing, cgroupCPUs, allowed)
		}
	}
	if both := iso.Intersection(shared); !both.IsEmpty() {
		return nil, fmt.Errorf("CPU %s is both isolated and shared", both)
	}
	return p, nil
}

// partitionCPUs reads the isolated child's cpuset.cpus, under the container
// directory that cgroupCPUs lives in. The second result is false when there
// is no partition, or when the child exists but holds no CPUs yet. A child
// holding CPUs is the partition: those CPUs are no longer in the container's
// own effective set, which is exactly the case the caller has to allow.
func partitionCPUs(containerDir string) (cpuset.Set, bool) {
	data, err := os.ReadFile(filepath.Join(containerDir, names.CpuPartitionChild, "cpuset.cpus"))
	if err != nil {
		return cpuset.Set{}, false
	}
	set, err := cpuset.Parse(strings.TrimSpace(string(data)))
	if err != nil || set.IsEmpty() {
		return cpuset.Set{}, false
	}
	return set, true
}

// Resolve turns one SPEC into a value for an environment variable.
//
//	isolated:<i>     the i-th CPU of ERGON_ISOLATED_CPUS
//	shared:<i>       the i-th CPU of ERGON_SHARED_CPUS
//	core:<j>         the first CPU of core group j
//	core:<j>:<t>     CPU t of core group j
//	isolated         the whole isolated list, comma-separated
//	shared           the whole shared list, comma-separated
func (p *Plan) Resolve(spec string) (string, error) {
	kind, rest, hasIndex := strings.Cut(spec, ":")
	switch kind {
	case "isolated", "shared":
		set := p.Isolated
		if kind == "shared" {
			set = p.Shared
		}
		if !hasIndex {
			return set.List(), nil
		}
		i, err := strconv.Atoi(rest)
		if err != nil || i < 0 {
			return "", fmt.Errorf("%q: %q is not a CPU index", spec, rest)
		}
		cpus := set.Slice()
		if i >= len(cpus) {
			return "", fmt.Errorf("%q: index %d is out of range; %s is %s", spec, i, kind, set.List())
		}
		return strconv.Itoa(cpus[i]), nil
	case "core":
		rest, tText, hasT := strings.Cut(rest, ":")
		j, err := strconv.Atoi(rest)
		if err != nil || j < 0 {
			return "", fmt.Errorf("%q: %q is not a core index", spec, rest)
		}
		if j >= len(p.Cores) {
			return "", fmt.Errorf("%q: core %d is out of range; there are %d cores", spec, j, len(p.Cores))
		}
		cpus := p.Cores[j].Slice()
		t := 0
		if hasT {
			if t, err = strconv.Atoi(tText); err != nil || t < 0 {
				return "", fmt.Errorf("%q: %q is not a thread index", spec, tText)
			}
		}
		if t >= len(cpus) {
			return "", fmt.Errorf("%q: thread %d is out of range; core %d has %d CPU(s)", spec, t, j, len(cpus))
		}
		return strconv.Itoa(cpus[t]), nil
	default:
		return "", fmt.Errorf("%q: unknown spec kind %q (use isolated, shared or core)", spec, kind)
	}
}

// EnvBinding is one --env NAME=SPEC to translate.
type EnvBinding struct {
	Name string
	Spec string
}

// AeronCpusetOffsets are the Aeron settings that make AERON_*_CPU_AFFINITY
// values offsets into the cgroup cpuset instead of real CPU numbers: Aeron
// 1.54's name, then the name before it.
var AeronCpusetOffsets = []string{"AERON_THREAD_AFFINITY_USE_CPUSET_OFFSETS", "AERON_DRIVER_CPUSET_AFFINITY"}

// Translate resolves every binding and returns the environment assignments.
// It refuses a translated AERON_*_CPU_AFFINITY when one of
// AeronCpusetOffsets is true: Aeron would then read our real CPU number as
// an offset into the cpuset.
func (p *Plan) Translate(env []string, bindings []EnvBinding) ([]string, error) {
	aeronCpuset := ""
	for _, e := range env {
		k, v, ok := strings.Cut(e, "=")
		if ok && slices.Contains(AeronCpusetOffsets, k) && (strings.EqualFold(v, "true") || v == "1") {
			aeronCpuset = k
		}
	}
	out := make([]string, 0, len(bindings))
	for _, b := range bindings {
		v, err := p.Resolve(b.Spec)
		if err != nil {
			return nil, fmt.Errorf("--env %s=%s: %w", b.Name, b.Spec, err)
		}
		if aeronCpuset != "" && strings.HasPrefix(b.Name, "AERON_") && strings.HasSuffix(b.Name, "_CPU_AFFINITY") {
			return nil, fmt.Errorf("--env %s=%s: %s=true makes Aeron read this as an offset into the cgroup cpuset; unset it or pass --affinity keep and set the variable by hand",
				b.Name, b.Spec, aeronCpuset)
		}
		out = append(out, b.Name+"="+v)
	}
	return out, nil
}

// PinSelf sets the calling thread's affinity to s, so every thread the
// command creates starts there. Lock the OS thread first: execve keeps the
// mask of the calling thread.
func PinSelf(s cpuset.Set) error {
	var mask unix.CPUSet
	for _, c := range s.Slice() {
		if c >= 1024 {
			return fmt.Errorf("CPU %d is outside cpu_set_t", c)
		}
		mask.Set(c)
	}
	return unix.SchedSetaffinity(0, &mask)
}

// SelfAffinity returns the calling thread's affinity mask as a CPU set.
func SelfAffinity() (cpuset.Set, error) {
	var mask unix.CPUSet
	if err := unix.SchedGetaffinity(0, &mask); err != nil {
		return cpuset.Set{}, err
	}
	var cpus []int
	for c := 0; c < 1024; c++ {
		if mask.IsSet(c) {
			cpus = append(cpus, c)
		}
	}
	return cpuset.New(cpus...), nil
}

// Exec replaces the current process with cmd. The caller must have locked
// the OS thread and set its affinity.
func Exec(argv []string, env []string) error {
	if len(argv) == 0 {
		return fmt.Errorf("no command to run")
	}
	path, err := execPath(argv[0])
	if err != nil {
		return err
	}
	return unix.Exec(path, argv, env)
}

func execPath(name string) (string, error) {
	if strings.Contains(name, "/") {
		return name, nil
	}
	path := os.Getenv("PATH")
	for _, dir := range filepath.SplitList(path) {
		if dir == "" {
			dir = "."
		}
		candidate := filepath.Join(dir, name)
		if st, err := os.Stat(candidate); err == nil && !st.IsDir() && st.Mode()&0o111 != 0 {
			return candidate, nil
		}
	}
	return "", fmt.Errorf("%s is not on PATH (%s)", name, path)
}

// SetFifo puts the calling thread on SCHED_FIFO at the given priority. It
// needs RLIMIT_RTPRIO > 0 or CAP_SYS_NICE; the NRI plugin grants the rlimit
// so the pod spec stays unprivileged. A failure is typed so the caller can
// name the fix. The policy survives exec (it is per-thread).
func SetFifo(prio int) error {
	if prio < 1 || prio > 99 {
		return fmt.Errorf("SCHED_FIFO priority %d is outside 1..99", prio)
	}
	attr := unix.SchedAttr{
		Size:     unix.SizeofSchedAttr,
		Policy:   unix.SCHED_FIFO,
		Priority: uint32(prio),
	}
	if err := unix.SchedSetAttr(0, &attr, 0); err != nil {
		return fmt.Errorf("SCHED_FIFO %d: %w (need RLIMIT_RTPRIO or CAP_SYS_NICE; ask for %s)",
			prio, err, "sched-fifo.ergon.io")
	}
	return nil
}

// CurrentPolicy reports the calling thread's scheduling policy and priority.
// Policy is one of the SCHED_* constants. Used by tests and by `show`.
func CurrentPolicy() (uint32, uint32, error) {
	attr, err := unix.SchedGetAttr(0, 0)
	if err != nil {
		return 0, 0, err
	}
	return attr.Policy, attr.Priority, nil
}
