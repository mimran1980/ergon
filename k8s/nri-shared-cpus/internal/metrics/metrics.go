// SPDX-License-Identifier: Apache-2.0

// Package metrics keeps the plugin's counters and serves /metrics (Prometheus
// text format, written by hand), /healthz and /readyz.
package metrics

import (
	"fmt"
	"io"
	"net/http"
	"slices"
	"strings"
	"sync"
)

// Metrics is safe for concurrent use. The zero value is not usable; use New.
type Metrics struct {
	mu           sync.Mutex
	nriConnected bool
	registered   bool
	containers   int
	load         map[int]int
	refused      map[string]uint64
	rewidened    map[string]uint64
	unhandled    uint64
	staleEnv     uint64
	stalePart    uint64
	ineligible   uint64
	grants       map[string]uint64
}

// New returns zeroed metrics.
func New() *Metrics {
	return &Metrics{load: map[int]int{}, refused: map[string]uint64{}, rewidened: map[string]uint64{"update": 0, "sync": 0}, grants: map[string]uint64{}}
}

// SetNRIConnected sets the NRI connection gauge.
func (m *Metrics) SetNRIConnected(v bool) { m.mu.Lock(); m.nriConnected = v; m.mu.Unlock() }

// SetRegistered sets the device plugin registration gauge.
func (m *Metrics) SetRegistered(v bool) { m.mu.Lock(); m.registered = v; m.mu.Unlock() }

// Ready reports whether NRI is connected and the device plugin registered.
func (m *Metrics) Ready() bool {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.nriConnected && m.registered
}

// SetTable sets the container count and the per-CPU load.
func (m *Metrics) SetTable(containers int, load map[int]int) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.containers = containers
	m.load = map[int]int{}
	for c, l := range load {
		m.load[c] = l
	}
}

// Refused counts a refused container by reason.
func (m *Metrics) Refused(reason string) { m.mu.Lock(); m.refused[reason]++; m.mu.Unlock() }

// Rewidened counts a re-widen by event ("update" or "sync").
func (m *Metrics) Rewidened(event string) { m.mu.Lock(); m.rewidened[event]++; m.mu.Unlock() }

// Unhandled counts a container that asked for shared CPUs but was created
// without the plugin.
func (m *Metrics) Unhandled() { m.mu.Lock(); m.unhandled++; m.mu.Unlock() }

// StaleEnv counts a cpuset change that made a container's env stale.
func (m *Metrics) StaleEnv() { m.mu.Lock(); m.staleEnv++; m.mu.Unlock() }

// StalePartition counts a cpuset change that made a container's cpu-partition
// hooks name the wrong CPUs. NRI cannot re-run a createRuntime hook, so the
// container has to be recreated.
func (m *Metrics) StalePartition() { m.mu.Lock(); m.stalePart++; m.mu.Unlock() }

// Ineligible counts a container that is no longer an exclusive integer-CPU
// container (quota or shares) and had its shared CPUs dropped.
func (m *Metrics) Ineligible() { m.mu.Lock(); m.ineligible++; m.mu.Unlock() }

// Grant counts a latency-pod grant by kind ("rlimit", "seccomp", "numa",
// "ptp", "fifo", "hugepages") and value. The Prometheus name is
// ergon_shared_cpus_grants_total{kind,value}.
func (m *Metrics) Grant(kind, value string) {
	m.mu.Lock()
	m.grants[kind+"="+value]++
	m.mu.Unlock()
}

// Counter returns one counter's value, for tests: "refused/<reason>",
// "rewidened/<event>", "unhandled", "stale_env" or "ineligible".
func (m *Metrics) Counter(name string) uint64 {
	m.mu.Lock()
	defer m.mu.Unlock()
	switch {
	case name == "unhandled":
		return m.unhandled
	case name == "stale_env":
		return m.staleEnv
	case name == "stale_partition":
		return m.stalePart
	case name == "ineligible":
		return m.ineligible
	case len(name) > 8 && name[:8] == "refused/":
		return m.refused[name[8:]]
	case len(name) > 10 && name[:10] == "rewidened/":
		return m.rewidened[name[10:]]
	case len(name) > 6 && name[:6] == "grant/":
		return m.grants[name[6:]]
	}
	return 0
}

func b2i(b bool) int {
	if b {
		return 1
	}
	return 0
}

