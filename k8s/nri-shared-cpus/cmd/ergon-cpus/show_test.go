// SPDX-License-Identifier: Apache-2.0

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestShowReportsQuotaAndAffinityRule(t *testing.T) {
	dir := t.TempDir()
	cpus := filepath.Join(dir, "cpuset.cpus.effective")
	if err := os.WriteFile(cpus, []byte("0,1,4,5\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "cpu.max"), []byte("max 100000\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	env := []string{
		"ERGON_ISOLATED_CPUS=1,5",
		"ERGON_SHARED_CPUS=0,4",
		"ERGON_ISOLATED_CORES=1,5",
		"ERGON_SHARED_MILLICPUS=2000",
	}
	out, err := buildShow(env, cpus)
	if err != nil {
		t.Fatal(err)
	}
	if out["cpu_max"] != "max 100000" {
		t.Fatalf("cpu_max = %v, want the cgroup quota text", out["cpu_max"])
	}
	rule, _ := out["affinity_rule"].(string)
	if !strings.Contains(rule, "inherits") || !strings.Contains(rule, "hot mask") {
		t.Fatalf("affinity_rule = %q", rule)
	}
	id, _ := out["config_identity"].(string)
	if id != "isolated=1,5 shared=0,4 milli=2000" {
		t.Fatalf("config_identity = %q", id)
	}
	threads, ok := out["threads"].([]map[string]string)
	if !ok || len(threads) == 0 || threads[0]["cpus_allowed"] == "" {
		t.Fatalf("threads = %#v", out["threads"])
	}
}
