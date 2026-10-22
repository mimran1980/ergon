// SPDX-License-Identifier: Apache-2.0

package nriplugin

import (
	"context"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/containerd/nri/pkg/api"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/config"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

// hotSources builds a node whose shared set is 0,4 and whose exclusive CPUs
// 1,5 are isolated. IRQ destination is the reserved set itself, because the
// helper CPUs are the whole reservation (the one-core shape, on two CPUs).
func hotSources(t *testing.T) config.Sources {
	t.Helper()
	root := t.TempDir()
	write := func(path, body string) {
		t.Helper()
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte(body), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	sys := filepath.Join(root, "sys")
	proc := filepath.Join(root, "proc")
	write(filepath.Join(sys, "devices/system/cpu/online"), "0-7\n")
	write(filepath.Join(sys, "devices/system/cpu/isolated"), "1-3,5-7\n")
	write(filepath.Join(sys, "devices/system/cpu/nohz_full"), "1-3,5-7\n")
	if err := os.MkdirAll(filepath.Join(sys, "class/net/eth0/device/msi_irqs"), 0o755); err != nil {
		t.Fatal(err)
	}
	write(filepath.Join(sys, "class/net/eth0/device/msi_irqs/40"), "")
	write(filepath.Join(proc, "irq/40/smp_affinity_list"), "0\n")
	write(filepath.Join(proc, "irq/40/effective_affinity_list"), "0\n")
	kubelet := filepath.Join(root, "kubelet.yaml")
	write(kubelet, hotKubelet("\"0,4\"", "true"))
	return config.Sources{SysRoot: sys, ProcRoot: proc, KubeletConfig: kubelet}
}

func hotKubelet(reserved, quota string) string {
	return "cpuManagerPolicy: static\ncpuCFSQuota: " + quota + "\nreservedSystemCPUs: " + reserved + "\n"
}

func installContract(t *testing.T, src config.Sources) testPluginResult {
	t.Helper()
	p := testPlugin(t, "0,4")
	p.SetHostContract(func(shared, exclusive cpuset.Set) error {
		return config.CheckContract(src, shared, exclusive)
	})
	return p
}

func TestHotCreateCannotOptOutHook(t *testing.T) {
	src := hotSources(t)
	p := installContract(t, src)
	c := ctr("ctr-hot", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	pod := pod()
	pod.Labels = map[string]string{names.LatencyLabel: names.LatencyHot}
	pod.Annotations = map[string]string{names.AffinityPrefix + "app": config.AffinityAll}
	_, _, err := p.CreateContainer(context.Background(), pod, c)
	if err == nil || !strings.Contains(err.Error(), "hot containers cannot select") {
		t.Fatalf("hot opt-out: %v", err)
	}
	t.Log("hot containers cannot select hook opt-out")

	pod.Annotations = nil
	adj, _, err := p.CreateContainer(context.Background(), pod, c)
	if err != nil {
		t.Fatal(err)
	}
	if adj == nil || len(adj.GetHooks().GetCreateRuntime()) == 0 {
		t.Fatal("hot container was created without the start-affinity hook")
	}
}

func TestHotCreateUsesTheNodeContract(t *testing.T) {
	src := hotSources(t)
	p := installContract(t, src)
	c := ctr("ctr-hot", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})
	pod := pod()
	pod.Labels = map[string]string{names.LatencyLabel: names.LatencyHot}
	adj, _, err := p.CreateContainer(context.Background(), pod, c)
	if err != nil {
		t.Fatal(err)
	}
	if adj == nil || adj.GetLinux().GetResources().GetCpu().GetCpus() == "" {
		t.Fatal("hot create produced no cpuset")
	}
	if healthCode(p) != 200 {
		t.Fatal("healthz is not 200 after a clean hot create")
	}

	// Drift: the mounted reservation no longer covers the helper set.
	// The process does not exit; the next non-hot create still runs.
	if err := os.WriteFile(src.KubeletConfig, []byte(hotKubelet("\"0\"", "true")), 0o644); err != nil {
		t.Fatal(err)
	}
	_, _, err = p.CreateContainer(context.Background(), pod, ctr("ctr-drift", "1,5", nil, map[string]string{names.MilliAnnotation: "250"}))
	if err == nil || !strings.Contains(err.Error(), "reserved-set mismatch") {
		t.Fatalf("drift err = %v, want reserved-set mismatch", err)
	}
	if healthCode(p) != 200 {
		t.Fatal("a contract refusal failed liveness")
	}
	if _, _, err := p.CreateContainer(context.Background(), podWithoutHot(), ctr("ctr-ordinary", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})); err != nil {
		t.Fatalf("non-hot create after drift: %v", err)
	}
	t.Log("drift refuses a new hot create while the plugin process stays up")
	t.Log("refusal: reserved-set mismatch")
}

func podWithoutHot() *api.PodSandbox {
	return pod()
}

func healthCode(p testPluginResult) int {
	rec := httptest.NewRecorder()
	p.m.Handler().ServeHTTP(rec, httptest.NewRequest(http.MethodGet, "/healthz", nil))
	return rec.Code
}

func TestHotCreateRefusalsFromCreateContainer(t *testing.T) {
	cases := []struct {
		name string
		want string
		edit func(t *testing.T, src config.Sources)
	}{
		{
			name: "cpuCFSQuota disabled",
			want: "cpuCFSQuota disabled",
			edit: func(t *testing.T, src config.Sources) {
				if err := os.WriteFile(src.KubeletConfig, []byte(hotKubelet("\"0,4\"", "false")), 0o644); err != nil {
					t.Fatal(err)
				}
			},
		},
		{
			name: "NIC requested/effective affinity overlap",
			want: "NIC requested/effective affinity overlap",
			edit: func(t *testing.T, src config.Sources) {
				path := filepath.Join(src.ProcRoot, "irq/40/effective_affinity_list")
				if err := os.WriteFile(path, []byte("1\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			},
		},
		{
			name: "exclusive CPU not isolated",
			want: "exclusive CPU not isolated",
			edit: func(t *testing.T, src config.Sources) {
				path := filepath.Join(src.SysRoot, "devices/system/cpu/isolated")
				if err := os.WriteFile(path, []byte("2,6\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			},
		},
		{
			name: "offline shared CPUs",
			want: "offline shared CPUs",
			edit: func(t *testing.T, src config.Sources) {
				path := filepath.Join(src.SysRoot, "devices/system/cpu/online")
				if err := os.WriteFile(path, []byte("1-7\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			},
		},
		{
			name: "isolcpus shared CPUs",
			want: "isolcpus shared CPUs",
			edit: func(t *testing.T, src config.Sources) {
				path := filepath.Join(src.SysRoot, "devices/system/cpu/isolated")
				if err := os.WriteFile(path, []byte("0-7\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			},
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			src := hotSources(t)
			tc.edit(t, src)
			p := installContract(t, src)
			pod := pod()
			pod.Labels = map[string]string{names.LatencyLabel: names.LatencyHot}
			_, _, err := p.CreateContainer(context.Background(), pod, ctr("ctr-bad", "1,5", nil, map[string]string{names.MilliAnnotation: "250"}))
			if err == nil || !strings.Contains(err.Error(), tc.want) {
				t.Fatalf("err = %v, want %q", err, tc.want)
			}
			if healthCode(p) != 200 {
				t.Fatal("healthz is not 200")
			}
			t.Log("refusal:", tc.want)
		})
	}
}

func TestHotCreateWithoutAContractIsRefused(t *testing.T) {
	p := testPlugin(t, "0,4")
	pod := pod()
	pod.Labels = map[string]string{names.LatencyLabel: names.LatencyHot}
	_, _, err := p.CreateContainer(context.Background(), pod, ctr("ctr-hot", "1,5", nil, map[string]string{names.MilliAnnotation: "250"}))
	if err == nil || !strings.Contains(err.Error(), "node contract is not configured") {
		t.Fatalf("err = %v", err)
	}
	if _, _, err := p.CreateContainer(context.Background(), podWithoutHot(), ctr("ctr-ok", "1,5", nil, map[string]string{names.MilliAnnotation: "250"})); err != nil {
		t.Fatalf("non-hot: %v", err)
	}
}
