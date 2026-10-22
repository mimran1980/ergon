// SPDX-License-Identifier: Apache-2.0

package main

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestLoadConfigProfiles(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "config.yaml")
	body := `
nodeProfileLabel: ergon.io/node-profile
profiles:
  small:
    sharedCpus: "0"
    namespaces: [trading]
  wide:
    sharedCpus: "0,1"
    namespaces: [trading]
`
	if err := os.WriteFile(path, []byte(body), 0o644); err != nil {
		t.Fatal(err)
	}
	calls := 0
	getter := func(_ context.Context, name string) (map[string]string, error) {
		calls++
		if name != "node-a" {
			t.Errorf("node name %q", name)
		}
		return map[string]string{"ergon.io/node-profile": "wide"}, nil
	}
	cfg, err := loadConfig(path, "node-a", getter)
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Shared.List() != "0,1" || calls != 1 {
		t.Fatalf("wide via startup load: %+v calls=%d", cfg.Shared.List(), calls)
	}
	_, err = loadConfig(path, "", getter)
	if err == nil || !strings.Contains(err.Error(), "NODE_NAME is empty") {
		t.Fatalf("empty NODE_NAME: %v", err)
	}
	_, err = loadConfig(path, "node-a", func(context.Context, string) (map[string]string, error) {
		return map[string]string{}, nil
	})
	if err == nil || !strings.Contains(err.Error(), "missing node profile label") {
		t.Fatalf("missing label: %v", err)
	}
	t.Log("missing node profile label")
	_, err = loadConfig(path, "node-a", func(context.Context, string) (map[string]string, error) {
		return map[string]string{"ergon.io/node-profile": "no-such"}, nil
	})
	if err == nil || !strings.Contains(err.Error(), "unknown profile") {
		t.Fatalf("unknown profile: %v", err)
	}
	t.Log("unknown profile")
	_, err = loadConfig(path, "node-a", func(context.Context, string) (map[string]string, error) {
		return nil, errors.New("connection refused")
	})
	if err == nil || !strings.Contains(err.Error(), "node API unavailable") {
		t.Fatalf("api down: %v", err)
	}
	t.Log("node API unavailable")

	legacy := filepath.Join(dir, "legacy.yaml")
	if err := os.WriteFile(legacy, []byte("sharedCpus: \"4\"\nnamespaces: [trading]\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	before := calls
	cfg, err = loadConfig(legacy, "", func(context.Context, string) (map[string]string, error) {
		t.Error("legacy load must not GET the node")
		return nil, errors.New("called")
	})
	if err != nil || cfg.Shared.List() != "4" || calls != before {
		t.Fatalf("legacy: %v %v calls=%d", err, cfg, calls)
	}
}
