// SPDX-License-Identifier: Apache-2.0

package nriplugin

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

// admissionPolicyPath finds the shipped policy. On the host it is the
// sibling module. scripts/check.sh mounts that directory at /admission
// because the unit container only has this module.
func admissionPolicyPath() (string, error) {
	_, here, _, _ := runtime.Caller(0)
	dir := filepath.Dir(here)
	var root string
	for {
		if _, err := os.Stat(filepath.Join(dir, "go.mod")); err == nil {
			root = dir
			break
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}
	candidates := []string{
		"/admission/validating-admission-policy.yaml",
		filepath.Join(root, "..", "nri-latency-pods", "admission", "validating-admission-policy.yaml"),
	}
	for _, p := range candidates {
		if _, err := os.Stat(p); err == nil {
			return p, nil
		}
	}
	return "", os.ErrNotExist
}

func chartAdmissionPolicy() string {
	_, here, _, _ := runtime.Caller(0)
	dir := filepath.Dir(here)
	for {
		if _, err := os.Stat(filepath.Join(dir, "go.mod")); err == nil {
			p := filepath.Join(dir, "charts", "ergon-shared-cpus", "files", "validating-admission-policy.yaml")
			if _, err := os.Stat(p); err == nil {
				return p
			}
			return ""
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return ""
		}
		dir = parent
	}
}

// TestShippedAdmissionPolicy is the unit stand-in for the kind admission
// lane. It checks the policy that ships, including the exact plugin token
// the Go matcher accepts.
func TestShippedAdmissionPolicy(t *testing.T) {
	path, err := admissionPolicyPath()
	if err != nil {
		t.Fatal("validating-admission-policy.yaml not found beside the module or at /admission")
	}
	body, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	text := string(body)
	for _, needle := range []string{
		"livenessProbe",
		"readinessProbe",
		"startupProbe",
		"lifecycle",
		"postStart",
		"preStop",
		"object.spec.containers.all",
		"ergon.io/shared-millicpus",
		`matches('(^|\\[|,)\\s*"ergon-shared-cpus"\\s*(,|\\]|$)')`,
		"quantity(c.resources.requests['cpu']).isInteger()",
		"quantity(c.resources.requests['cpu']).compareTo(quantity(c.resources.limits['cpu']))",
		"quantity(c.resources.requests[k]).compareTo(quantity(c.resources.limits[k]))",
		"resizePolicy",
		"RestartContainer",
		"kind: ValidatingAdmissionPolicyBinding",
		"priorityClassName",
	} {
		if !strings.Contains(text, needle) {
			t.Errorf("policy lacks %q", needle)
		}
	}
	if strings.Contains(text, ".contains('ergon-shared-cpus')") {
		t.Error("policy still uses a substring test for the plugin name")
	}
	if strings.Contains(text, "containers.exists") {
		t.Error("policy still accepts the shared resource on any one container")
	}
	if strings.Contains(text, "requests['cpu'].isInteger()") || strings.Contains(text, "requests['cpu'].compareTo(") {
		t.Error("policy calls a quantity method on a resource string; wrap it with quantity()")
	}
	chart := chartAdmissionPolicy()
	if chart == "" {
		t.Fatal("chart validating-admission-policy.yaml not found")
	}
	chartBody, err := os.ReadFile(chart)
	if err != nil {
		t.Fatal(err)
	}
	if string(chartBody) != text {
		t.Error("chart policy and k8s/nri-latency-pods admission policy differ")
	}
	if !names.RequiredPluginListed(`["ergon-shared-cpus"]`) {
		t.Fatal("exact plugin token was not accepted")
	}
	if names.RequiredPluginListed(`["ergon-shared-cpus-extra"]`) || names.RequiredPluginListed(`["not-ergon-shared-cpus"]`) {
		t.Fatal("a plugin-name substring was accepted")
	}
	t.Log("startup, readiness and liveness exec probes and a lifecycle exec handler are in the shipped policy")
	t.Log("ordinary pods are outside the hot match; valid hot pods keep a whole-number cpu and RestartContainer")
}
