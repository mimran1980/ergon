// SPDX-License-Identifier: Apache-2.0

// Package config reads and checks the plugin's config file.
package config

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"

	"sigs.k8s.io/yaml"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/latency"
)

const (
	// DefaultHookPath is where the DaemonSet installs ergon-cpus on the node.
	DefaultHookPath = "/opt/ergon/bin/ergon-cpus"
	// DefaultMetricsAddress serves /metrics, /healthz and /readyz.
	DefaultMetricsAddress = ":9466"
	// DefaultSeccompProfilePath is where the DaemonSet installs the shipped
	// io_uring profile on the node (the Localhost fallback).
	DefaultSeccompProfilePath = "/etc/ergon-shared-cpus/ergon-io-uring.json"
	// DefaultCgroupRoot is the cgroup v2 mount point.
	DefaultCgroupRoot = "/sys/fs/cgroup"
	// DefaultDevRoot is where the plugin sees the host's /dev when the
	// DaemonSet does not remount it.
	DefaultDevRoot = "/dev"
	// MaxDeviceIDs caps the device plugin's IDs. One ListAndWatch reply must
	// stay under the kubelet's 4 MiB gRPC message limit (about 20 bytes an ID).
	MaxDeviceIDs = 100000
	// AffinityShared starts every container thread on the shared CPUs (hook).
	AffinityShared = "shared"
	// AffinityAll leaves the start affinity alone (no hook; use the wrapper).
	AffinityAll = "all"
)

// File is the YAML file as written. Unknown fields are errors.
type File struct {
	SharedCPUs       *string  `json:"sharedCpus"`
	Oversubscription *int     `json:"oversubscription,omitempty"`
	Namespaces       []string `json:"namespaces,omitempty"`
	DefaultAffinity  string   `json:"defaultAffinity,omitempty"`
	HookPath         string   `json:"hookPath,omitempty"`
	AnnotationOptIn  bool     `json:"annotationOptIn,omitempty"`
	MetricsAddress   string   `json:"metricsAddress,omitempty"`
	// Rlimits is the node-side rlimit ceiling. A pod asks for kinds, never
	// values.
	Rlimits *RlimitsFile `json:"rlimits,omitempty"`
	// Seccomp turns the io_uring widening on or off.
	Seccomp *SeccompFile `json:"seccomp,omitempty"`
	// SeccompProfilePath is where the shipped io_uring profile lives on the
	// node (the Localhost fallback for pods that cannot take the NRI grant).
	SeccompProfilePath string `json:"seccompProfilePath,omitempty"`
	// DevRoot is where this process can see the host's /dev. The DaemonSet
	// mounts the host's /dev at /host/dev so the plugin can stat the PHC and
	// PM QoS nodes the container image does not carry; granted paths stay the
	// container's /dev/... names.
	DevRoot string `json:"devRoot,omitempty"`
	// SchedFifo is "off" or "thread" (hot thread only, never container-wide),
	// or a YAML boolean (true = thread).
	SchedFifo string `json:"schedFifo,omitempty"`
	// Ptp is "off", "auto" or a /dev/ptp* path the node has.
	Ptp string `json:"ptp,omitempty"`
	// HugepagesAeronDir is the AERON_DIR on hugetlbfs when the pod mounts one.
	HugepagesAeronDir string `json:"hugepagesAeronDir,omitempty"`
	// Profiles is a set of complete named configurations. It cannot be
	// combined with the legacy fields above. The process picks one at
	// start from a node label. It does not watch the node or change labels.
	Profiles map[string]Profile `json:"profiles,omitempty"`
	// NodeProfileLabel is the label whose value is the profile name.
	NodeProfileLabel string `json:"nodeProfileLabel,omitempty"`
}