// Write writes all metrics in the Prometheus text format.
func (m *Metrics) Write(w io.Writer) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	var err error
	p := func(format string, args ...any) {
		if err == nil {
			_, err = fmt.Fprintf(w, format, args...)
		}
	}
	p("# HELP ergon_shared_cpus_nri_connected 1 while connected to the runtime's NRI socket.\n")
	p("# TYPE ergon_shared_cpus_nri_connected gauge\nergon_shared_cpus_nri_connected %d\n", b2i(m.nriConnected))
	p("# HELP ergon_shared_cpus_device_plugin_registered 1 while registered with the kubelet.\n")
	p("# TYPE ergon_shared_cpus_device_plugin_registered gauge\nergon_shared_cpus_device_plugin_registered %d\n", b2i(m.registered))
	p("# HELP ergon_shared_cpus_containers Containers holding shared CPUs.\n")
	p("# TYPE ergon_shared_cpus_containers gauge\nergon_shared_cpus_containers %d\n", m.containers)
	p("# HELP ergon_shared_cpus_cpu_load_millicpus Millicpus asked for on each shared CPU.\n")
	p("# TYPE ergon_shared_cpus_cpu_load_millicpus gauge\n")
	cpus := make([]int, 0, len(m.load))
	for c := range m.load {
		cpus = append(cpus, c)
	}
	slices.Sort(cpus)
	for _, c := range cpus {
		p("ergon_shared_cpus_cpu_load_millicpus{cpu=\"%d\"} %d\n", c, m.load[c])
	}
	p("# HELP ergon_shared_cpus_refused_total Containers refused at create, by reason.\n")
	p("# TYPE ergon_shared_cpus_refused_total counter\n")
	for _, r := range sortedKeys(m.refused) {
		p("ergon_shared_cpus_refused_total{reason=%q} %d\n", r, m.refused[r])
	}
	p("# HELP ergon_shared_cpus_rewidened_total Cpusets widened again, by event.\n")
	p("# TYPE ergon_shared_cpus_rewidened_total counter\n")
	for _, e := range sortedKeys(m.rewidened) {
		p("ergon_shared_cpus_rewidened_total{event=%q} %d\n", e, m.rewidened[e])
	}
	p("# HELP ergon_shared_cpus_unhandled_total Containers that asked for shared CPUs but were created without the plugin.\n")
	p("# TYPE ergon_shared_cpus_unhandled_total counter\nergon_shared_cpus_unhandled_total %d\n", m.unhandled)
	p("# HELP ergon_shared_cpus_stale_env_total Cpuset changes that left a container's CPU env stale.\n")
	p("# TYPE ergon_shared_cpus_stale_env_total counter\nergon_shared_cpus_stale_env_total %d\n", m.staleEnv)
	p("# HELP ergon_shared_cpus_stale_partition_total Cpuset changes that left a container's cpu-partition hooks naming the wrong CPUs.\n")
	p("# TYPE ergon_shared_cpus_stale_partition_total counter\nergon_shared_cpus_stale_partition_total %d\n", m.stalePart)
	p("# HELP ergon_shared_cpus_ineligible_total Containers no longer exclusive integer-CPU, shared CPUs dropped.\n")
	p("# TYPE ergon_shared_cpus_ineligible_total counter\nergon_shared_cpus_ineligible_total %d\n", m.ineligible)
	p("# HELP ergon_shared_cpus_grants_total Latency-pod grants given, by kind and value.\n")
	p("# TYPE ergon_shared_cpus_grants_total counter\n")
	for _, k := range sortedKeys(m.grants) {
		kind, value, _ := strings.Cut(k, "=")
		p("ergon_shared_cpus_grants_total{kind=%q,value=%q} %d\n", kind, value, m.grants[k])
	}
	return err
}

func sortedKeys(m map[string]uint64) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	slices.Sort(keys)
	return keys
}

// Handler serves /metrics, /healthz and /readyz.
func (m *Metrics) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("/metrics", func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "text/plain; version=0.0.4")
		_ = m.Write(w)
	})
	mux.HandleFunc("/healthz", func(w http.ResponseWriter, _ *http.Request) {
		_, _ = io.WriteString(w, "ok\n")
	})
	mux.HandleFunc("/readyz", func(w http.ResponseWriter, _ *http.Request) {
		if !m.Ready() {
			http.Error(w, "not ready: NRI not connected or device plugin not registered", http.StatusServiceUnavailable)
			return
		}
		_, _ = io.WriteString(w, "ok\n")
	})
	return mux
}
