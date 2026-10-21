// SPDX-License-Identifier: Apache-2.0

package config

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestParseDefaults(t *testing.T) {
	c, err := Parse([]byte("sharedCpus: \"0,1\"\nnamespaces: [trading]\n"))
	if err != nil {
		t.Fatal(err)
	}
	if c.Shared.List() != "0,1" || c.Oversubscription != 1 || c.DefaultAffinity != AffinityShared ||
		c.HookPath != DefaultHookPath || c.AnnotationOptIn || c.MetricsAddress != DefaultMetricsAddress {
		t.Errorf("defaults: %+v", c)
	}
	if !c.NamespaceAllowed("trading") || c.NamespaceAllowed("default") {
		t.Error("namespace allowlist")
	}
	if c.Capacity() != 2000 {
		t.Errorf("capacity = %d", c.Capacity())
	}
}

func TestParseFull(t *testing.T) {
	c, err := Parse([]byte(`
sharedCpus: "0-1"
oversubscription: 50
namespaces: [ergon-e2e, trading]
defaultAffinity: all
hookPath: /opt/x/ergon-cpus
annotationOptIn: true
metricsAddress: "127.0.0.1:9000"
`))
	if err != nil {
		t.Fatal(err)
	}
	if c.Capacity() != 100000 || c.DefaultAffinity != AffinityAll || !c.AnnotationOptIn || c.HookPath != "/opt/x/ergon-cpus" {
		t.Errorf("config: %+v", c)
	}
}

func TestParseRejects(t *testing.T) {
	cases := []struct{ yaml, field string }{
		{"namespaces: [a]\n", "sharedCpus"},
		{"sharedCpus: \"\"\n", "sharedCpus"},
		{"sharedCpus: \"1-0\"\n", "sharedCpus"},
		{"sharedCpus: \"0\"\noversubscription: 0\n", "oversubscription"},
		{"sharedCpus: \"0\"\noversubscription: 101\n", "oversubscription"},
		{"sharedCpus: \"0-2\"\noversubscription: 34\n", "oversubscription"}, // 102,000 ids
		{"sharedCpus: \"0\"\nnamespaces: [Bad_Name]\n", "namespaces"},
		{"sharedCpus: \"0\"\nnamespaces: [\"\"]\n", "namespaces"},
		{"sharedCpus: \"0\"\ndefaultAffinity: some\n", "defaultAffinity"},
		{"sharedCpus: \"0\"\nhookPath: relative/ergon-cpus\n", "hookPath"},
		// The daemon overwrites hookPath as root; a typo must not clobber
		// an unrelated node file (e.g. /etc/passwd).
		{"sharedCpus: \"0\"\nhookPath: /etc/passwd\n", "hookPath"},
		{"sharedCpus: \"0\"\nunknownField: 1\n", "unknownField"},
		{"sharedCpus: \"0\"\nsharedCpus: \"1\"\n", "sharedCpus"},
		{"sharedCpus: [0]\n", "sharedCpus"},
	}
	for _, c := range cases {
		_, err := Parse([]byte(c.yaml))
		if err == nil {
			t.Errorf("accepted %q", c.yaml)
			continue
		}
		if !strings.Contains(err.Error(), c.field) {
			t.Errorf("%q: error %q does not name %s", c.yaml, err, c.field)
		}
	}
}