// Profile is one named configuration. It has the operator fields of File
// and cannot nest another profile map.
type Profile struct {
	SharedCPUs         *string      `json:"sharedCpus"`
	Oversubscription   *int         `json:"oversubscription,omitempty"`
	Namespaces         []string     `json:"namespaces,omitempty"`
	DefaultAffinity    string       `json:"defaultAffinity,omitempty"`
	HookPath           string       `json:"hookPath,omitempty"`
	AnnotationOptIn    bool         `json:"annotationOptIn,omitempty"`
	MetricsAddress     string       `json:"metricsAddress,omitempty"`
	Rlimits            *RlimitsFile `json:"rlimits,omitempty"`
	Seccomp            *SeccompFile `json:"seccomp,omitempty"`
	SeccompProfilePath string       `json:"seccompProfilePath,omitempty"`
	DevRoot            string       `json:"devRoot,omitempty"`
	SchedFifo          string       `json:"schedFifo,omitempty"`
	Ptp                string       `json:"ptp,omitempty"`
	HugepagesAeronDir  string       `json:"hugepagesAeronDir,omitempty"`
}

func (p Profile) asFile() File {
	return File{
		SharedCPUs:         p.SharedCPUs,
		Oversubscription:   p.Oversubscription,
		Namespaces:         p.Namespaces,
		DefaultAffinity:    p.DefaultAffinity,
		HookPath:           p.HookPath,
		AnnotationOptIn:    p.AnnotationOptIn,
		MetricsAddress:     p.MetricsAddress,
		Rlimits:            p.Rlimits,
		Seccomp:            p.Seccomp,
		SeccompProfilePath: p.SeccompProfilePath,
		DevRoot:            p.DevRoot,
		SchedFifo:          p.SchedFifo,
		Ptp:                p.Ptp,
		HugepagesAeronDir:  p.HugepagesAeronDir,
	}
}

// RlimitsFile is the YAML rlimits block. "inf" or a positive count.
type RlimitsFile struct {
	Memlock string `json:"memlock,omitempty"`
	Nofile  string `json:"nofile,omitempty"`
	RtPrio  string `json:"rtPrio,omitempty"`
	Nice    string `json:"nice,omitempty"`
}

// SeccompFile is the YAML seccomp block.
type SeccompFile struct {
	// IOUring is "allow" or "off", or a YAML boolean (true = allow).
	IOUring string `json:"ioUring,omitempty"`
}

// Config is a checked configuration.
type Config struct {
	// Shared is the node's housekeeping CPUs that helpers may use.
	Shared cpuset.Set
	// Oversubscription multiplies the advertised millicpus (1..100).
	Oversubscription int
	// Namespaces is the allowlist of namespaces that may opt in.
	Namespaces []string
	// DefaultAffinity is AffinityShared or AffinityAll.
	DefaultAffinity string
	// HookPath is the absolute path of ergon-cpus on the node.
	HookPath string
	// AnnotationOptIn allows the cpu-shared.ergon.io/<container> fallback.
	AnnotationOptIn bool
	// MetricsAddress is the HTTP listen address.
	MetricsAddress string
	// Rlimits is the rlimit ceiling a pod may ask for (kinds, not values).
	Rlimits latency.RlimitConfig
	// SeccompIOUring enables the io_uring widening.
	SeccompIOUring bool
	// SeccompProfilePath is where the shipped io_uring profile lives on the
	// node. Default DefaultSeccompProfilePath.
	SeccompProfilePath string
	// DevRoot is where this process can see the host's /dev. Default "/dev";
	// the DaemonSet mounts the host's /dev at /host/dev and sets that.
	DevRoot string
	// SchedFifo is SchedFifoOff or SchedFifoThread.
	SchedFifo string
	// Ptp is "off", "auto" or a /dev/ptp* path.
	Ptp string
	// HugepagesAeronDir is the AERON_DIR to export when a pod mounts hugetlbfs.
	HugepagesAeronDir string

	// NodeNumaNodes is every NUMA node on the machine, as DiscoverHostNuma
	// read it from sysfs at start-up. It is never empty on a running plugin:
	// a kernel with no NUMA directory still has node 0. Empty means the
	// topology was never read, and every numa.ergon.io request is refused.
	NodeNumaNodes []int
	// NicNumaNode is the NUMA node the data NIC is on (-1 if unknown). While
	// it is unknown, numa.ergon.io: "" ("the NIC's node") is refused on a
	// multi-node machine and answered on a one-node one.
	NicNumaNode int
	// HostPtp is every /dev/ptp* the node has (filesystem, at start).
	HostPtp []string
	// HostCpuDmaLatency is /dev/cpu_dma_latency when the node has it (x86
	// PM QoS). Empty on arm64 and on guests that do not expose it.
	HostCpuDmaLatency string
	// GOARCH is the node's architecture ("amd64", "arm64"). The cpuqos grant
	// is x86 only; every other grant works on both.
	GOARCH string
	// CgroupRoot is the cgroup v2 mount point (usually /sys/fs/cgroup).
	CgroupRoot string
}

