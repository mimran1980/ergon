// SPDX-License-Identifier: Apache-2.0

package metrics

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestWriteAndReady(t *testing.T) {
	m := New()
	m.SetTable(2, map[int]int{1: 250, 0: 500})
	m.Refused("namespace")
	m.Refused("namespace")
	m.Rewidened("sync")
	m.Unhandled()
	m.StaleEnv()
	var b strings.Builder
	if err := m.Write(&b); err != nil {
		t.Fatal(err)
	}
	out := b.String()
	for _, want := range []string{
		"ergon_shared_cpus_nri_connected 0\n",
		"ergon_shared_cpus_containers 2\n",
		"ergon_shared_cpus_cpu_load_millicpus{cpu=\"0\"} 500\nergon_shared_cpus_cpu_load_millicpus{cpu=\"1\"} 250\n",
		"ergon_shared_cpus_refused_total{reason=\"namespace\"} 2\n",
		"ergon_shared_cpus_rewidened_total{event=\"sync\"} 1\n",
		"ergon_shared_cpus_rewidened_total{event=\"update\"} 0\n",
		"ergon_shared_cpus_unhandled_total 1\n",
		"ergon_shared_cpus_stale_env_total 1\n",
	} {
		if !strings.Contains(out, want) {
			t.Errorf("missing %q in\n%s", want, out)
		}
	}
	h := m.Handler()
	get := func(path string) int {
		rec := httptest.NewRecorder()
		h.ServeHTTP(rec, httptest.NewRequest(http.MethodGet, path, nil))
		return rec.Code
	}
	if get("/healthz") != 200 || get("/readyz") != 503 || get("/metrics") != 200 {
		t.Error("not ready yet")
	}
	m.SetNRIConnected(true)
	if get("/readyz") != 503 {
		t.Error("ready without the device plugin")
	}
	m.SetRegistered(true)
	if get("/readyz") != 200 {
		t.Error("not ready when both are up")
	}
}
