// SPDX-License-Identifier: Apache-2.0

package config

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The fixture trees below follow what nodeadm writes on EKS AL2023: a JSON
// base at /etc/kubernetes/kubelet/config.json without the CPU manager
// settings, and the NodeConfig kubelet settings as the JSON drop-in
// config.json.d/40-nodeadm.conf. The kubelet reads only *.conf files, walks
// the directory in lexical order and applies each as a JSON merge patch.

// eksBase is a nodeadm-style base: no cpuManagerPolicy (the kubelet default
// is none), no reservedSystemCPUs, and the kube/system reserved cgroups set.
const eksBase = `{
    "kind": "KubeletConfiguration",
    "apiVersion": "kubelet.config.k8s.io/v1beta1",
    "cgroupDriver": "systemd",
    "kubeReservedCgroup": "/runtime",
    "systemReservedCgroup": "/system",
    "featureGates": {
        "RotateKubeletServerCertificate": true
    }
}
`

// nodeadmDropIn is the 40-nodeadm.conf this repo's userdata.sh produces:
// cpuManagerPolicy static, reservedSystemCPUs, the reserved cgroups emptied.
func nodeadmDropIn(reserved string) string {
	return `{
    "apiVersion": "kubelet.config.k8s.io/v1beta1",
    "kind": "KubeletConfiguration",
    "cpuManagerPolicy": "static",
    "cpuManagerPolicyOptions": {"full-pcpus-only": "true"},
    "reservedSystemCPUs": "` + reserved + `",
    "kubeReservedCgroup": "",
    "systemReservedCgroup": ""
}
`
}

