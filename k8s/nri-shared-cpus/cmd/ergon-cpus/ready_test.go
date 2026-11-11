// SPDX-License-Identifier: Apache-2.0

package main

import (
	"bytes"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

var readyEnv = []string{
	"ERGON_ISOLATED_CPUS=2,3",
	"ERGON_SHARED_CPUS=0",
	"ERGON_ISOLATED_CORES=2,3",
	"ERGON_SHARED_MILLICPUS=250",
}

func probeReady(t *testing.T, h http.Handler) (int, string) {
	t.Helper()
	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, httptest.NewRequest(http.MethodGet, "/placement", nil))
	return rec.Code, rec.Body.String()
}

func setCPUs(t *testing.T, path, text string) {
	t.Helper()
	if err := os.WriteFile(path, []byte(text), 0o644); err != nil {
		t.Fatal(err)
	}
}

// The kubelet rewrites the cpuset to the exclusive CPUs while the plugin is
// down: the probe must fail on the next request and pass again only once the
// shared CPU is back in cpuset.cpus.effective.
func TestReadyFollowsTheCpuset(t *testing.T) {
	cpus := filepath.Join(t.TempDir(), "cpuset.cpus.effective")
	setCPUs(t, cpus, "0,2-3\n")
	var log bytes.Buffer
	h := newReadiness(readyEnv, cpus, t.TempDir(), &log)

	if code, body := probeReady(t, h); code != http.StatusOK || !strings.Contains(body, "placement verified") {
		t.Fatalf("whole layout: %d %q, want 200 placement verified", code, body)
	}

	setCPUs(t, cpus, "2-3\n")
	code, body := probeReady(t, h)
	if code != http.StatusServiceUnavailable {
		t.Fatalf("shrunk cpuset: %d %q, want 503", code, body)
	}
	if !strings.Contains(body, "ERGON_SHARED_CPUS is 0, which is not in") {
		t.Fatalf("shrunk cpuset body %q does not name the missing shared CPU", body)
	}
	if code, _ := probeReady(t, h); code != http.StatusServiceUnavailable {
		t.Fatalf("still shrunk: %d, want 503", code)
	}

	setCPUs(t, cpus, "0,2-3\n")
	if code, body := probeReady(t, h); code != http.StatusOK {
		t.Fatalf("re-widened: %d %q, want 200", code, body)
	}

	// One log line per change of state, not per probe.
	lines := strings.Split(strings.TrimSpace(log.String()), "\n")
	if len(lines) != 3 {
		t.Fatalf("log has %d lines, want 3 (verified, broken, verified):\n%s", len(lines), log.String())
	}
	for i, want := range []string{"placement verified", "placement broken", "placement verified"} {
		if !strings.Contains(lines[i], want) {
			t.Fatalf("log line %d = %q, want %q", i, lines[i], want)
		}
	}
}

// A cpu-partition pod: a valid partition takes the exclusive CPUs out of the
// container's effective set. After an outage the cpuset can be whole again
// while the partition stays invalid (the CPUs are back, the isolation is
// not): that is not a verified placement.
func TestReadyNeedsAValidPartition(t *testing.T) {
	dir := t.TempDir()
	cpus := filepath.Join(dir, "cpuset.cpus.effective")
	child := filepath.Join(dir, names.CpuPartitionChild)
	if err := os.Mkdir(child, 0o755); err != nil {
		t.Fatal(err)
	}
	state := filepath.Join(child, "cpuset.cpus.partition")
	setCPUs(t, filepath.Join(child, "cpuset.cpus"), "2-3\n")
	setCPUs(t, cpus, "0\n")
	setCPUs(t, state, "isolated\n")
	var log bytes.Buffer
	h := newReadiness(readyEnv, cpus, t.TempDir(), &log)

	if code, body := probeReady(t, h); code != http.StatusOK {
		t.Fatalf("valid partition: %d %q, want 200", code, body)
	}

	setCPUs(t, cpus, "0,2-3\n")
	setCPUs(t, state, "isolated invalid (Parent unable to distribute cpu downstream)\n")
	code, body := probeReady(t, h)
	if code != http.StatusServiceUnavailable || !strings.Contains(body, "isolated invalid") {
		t.Fatalf("invalid partition: %d %q, want 503 naming the partition state", code, body)
	}

	if err := os.Remove(state); err != nil {
		t.Fatal(err)
	}
	if code, body := probeReady(t, h); code != http.StatusServiceUnavailable {
		t.Fatalf("unreadable partition state: %d %q, want 503", code, body)
	}
}

// Fail closed: a cpuset that cannot be read is not a verified placement.
func TestReadyFailsClosedWithoutTheCpuset(t *testing.T) {
	var log bytes.Buffer
	h := newReadiness(readyEnv, filepath.Join(t.TempDir(), "missing"), t.TempDir(), &log)
	if code, _ := probeReady(t, h); code != http.StatusServiceUnavailable {
		t.Fatalf("unreadable cpuset: %d, want 503", code)
	}
}

func TestReadyNeedsAnAddress(t *testing.T) {
	for _, args := range [][]string{{"ready"}, {"ready", "--addr"}, {"ready", "--bogus", ":1"}} {
		if rc := run(args, readyEnv, os.Stdin, os.Stdout, os.Stderr); rc != 2 {
			t.Fatalf("run(%q) = %d, want 2", args, rc)
		}
	}
}

func TestWrapReadyAddrNeedsAValue(t *testing.T) {
	if rc := run([]string{"--ready-addr"}, readyEnv, os.Stdin, os.Stdout, os.Stderr); rc != 2 {
		t.Fatalf("--ready-addr without a value = %d, want 2", rc)
	}
}
