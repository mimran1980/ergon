// SPDX-License-Identifier: Apache-2.0

package config

import (
	"context"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

const twoProfiles = `
nodeProfileLabel: ergon.io/node-profile
profiles:
  small:
    sharedCpus: "0"
    namespaces: [trading]
  wide:
    sharedCpus: "0,1"
    namespaces: [trading]
    oversubscription: 1
    seccomp:
      ioUring: "off"
    schedFifo: "off"
    ptp: "off"
    rlimits:
      memlock: "inf"
`

func TestResolveSelectsOneProfile(t *testing.T) {
	wide, err := Resolve([]byte(twoProfiles), "wide")
	if err != nil {
		t.Fatal(err)
	}
	if wide.Shared.List() != "0,1" || wide.Rlimits.Memlock != "inf" || wide.SeccompIOUring || wide.Ptp != "off" || wide.SchedFifo != SchedFifoOff {
		t.Fatalf("wide profile: %+v", wide)
	}
	small, err := Resolve([]byte(twoProfiles), "small")
	if err != nil {
		t.Fatal(err)
	}
	if small.Shared.List() != "0" || small.Rlimits.Memlock != "" {
		t.Fatalf("small profile: %+v", small)
	}
	if _, err := Parse([]byte(twoProfiles)); err == nil || !strings.Contains(err.Error(), "profiles need Resolve") {
		t.Fatalf("Parse of a profiles file = %v", err)
	}
	_, err = Resolve([]byte(twoProfiles), "missing")
	if err == nil || !strings.Contains(err.Error(), "unknown profile") {
		t.Fatalf("unknown profile: %v", err)
	}
	t.Log("unknown profile")
}

func TestProfileModeRejectsLegacyFields(t *testing.T) {
	both := "sharedCpus: \"0\"\nnodeProfileLabel: ergon.io/node-profile\nprofiles:\n  small:\n    sharedCpus: \"0\"\n    namespaces: [trading]\n"
	_, err := Parse([]byte(both))
	if err == nil || !strings.Contains(err.Error(), "sharedCpus and profiles are both set") {
		t.Fatalf("ambiguous mode: %v", err)
	}
	_, err = Resolve([]byte(both), "small")
	if err == nil || !strings.Contains(err.Error(), "use one mode") {
		t.Fatalf("Resolve ambiguous: %v", err)
	}
	legacy := "sharedCpus: \"4\"\nnamespaces: [trading]\n"
	cfg, err := Select([]byte(legacy), "", nil)
	if err != nil || cfg.Shared.List() != "4" {
		t.Fatalf("legacy select: %v %+v", err, cfg)
	}
}

func TestSelectFromNodeLabels(t *testing.T) {
	data := []byte(twoProfiles)
	_, err := Select(data, "", map[string]string{"ergon.io/node-profile": "wide"})
	if err == nil || !strings.Contains(err.Error(), "NODE_NAME is empty") {
		t.Fatalf("empty node name: %v", err)
	}
	_, err = Select(data, "node-a", map[string]string{})
	if err == nil || !strings.Contains(err.Error(), "missing node profile label") {
		t.Fatalf("missing label: %v", err)
	}
	t.Log("missing node profile label")
	cfg, err := Select(data, "node-a", map[string]string{"ergon.io/node-profile": "small"})
	if err != nil || cfg.Shared.List() != "0" {
		t.Fatalf("select small: %v %+v", err, cfg)
	}
	_, err = SelectFromGetter(context.Background(), data, "node-a", func(context.Context, string) (map[string]string, error) {
		return nil, context.DeadlineExceeded
	})
	if err == nil || !strings.Contains(err.Error(), "node API unavailable") {
		t.Fatalf("getter down: %v", err)
	}
	t.Log("node API unavailable")
}

func TestFetchNodeLabels(t *testing.T) {
	srv := httptest.NewTLSServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/v1/nodes/node-a" {
			http.NotFound(w, r)
			return
		}
		if !strings.HasPrefix(r.Header.Get("Authorization"), "Bearer ") {
			http.Error(w, "no token", http.StatusUnauthorized)
			return
		}
		if r.Method != http.MethodGet {
			http.Error(w, "method", http.StatusMethodNotAllowed)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"metadata":{"name":"node-a","labels":{"ergon.io/node-profile":"wide","other":"x"}}}`))
	}))
	defer srv.Close()
	labels, err := fetchNodeLabels(context.Background(), srv.Client(), srv.URL, "token", "node-a")
	if err != nil {
		t.Fatal(err)
	}
	cfg, err := Select([]byte(twoProfiles), "node-a", labels)
	if err != nil || cfg.Shared.List() != "0,1" {
		t.Fatalf("fetched profile: %v %+v", err, cfg)
	}
	down := httptest.NewTLSServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "down", http.StatusServiceUnavailable)
	}))
	defer down.Close()
	_, err = fetchNodeLabels(context.Background(), down.Client(), down.URL, "token", "node-a")
	if err == nil || !strings.Contains(err.Error(), "node API unavailable") {
		t.Fatalf("status: %v", err)
	}
	t.Log("node API unavailable")
}

func TestInClusterNodeLabelsNeedsAddress(t *testing.T) {
	t.Setenv("KUBERNETES_SERVICE_HOST", "")
	t.Setenv("KUBERNETES_SERVICE_PORT", "")
	_, err := InClusterNodeLabels(context.Background(), "node-a")
	if err == nil || !strings.Contains(err.Error(), "node API unavailable") {
		t.Fatal(err)
	}
	t.Log("node API unavailable")
}