// dropInFixture is the trading fixture with its kubelet file replaced by base
// and a drop-in directory holding files (name -> body). configz and the
// argv overrides are cleared: this test is about the file merge alone.
func dropInFixture(t *testing.T, base string, files map[string]string) fixture {
	t.Helper()
	fx := tradingFixture(t)
	fx.src.ConfigZ = ""
	fx.src.Overrides = ""
	if err := os.WriteFile(fx.src.KubeletConfig, []byte(base), 0o644); err != nil {
		t.Fatal(err)
	}
	dir := filepath.Join(fx.root, "config.json.d")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	for name, body := range files {
		p := filepath.Join(dir, name)
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(p, []byte(body), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	fx.src.KubeletConfigDir = dir
	return fx
}

func evalErr(t *testing.T, fx fixture) error {
	t.Helper()
	rep, err := Evaluate(fx.src, mustShared(t, "0-1"))
	if err != nil {
		t.Fatalf("evaluate: %v", err)
	}
	return rep.Err()
}

func wantRefusal(t *testing.T, err error, substr string) {
	t.Helper()
	if err == nil || !strings.Contains(err.Error(), substr) {
		t.Fatalf("want a refusal containing %q, got %v", substr, err)
	}
}

func TestKubeletDropInBaseOnly(t *testing.T) {
	// The base alone (an empty drop-in directory): the base decides.
	fx := dropInFixture(t, staticKubelet("0-2", "true"), nil)
	if err := evalErr(t, fx); err != nil {
		t.Fatalf("static base with an empty drop-in dir must pass: %v", err)
	}
	fx = dropInFixture(t, eksBase, nil)
	wantRefusal(t, evalErr(t, fx), `cpuManagerPolicy is "", want static`)
}

func TestKubeletDropInOverridesBase(t *testing.T) {
	// The EKS shape: base has no CPU manager settings, the drop-in sets them.
	fx := dropInFixture(t, eksBase, map[string]string{"40-nodeadm.conf": nodeadmDropIn("0-2")})
	if err := evalErr(t, fx); err != nil {
		t.Fatalf("nodeadm drop-in must supply cpuManagerPolicy and reservedSystemCPUs: %v", err)
	}
	// A drop-in that moves reservedSystemCPUs away from the helpers refuses.
	fx = dropInFixture(t, eksBase, map[string]string{"40-nodeadm.conf": nodeadmDropIn("2")})
	wantRefusal(t, evalErr(t, fx), "reserved-set mismatch")
	// A drop-in overrides a static base back to none.
	fx = dropInFixture(t, staticKubelet("0-2", "true"), map[string]string{
		"50-none.conf": "apiVersion: kubelet.config.k8s.io/v1beta1\nkind: KubeletConfiguration\ncpuManagerPolicy: none\n",
	})
	wantRefusal(t, evalErr(t, fx), `cpuManagerPolicy is "none", want static`)
	// Maps merge key by key: the base gate survives a drop-in that sets
	// another gate, and a drop-in can turn the checked gate off.
	fx = dropInFixture(t, staticKubelet("0-2", "true"), map[string]string{
		"50-gates.conf": "apiVersion: kubelet.config.k8s.io/v1beta1\nkind: KubeletConfiguration\nfeatureGates:\n  KubeletTracing: true\n",
	})
	if err := evalErr(t, fx); err != nil {
		t.Fatalf("an unrelated gate must not drop the base gate: %v", err)
	}
	fx = dropInFixture(t, staticKubelet("0-2", "true"), map[string]string{
		"50-gates.conf": "apiVersion: kubelet.config.k8s.io/v1beta1\nkind: KubeletConfiguration\nfeatureGates:\n  DisableCPUQuotaWithExclusiveCPUs: false\n",
	})
	wantRefusal(t, evalErr(t, fx), "DisableCPUQuotaWithExclusiveCPUs is false")
	// null removes a key (RFC 7386), so the kubelet default applies.
	fx = dropInFixture(t, staticKubelet("0-2", "true"), map[string]string{
		"50-null.conf": `{"apiVersion":"kubelet.config.k8s.io/v1beta1","kind":"KubeletConfiguration","cpuManagerPolicy":null}`,
	})
	wantRefusal(t, evalErr(t, fx), `cpuManagerPolicy is "", want static`)
}

func TestKubeletDropInLexicalOrder(t *testing.T) {
	none := "apiVersion: kubelet.config.k8s.io/v1beta1\nkind: KubeletConfiguration\ncpuManagerPolicy: none\n"
	// Later file wins: 40 sets static, 90 sets none.
	fx := dropInFixture(t, eksBase, map[string]string{
		"40-nodeadm.conf": nodeadmDropIn("0-2"),
		"90-later.conf":   none,
	})
	wantRefusal(t, evalErr(t, fx), `cpuManagerPolicy is "none", want static`)
	// Swapped names: none first, static last.
	fx = dropInFixture(t, eksBase, map[string]string{
		"10-early.conf":   none,
		"40-nodeadm.conf": nodeadmDropIn("0-2"),
	})
	if err := evalErr(t, fx); err != nil {
		t.Fatalf("the last drop-in in lexical order must win: %v", err)
	}
	// The kubelet walks subdirectories too, in lexical path order:
	// 50-sub/x.conf comes after 40-nodeadm.conf and before 60-z.conf.
	fx = dropInFixture(t, eksBase, map[string]string{
		"40-nodeadm.conf": nodeadmDropIn("0-2"),
		"50-sub/x.conf":   none,
	})
	wantRefusal(t, evalErr(t, fx), `cpuManagerPolicy is "none", want static`)
	fx = dropInFixture(t, eksBase, map[string]string{
		"40-nodeadm.conf": none,
		"50-sub/x.conf":   none,
		"60-z.conf":       nodeadmDropIn("0-2"),
	})
	if err := evalErr(t, fx); err != nil {
		t.Fatalf("60-z.conf comes after 50-sub/x.conf and must win: %v", err)
	}
}

// The kubelet decodes its --config file case-sensitively (apimachinery's
// UnmarshalCaseSensitivePreserveInts), so a mis-cased key there is an
// unknown field it ignores. Drop-ins are kept as raw maps, merged, and the
// result is decoded with encoding/json, which matches keys ignoring case.
func TestKubeletMiscasedKeys(t *testing.T) {
	miscased := "apiVersion: kubelet.config.k8s.io/v1beta1\nkind: KubeletConfiguration\nCPUManagerPolicy: static\nreservedSystemCPUs: \"0-2\"\n"
	// Base only, no drop-in directory: the kubelet runs with policy none.
	fx := tradingFixture(t)
	fx.src.ConfigZ = ""
	fx.src.Overrides = ""
	if err := os.WriteFile(fx.src.KubeletConfig, []byte(miscased), 0o644); err != nil {
		t.Fatal(err)
	}
	wantRefusal(t, evalErr(t, fx), `cpuManagerPolicy is "", want static`)
	// The same base under a drop-in directory.
	fx = dropInFixture(t, miscased, nil)
	wantRefusal(t, evalErr(t, fx), `cpuManagerPolicy is "", want static`)
	// A mis-cased key in a drop-in is honoured, as the kubelet honours it.
	fx = dropInFixture(t, eksBase, map[string]string{
		"40-nodeadm.conf": "apiVersion: kubelet.config.k8s.io/v1beta1\nkind: KubeletConfiguration\nCpuManagerPolicy: static\nreservedSystemCPUs: \"0-2\"\n",
	})
	if err := evalErr(t, fx); err != nil {
		t.Fatalf("a mis-cased drop-in key reaches the kubelet: %v", err)
	}
}

func TestKubeletDropInSkipsOtherSuffixes(t *testing.T) {
	// The kubelet reads only *.conf. A .json, .yaml, .conf.bak or a file with
	// no suffix is skipped, even when it would refuse.
	none := "apiVersion: kubelet.config.k8s.io/v1beta1\nkind: KubeletConfiguration\ncpuManagerPolicy: none\n"
	fx := dropInFixture(t, eksBase, map[string]string{
		"40-nodeadm.conf":     nodeadmDropIn("0-2"),
		"90-ignored.json":     none,
		"90-ignored.yaml":     none,
		"90-ignored.conf.bak": none,
		"README":              "not a config",
	})
	if err := evalErr(t, fx); err != nil {
		t.Fatalf("non-.conf files must be skipped as the kubelet skips them: %v", err)
	}
}

func TestKubeletDropInMalformedRefused(t *testing.T) {
	cases := map[string]string{
		"not yaml":           "cpuManagerPolicy: [static\n",
		"not an object":      "- static\n",
		"empty":              "",
		"no apiVersion/kind": "cpuManagerPolicy: static\n",
		"wrong kind":         "apiVersion: kubelet.config.k8s.io/v1beta1\nkind: KubeProxyConfiguration\n",
		"wrong apiVersion":   "apiVersion: kubelet.config.k8s.io/v1\nkind: KubeletConfiguration\n",
		"bad reserved":       "apiVersion: kubelet.config.k8s.io/v1beta1\nkind: KubeletConfiguration\nreservedSystemCPUs: \"zero\"\n",
	}
	for name, body := range cases {
		t.Run(name, func(t *testing.T) {
			fx := dropInFixture(t, staticKubelet("0-2", "true"), map[string]string{"50-bad.conf": body})
			wantRefusal(t, evalErr(t, fx), "kubelet configuration")
		})
	}
}

func TestKubeletDropInDirMissingRefused(t *testing.T) {
	fx := dropInFixture(t, staticKubelet("0-2", "true"), nil)
	fx.src.KubeletConfigDir = filepath.Join(fx.root, "absent.d")
	wantRefusal(t, evalErr(t, fx), "absent.d")
	// A file where a directory is configured is refused too.
	fx.src.KubeletConfigDir = fx.src.KubeletConfig
	wantRefusal(t, evalErr(t, fx), "not a directory")
	// A drop-in directory without a base file is still "not mounted".
	fx = dropInFixture(t, eksBase, map[string]string{"40-nodeadm.conf": nodeadmDropIn("0-2")})
	fx.src.KubeletConfig = ""
	wantRefusal(t, evalErr(t, fx), "kubelet configuration is not mounted")
}

func TestKubeletDropInUnreadableRefused(t *testing.T) {
	// A dangling symlink named *.conf fails to open for every user, root
	// included (the check lane runs as root, which reads a mode 0 file).
	fx := dropInFixture(t, staticKubelet("0-2", "true"), nil)
	if err := os.Symlink(filepath.Join(fx.root, "gone"), filepath.Join(fx.src.KubeletConfigDir, "50-gone.conf")); err != nil {
		t.Fatal(err)
	}
	wantRefusal(t, evalErr(t, fx), "50-gone.conf")
}

func TestKubeletDropInConfigZAndOverridesSeeMergedConfig(t *testing.T) {
	// configz is the effective config, so it is compared with the merge,
	// not with the base alone.
	fx := dropInFixture(t, eksBase, map[string]string{"40-nodeadm.conf": nodeadmDropIn("0-2")})
	fx.src.ConfigZ = filepath.Join(fx.root, "configz.json")
	if err := evalErr(t, fx); err != nil {
		t.Fatalf("configz of the merged config must agree: %v", err)
	}
	// An argv override that disagrees with the merge is still a conflict.
	fx.src.Overrides = filepath.Join(fx.root, "overrides")
	if err := os.WriteFile(fx.src.Overrides, []byte("--cpu-manager-policy=none\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	wantRefusal(t, evalErr(t, fx), "--cpu-manager-policy")
}