const (
	// SchedFifoOff grants no realtime policy.
	SchedFifoOff = "off"
	// SchedFifoThread grants RLIMIT_RTPRIO and ERGON_SCHED_FIFO; the app (or
	// prelock) takes SCHED_FIFO on the hot thread only.
	SchedFifoThread = "thread"
)

var dnsLabel = regexp.MustCompile(`^[a-z0-9]([-a-z0-9]{0,61}[a-z0-9])?$`)

// Parse reads and checks a config file. Every error names its field.
func Parse(data []byte) (*Config, error) {
	var f File
	if err := yaml.UnmarshalStrict(data, &f); err != nil {
		return nil, fmt.Errorf("config: %w", err)
	}
	profiles, err := checkMode(f)
	if err != nil {
		return nil, err
	}
	if profiles {
		return nil, errors.New("config: profiles need Resolve")
	}
	c := &Config{
		Oversubscription:   1,
		Namespaces:         f.Namespaces,
		DefaultAffinity:    AffinityShared,
		HookPath:           DefaultHookPath,
		AnnotationOptIn:    f.AnnotationOptIn,
		MetricsAddress:     DefaultMetricsAddress,
		SeccompProfilePath: DefaultSeccompProfilePath,
		CgroupRoot:         DefaultCgroupRoot,
		DevRoot:            DefaultDevRoot,
		// Nothing in this tree discovers the data NIC's node yet, so it
		// starts unknown: 0 would name a real NUMA node on a multi-node
		// machine.
		NicNumaNode: -1,
	}
	if f.SharedCPUs == nil {
		return nil, errors.New("config: sharedCpus is required")
	}
	shared, err := cpuset.Parse(*f.SharedCPUs)
	if err != nil {
		return nil, fmt.Errorf("config: sharedCpus: %w", err)
	}
	c.Shared = shared
	if f.Oversubscription != nil {
		c.Oversubscription = *f.Oversubscription
	}
	if c.Oversubscription < 1 || c.Oversubscription > 100 {
		return nil, fmt.Errorf("config: oversubscription %d is outside 1..100", c.Oversubscription)
	}
	if c.Capacity() > MaxDeviceIDs {
		return nil, fmt.Errorf("config: oversubscription %d gives %d device IDs (%d shared CPUs x 1000 x %d); the most is %d",
			c.Oversubscription, c.Capacity(), shared.Len(), c.Oversubscription, MaxDeviceIDs)
	}
	for _, ns := range f.Namespaces {
		if !dnsLabel.MatchString(ns) {
			return nil, fmt.Errorf("config: namespaces: %q is not a valid namespace name", ns)
		}
	}
	switch f.DefaultAffinity {
	case "":
	case AffinityShared, AffinityAll:
		c.DefaultAffinity = f.DefaultAffinity
	default:
		return nil, fmt.Errorf("config: defaultAffinity %q must be %q or %q", f.DefaultAffinity, AffinityShared, AffinityAll)
	}
	if f.HookPath != "" {
		if !filepath.IsAbs(f.HookPath) || filepath.Clean(f.HookPath) != f.HookPath {
			return nil, fmt.Errorf("config: hookPath %q must be a clean absolute path", f.HookPath)
		}
		// The daemon overwrites hookPath as root. The file name must stay
		// ergon-cpus so a typo cannot clobber an unrelated node file.
		if filepath.Base(f.HookPath) != "ergon-cpus" {
			return nil, fmt.Errorf("config: hookPath %q must end with /ergon-cpus (the daemon writes that file as root)", f.HookPath)
		}
		c.HookPath = f.HookPath
	}
	if f.MetricsAddress != "" {
		c.MetricsAddress = f.MetricsAddress
	}
	if f.Rlimits != nil {
		c.Rlimits = latency.RlimitConfig{
			Memlock: f.Rlimits.Memlock,
			Nofile:  f.Rlimits.Nofile,
			RtPrio:  f.Rlimits.RtPrio,
			Nice:    f.Rlimits.Nice,
		}
		for _, kv := range []struct{ name, val string }{
			{"rlimits.memlock", f.Rlimits.Memlock},
			{"rlimits.nofile", f.Rlimits.Nofile},
			{"rlimits.rtPrio", f.Rlimits.RtPrio},
			{"rlimits.nice", f.Rlimits.Nice},
		} {
			if kv.val == "" {
				continue
			}
			if _, err := parseRlimitValueForConfig(kv.val); err != nil {
				return nil, fmt.Errorf("config: %s: %w", kv.name, err)
			}
		}
	}
	ioUring, err := parseSwitch("seccomp.ioUring", f.Seccomp.GetIOUring(), "allow")
	if err != nil {
		return nil, err
	}
	c.SeccompIOUring = ioUring
	fifo, err := parseSwitch("schedFifo", f.SchedFifo, SchedFifoThread)
	if err != nil {
		return nil, err
	}
	c.SchedFifo = SchedFifoOff
	if fifo {
		c.SchedFifo = SchedFifoThread
	}
	switch f.Ptp {
	case "", "off":
		c.Ptp = "off"
	case "auto":
		c.Ptp = "auto"
	default:
		if !strings.HasPrefix(f.Ptp, "/dev/ptp") {
			return nil, fmt.Errorf("config: ptp %q must be %q, %q or a /dev/ptp* path", f.Ptp, "off", "auto")
		}
		c.Ptp = f.Ptp
	}
	if f.HugepagesAeronDir != "" {
		if !filepath.IsAbs(f.HugepagesAeronDir) {
			return nil, fmt.Errorf("config: hugepagesAeronDir %q must be an absolute path", f.HugepagesAeronDir)
		}
		c.HugepagesAeronDir = f.HugepagesAeronDir
	}
	if f.SeccompProfilePath != "" {
		if !filepath.IsAbs(f.SeccompProfilePath) || filepath.Clean(f.SeccompProfilePath) != f.SeccompProfilePath {
			return nil, fmt.Errorf("config: seccompProfilePath %q must be a clean absolute path", f.SeccompProfilePath)
		}
		c.SeccompProfilePath = f.SeccompProfilePath
	}
	if f.DevRoot != "" {
		if !filepath.IsAbs(f.DevRoot) || filepath.Clean(f.DevRoot) != f.DevRoot {
			return nil, fmt.Errorf("config: devRoot %q must be a clean absolute path", f.DevRoot)
		}
		c.DevRoot = f.DevRoot
	}
	return c, nil
}