func fakeSys(t *testing.T, online, isolated, nohz string) string {
	t.Helper()
	root := t.TempDir()
	dir := filepath.Join(root, "devices/system/cpu")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	for name, text := range map[string]string{"online": online, "isolated": isolated, "nohz_full": nohz} {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(text), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return root
}

func fakeCgroup(t *testing.T, v2 bool) string {
	t.Helper()
	root := t.TempDir()
	if v2 {
		if err := os.WriteFile(filepath.Join(root, "cgroup.controllers"), []byte("cpuset cpu\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return root
}

func TestCheckHost(t *testing.T) {
	c, err := Parse([]byte("sharedCpus: \"0,1\"\nnamespaces: [a]\n"))
	if err != nil {
		t.Fatal(err)
	}
	good := fakeSys(t, "0-9\n", "\n", "(null)\n")
	if warn, err := CheckHost(c, good, fakeCgroup(t, true), "6.12.72-linuxkit"); err != nil || len(warn) != 0 {
		t.Errorf("good host: warn %v err %v", warn, err)
	}
	bad := []struct {
		name, sys, release string
		v2                 bool
		want               string
	}{
		{"offline cpu", fakeSys(t, "1-9\n", "\n", "\n"), "6.12.0", true, "not online"},
		{"isolated cpu", fakeSys(t, "0-9\n", "1\n", "\n"), "6.12.0", true, "isolated"},
		{"old kernel", good, "6.1.100", true, "6.2"},
		{"bad kernel string", good, "x", true, "kernel"},
		{"cgroup v1", good, "6.12.0", false, "cgroup v2"},
	}
	for _, b := range bad {
		_, err := CheckHost(c, b.sys, fakeCgroup(t, b.v2), b.release)
		if err == nil || !strings.Contains(err.Error(), b.want) {
			t.Errorf("%s: err = %v, want %q", b.name, err, b.want)
		}
	}
	warn, err := CheckHost(c, fakeSys(t, "0-9\n", "\n", "0-9\n"), fakeCgroup(t, true), "6.2.0")
	if err != nil || len(warn) != 1 || !strings.Contains(warn[0], "nohz_full") {
		t.Errorf("nohz_full: warn %v err %v", warn, err)
	}
	c2, err := Parse([]byte("sharedCpus: \"0\"\n"))
	if err != nil {
		t.Fatal(err)
	}
	warn, err = CheckHost(c2, good, fakeCgroup(t, true), "6.12.0")
	if err != nil || len(warn) != 1 || !strings.Contains(warn[0], "namespaces") {
		t.Errorf("empty allowlist: warn %v err %v", warn, err)
	}
}

// TestCheckHostWarnsWithoutTheExclusiveFeature: a kernel that provides no
// cpuset.cpus.exclusive cannot host a remote partition, so every cpu-partition
// opt-in is refused. That is a warning, not an error: shared-only pods work
// without it. The filesystem is the authority, not the version string — the
// same release is used below for both a kernel that has the file (a backport)
// and one that does not, so a version gate would get one of them wrong.
func TestCheckHostWarnsWithoutTheExclusiveFeature(t *testing.T) {
	c, err := Parse([]byte("sharedCpus: \"0,1\"\nnamespaces: [a]\n"))
	if err != nil {
		t.Fatal(err)
	}
	good := fakeSys(t, "0-9\n", "\n", "(null)\n")
	for _, tc := range []struct {
		name      string
		exclusive bool
		want      string
	}{
		{"backport provides the file", true, ""},
		{"no such file", false, "cpuset.cpus.exclusive"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			root := fakeCgroup(t, true)
			child := filepath.Join(root, "kubepods.slice")
			if err := os.MkdirAll(child, 0o755); err != nil {
				t.Fatal(err)
			}
			if err := os.WriteFile(filepath.Join(child, "cpuset.cpus"), []byte("\n"), 0o644); err != nil {
				t.Fatal(err)
			}
			if tc.exclusive {
				if err := os.WriteFile(filepath.Join(child, "cpuset.cpus.exclusive"), []byte("\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			}
			warn, err := CheckHost(c, good, root, "6.6.0")
			if err != nil {
				t.Fatalf("CheckHost: %v", err)
			}
			if tc.want == "" {
				if len(warn) != 0 {
					t.Errorf("warn = %v, want none when the file is there", warn)
				}
				return
			}
			found := false
			for _, w := range warn {
				if strings.Contains(w, tc.want) {
					found = true
				}
			}
			if !found {
				t.Errorf("warn = %v, want one mentioning %q", warn, tc.want)
			}
		})
	}
}

func TestKernelAtLeast(t *testing.T) {
	for release, want := range map[string]bool{
		"6.2.0": true, "6.12.72-linuxkit": true, "7.0": true, "6.1.99": false, "5.15.0-1": false, "6.18.1-amzn": true,
	} {
		got, err := kernelAtLeast(release, 6, 2)
		if err != nil || got != want {
			t.Errorf("kernelAtLeast(%q) = %v, %v", release, got, err)
		}
	}
}