// parseSwitch reads an off/on field: empty, "off" or a YAML 1.1 false
// (false/no/n/off/0) is off; onMode or a YAML 1.1 true (true/yes/y/on/1)
// is on. Any case. YAML 1.1 turns unquoted spellings into booleans and
// sigs.k8s.io/yaml stringifies them to "true"/"false", so both forms land
// here.
func parseSwitch(field, v, onMode string) (bool, error) {
	switch strings.ToLower(v) {
	case "", "off", "false", "no", "n", "0":
		return false, nil
	case onMode, "true", "yes", "y", "on", "1":
		return true, nil
	}
	return false, fmt.Errorf("config: %s %q must be %q or %q (or a YAML boolean: true/false, yes/no, y/n, on/off, 1/0)",
		field, v, onMode, "off")
}

// GetIOUring is a nil-safe accessor for the YAML seccomp block.
func (s *SeccompFile) GetIOUring() string {
	if s == nil {
		return ""
	}
	return s.IOUring
}

// parseRlimitValueForConfig checks an rlimit value the same way the grant
// table does, so a bad config fails at start not at create.
func parseRlimitValueForConfig(s string) (uint64, error) {
	return latency.ParseRlimitValue(s)
}

// Load reads and checks the config file at path.
func Load(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("config: %w", err)
	}
	return Parse(data)
}

// Capacity is the number of millicpu device IDs the node offers.
func (c *Config) Capacity() int { return c.Shared.Len() * 1000 * c.Oversubscription }

// NamespaceAllowed reports whether pods of ns may opt in.
func (c *Config) NamespaceAllowed(ns string) bool {
	for _, n := range c.Namespaces {
		if n == ns {
			return true
		}
	}
	return false
}

// CheckHost checks the node: the shared CPUs are online and not isolated,
// the kernel is 6.2 or later (it keeps a thread's requested affinity across
// cpuset changes) and cgroup v2 is mounted. sysRoot is usually /sys and
// cgroupRoot /sys/fs/cgroup. It returns warnings that do not stop the plugin.
func CheckHost(c *Config, sysRoot, cgroupRoot, kernelRelease string) ([]string, error) {
	var warnings []string
	cpuDir := filepath.Join(sysRoot, "devices/system/cpu")
	online, err := readCPUFile(filepath.Join(cpuDir, "online"))
	if err != nil {
		return nil, fmt.Errorf("host: %w", err)
	}
	if missing := c.Shared.Difference(online); !missing.IsEmpty() {
		return nil, fmt.Errorf("host: sharedCpus %s: CPU %s is not online (online: %s)", c.Shared, missing, online)
	}
	isolated, err := readCPUFile(filepath.Join(cpuDir, "isolated"))
	if err != nil {
		return nil, fmt.Errorf("host: %w", err)
	}
	if both := c.Shared.Intersection(isolated); !both.IsEmpty() {
		return nil, fmt.Errorf("host: sharedCpus %s: CPU %s is isolated (isolcpus), so helpers there would not load-balance", c.Shared, both)
	}
	nohz, err := readCPUFile(filepath.Join(cpuDir, "nohz_full"))
	if err != nil {
		return nil, fmt.Errorf("host: %w", err)
	}
	if both := c.Shared.Intersection(nohz); !both.IsEmpty() {
		warnings = append(warnings, fmt.Sprintf("sharedCpus: CPU %s is in nohz_full; housekeeping CPUs usually are not", both))
	}
	ok, err := kernelAtLeast(kernelRelease, 6, 2)
	if err != nil {
		return nil, fmt.Errorf("host: %w", err)
	}
	if !ok {
		return nil, fmt.Errorf("host: kernel %s is older than 6.2: it resets thread affinity on every cpuset change", kernelRelease)
	}
	if _, err := os.Stat(filepath.Join(cgroupRoot, "cgroup.controllers")); err != nil {
		return nil, fmt.Errorf("host: cgroup v2 is needed (%s/cgroup.controllers: %w)", cgroupRoot, err)
	}
	// Partition mode writes cpuset.cpus.exclusive down the hierarchy, and that
	// file only exists from Linux 6.7 or a vendor kernel that backported it.
	// A warning, not an error: shared-only pods work without it, and the
	// filesystem is the authority rather than this node's version number.
	if present, probed := cpuset.HasExclusive(cgroupRoot, cgroupRoot); probed && !present {
		warnings = append(warnings, fmt.Sprintf(
			"kernel %s has no %s: every cpu-partition opt-in will be refused (a remote partition needs Linux 6.7 or a vendor backport of it)",
			kernelRelease, cpuset.ExclusiveFile))
	}
	if len(c.Namespaces) == 0 {
		warnings = append(warnings, "namespaces is empty: every opt-in will be refused")
	}
	return warnings, nil
}

// readCPUFile reads a sysfs CPU list. A missing file, an empty file and the
// kernel's "(null)" are the empty set.
func readCPUFile(path string) (cpuset.Set, error) {
	data, err := os.ReadFile(path)
	if errors.Is(err, fs.ErrNotExist) {
		return cpuset.Set{}, nil
	}
	if err != nil {
		return cpuset.Set{}, err
	}
	text := strings.TrimSpace(string(data))
	if text == "" || text == "(null)" {
		return cpuset.Set{}, nil
	}
	s, err := cpuset.Parse(text)
	if err != nil {
		return cpuset.Set{}, fmt.Errorf("%s: %w", path, err)
	}
	return s, nil
}

func kernelAtLeast(release string, major, minor int) (bool, error) {
	parts := strings.SplitN(release, ".", 3)
	if len(parts) < 2 {
		return false, fmt.Errorf("kernel release %q does not parse", release)
	}
	maj, err1 := strconv.Atoi(parts[0])
	minText := parts[1]
	if i := strings.IndexFunc(minText, func(r rune) bool { return r < '0' || r > '9' }); i >= 0 {
		minText = minText[:i]
	}
	mnr, err2 := strconv.Atoi(minText)
	if err1 != nil || err2 != nil {
		return false, fmt.Errorf("kernel release %q does not parse", release)
	}
	return maj > major || (maj == major && mnr >= minor), nil
}

// HostDevicePath maps a granted container device path (/dev/ptp0) to where this
// process can see the host node. DevRoot is usually /dev; the DaemonSet mounts
// the host's /dev at /host/dev so the plugin can stat devices the container
// image does not carry. Granted paths and the env contract stay the container's
// /dev/... names.
func HostDevicePath(devRoot, containerPath string) string {
	if devRoot == "" || devRoot == DefaultDevRoot {
		return containerPath
	}
	return filepath.Join(devRoot, filepath.Base(containerPath))
}

// DiscoverHostDevices fills in the node-fact fields the device grants need:
// every /dev/ptp* the node has, /dev/cpu_dma_latency when present, and GOARCH.
// Granted names are the container's /dev/... paths; the plugin stats them under
// DevRoot. A missing dev root fails closed: the plugin cannot grant devices it
// cannot see.
func DiscoverHostDevices(c *Config, goarch string) error {
	c.GOARCH = goarch
	entries, err := os.ReadDir(c.DevRoot)
	if err != nil {
		return fmt.Errorf("host devices: %s: %w", c.DevRoot, err)
	}
	var ptp []string
	for _, e := range entries {
		name := e.Name()
		if strings.HasPrefix(name, "ptp") {
			ptp = append(ptp, "/dev/"+name)
		}
	}
	sort.Strings(ptp)
	c.HostPtp = ptp
	if _, err := os.Stat(filepath.Join(c.DevRoot, "cpu_dma_latency")); err == nil {
		c.HostCpuDmaLatency = "/dev/cpu_dma_latency"
	} else {
		c.HostCpuDmaLatency = ""
	}
	return nil
}

// DiscoverHostNuma fills in NodeNumaNodes: every node<N> under
// sysRoot/devices/system/node. That list is what a numa.ergon.io request is
// checked against, so it is never left holding "we never looked". A kernel
// built without CONFIG_NUMA has no node directory and still has one node,
// which cpuset.mems names 0, so the list is [0]. A directory that exists but
// names no node is an error: the plugin would otherwise answer a binding
// request from a list it never saw.
func DiscoverHostNuma(c *Config, sysRoot string) error {
	dir := filepath.Join(sysRoot, "devices/system/node")
	entries, err := os.ReadDir(dir)
	if errors.Is(err, fs.ErrNotExist) {
		c.NodeNumaNodes = []int{0}
		return nil
	}
	if err != nil {
		return fmt.Errorf("host NUMA: %s: %w", dir, err)
	}
	var nodes []int
	for _, e := range entries {
		name := e.Name()
		if !strings.HasPrefix(name, "node") {
			continue
		}
		n, convErr := strconv.Atoi(strings.TrimPrefix(name, "node"))
		if convErr != nil || n < 0 {
			continue
		}
		nodes = append(nodes, n)
	}
	if len(nodes) == 0 {
		return fmt.Errorf("host NUMA: %s names no node<N>: numa.ergon.io cannot be answered", dir)
	}
	sort.Ints(nodes)
	c.NodeNumaNodes = nodes
	return nil
}
