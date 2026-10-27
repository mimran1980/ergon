// SPDX-License-Identifier: Apache-2.0

package config

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
)

// fixture is one synthetic node. The 48-CPU case is a fixture, not a measured
// c7i.24xlarge layout: three reserved cores, helpers on two of them, the IRQ
// destination on the third.
type fixture struct {
	root string
	src  Sources
}

type fixNIC struct {
	name string
	irqs []fixIRQ
}

type fixIRQ struct {
	n             int
	requested     string
	effective     string
	requestedMask string
	effectiveMask string
	dropRequested bool
	dropEffective bool
}

func writeFixture(t *testing.T, online, isolated, nohz, kubelet, configz, overrides string, nics []fixNIC, others []fixIRQ) fixture {
	t.Helper()
	root := t.TempDir()
	cpu := filepath.Join(root, "sys/devices/system/cpu")
	if err := os.MkdirAll(cpu, 0o755); err != nil {
		t.Fatal(err)
	}
	write := func(path, body string) {
		t.Helper()
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte(body), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	write(filepath.Join(cpu, "online"), online)
	write(filepath.Join(cpu, "isolated"), isolated)
	write(filepath.Join(cpu, "nohz_full"), nohz)
	if err := os.MkdirAll(filepath.Join(root, "sys/class/net/lo"), 0o755); err != nil {
		t.Fatal(err)
	}
	// A bridge has no device directory. It is not NIC inventory.
	if err := os.MkdirAll(filepath.Join(root, "sys/class/net/cni0"), 0o755); err != nil {
		t.Fatal(err)
	}
	for _, n := range nics {
		dev := filepath.Join(root, "sys/class/net", n.name, "device/msi_irqs")
		if err := os.MkdirAll(dev, 0o755); err != nil {
			t.Fatal(err)
		}
		for _, irq := range n.irqs {
			if err := os.WriteFile(filepath.Join(dev, strconv.Itoa(irq.n)), []byte{}, 0o644); err != nil {
				t.Fatal(err)
			}
			writeIRQ(t, root, irq)
		}
	}
	for _, irq := range others {
		writeIRQ(t, root, irq)
	}
	src := Sources{SysRoot: filepath.Join(root, "sys"), ProcRoot: filepath.Join(root, "proc")}
	if kubelet != "" {
		write(filepath.Join(root, "kubelet.yaml"), kubelet)
		src.KubeletConfig = filepath.Join(root, "kubelet.yaml")
	}
	if configz != "" {
		write(filepath.Join(root, "configz.json"), configz)
		src.ConfigZ = filepath.Join(root, "configz.json")
	}
	if overrides != "" {
		write(filepath.Join(root, "overrides"), overrides)
		src.Overrides = filepath.Join(root, "overrides")
	}
	return fixture{root: root, src: src}
}

func writeIRQ(t *testing.T, root string, irq fixIRQ) {
	t.Helper()
	dir := filepath.Join(root, "proc/irq", strconv.Itoa(irq.n))
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	put := func(name, body string, drop bool) {
		t.Helper()
		if drop || body == "" {
			return
		}
		if err := os.WriteFile(filepath.Join(dir, name), []byte(body), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	if irq.requestedMask != "" {
		put("smp_affinity", irq.requestedMask, false)
	} else {
		put("smp_affinity_list", irq.requested, irq.dropRequested)
	}
	if irq.effectiveMask != "" {
		put("effective_affinity", irq.effectiveMask, false)
	} else {
		put("effective_affinity_list", irq.effective, irq.dropEffective)
	}
}

func staticKubelet(reserved string, quota string) string {
	q := ""
	if quota != "" {
		q = "cpuCFSQuota: " + quota + "\n"
	}
	return "apiVersion: kubelet.config.k8s.io/v1beta1\nkind: KubeletConfiguration\ncpuManagerPolicy: static\n" +
		q + "reservedSystemCPUs: \"" + reserved + "\"\nfeatureGates:\n  DisableCPUQuotaWithExclusiveCPUs: true\n"
}

func staticConfigZ(reserved string) string {
	return "{\"kubeletconfig\":{\"cpuManagerPolicy\":\"static\",\"cpuCFSQuota\":true,\"reservedSystemCPUs\":\"" +
		reserved + "\",\"featureGates\":{\"DisableCPUQuotaWithExclusiveCPUs\":true}}}\n"
}

// tradingFixture is the synthetic 48-CPU node: reserved 0-2, helpers 0-1, IRQ
// destination CPU 2. Labelled as a fixture in the test log.
func tradingFixture(t *testing.T) fixture {
	t.Helper()
	nic := fixIRQ{n: 40, requested: "2\n", effective: "2\n"}
	cni := fixIRQ{n: 41, requested: "2\n", effective: "2\n"}
	return writeFixture(t,
		"0-47\n", "3-47\n", "3-47\n",
		staticKubelet("0-2", "true"),
		staticConfigZ("0-2"),
		"--v=2 --cpu-manager-policy=static --reserved-cpus=0-2 --cpu-cfs-quota=true\n",
		[]fixNIC{{name: "eth0", irqs: []fixIRQ{nic}}, {name: "ens5", irqs: []fixIRQ{cni}}},
		[]fixIRQ{{n: 1, requested: "0-47\n", effective: "0-47\n"}},
	)
}

func mustShared(t *testing.T, list string) cpuset.Set {
	t.Helper()
	s, err := cpuset.Parse(list)
	if err != nil {
		t.Fatal(err)
	}
	return s
}

func TestTradingFixture48CPUThreeReserved(t *testing.T) {
	fx := tradingFixture(t)
	shared := mustShared(t, "0-1")
	rep, err := Evaluate(fx.src, shared)
	if err != nil {
		t.Fatal(err)
	}
	if err := rep.Err(); err != nil {
		t.Fatal(err)
	}
	if rep.AllowedIRQ.String() != "2" {
		t.Fatalf("IRQ destination = %s, want 2", rep.AllowedIRQ)
	}
	if len(rep.Notes) == 0 || !strings.Contains(rep.Notes[0], "other irq 1") {
		t.Fatalf("notes = %v, want the non-NIC irq reported separately", rep.Notes)
	}
	if err := CheckContract(fx.src, shared, mustShared(t, "3,4")); err != nil {
		t.Fatal(err)
	}
	t.Log("synthetic 48-CPU topology with three reserved cores accepted")
}

func TestContractRefusals(t *testing.T) {
	shared := mustShared(t, "0-1")
	cases := []struct {
		name string
		edit func(t *testing.T, fx fixture)
		want string
	}{
		{
			name: "reserved-set mismatch",
			edit: func(t *testing.T, fx fixture) {
				if err := os.WriteFile(fx.src.KubeletConfig, []byte(staticKubelet("0", "true")), 0o644); err != nil {
					t.Fatal(err)
				}
				if err := os.WriteFile(fx.src.ConfigZ, []byte(staticConfigZ("0")), 0o644); err != nil {
					t.Fatal(err)
				}
				if err := os.WriteFile(fx.src.Overrides, []byte("--reserved-cpus=0\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			},
			want: "reserved-set mismatch",
		},
		{
			name: "cpuCFSQuota disabled",
			edit: func(t *testing.T, fx fixture) {
				body := strings.Replace(staticKubelet("0-2", "true"), "cpuCFSQuota: true", "cpuCFSQuota: false", 1)
				if err := os.WriteFile(fx.src.KubeletConfig, []byte(body), 0o644); err != nil {
					t.Fatal(err)
				}
				z := strings.Replace(staticConfigZ("0-2"), "\"cpuCFSQuota\":true", "\"cpuCFSQuota\":false", 1)
				if err := os.WriteFile(fx.src.ConfigZ, []byte(z), 0o644); err != nil {
					t.Fatal(err)
				}
			},
			want: "cpuCFSQuota disabled",
		},
		{
			name: "NIC requested/effective affinity overlap",
			edit: func(t *testing.T, fx fixture) {
				// Requested stays on the IRQ CPU. Effective has moved onto a helper.
				path := filepath.Join(fx.src.ProcRoot, "irq/40/effective_affinity_list")
				if err := os.WriteFile(path, []byte("0\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			},
			want: "NIC requested/effective affinity overlap",
		},
		{
			name: "offline shared CPUs",
			edit: func(t *testing.T, fx fixture) {
				path := filepath.Join(fx.src.SysRoot, "devices/system/cpu/online")
				if err := os.WriteFile(path, []byte("1-47\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			},
			want: "offline shared CPUs",
		},
		{
			name: "isolcpus shared CPUs",
			edit: func(t *testing.T, fx fixture) {
				path := filepath.Join(fx.src.SysRoot, "devices/system/cpu/isolated")
				if err := os.WriteFile(path, []byte("0-47\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			},
			want: "isolcpus shared CPUs",
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			fx := tradingFixture(t)
			tc.edit(t, fx)
			rep, err := Evaluate(fx.src, shared)
			if err != nil {
				t.Fatal(err)
			}
			if rep.Err() == nil || !strings.Contains(rep.Err().Error(), tc.want) {
				t.Fatalf("err = %v, want %q", rep.Err(), tc.want)
			}
			t.Log("refusal:", tc.want)
		})
	}
}

func TestExclusiveCPUNotIsolated(t *testing.T) {
	fx := tradingFixture(t)
	err := CheckContract(fx.src, mustShared(t, "0-1"), mustShared(t, "2,3"))
	if err == nil || !strings.Contains(err.Error(), "exclusive CPU not isolated") {
		t.Fatalf("err = %v", err)
	}
	t.Log("refusal: exclusive CPU not isolated")
}

func TestCheckHostOfflineAndIsolcpusSharedCPUs(t *testing.T) {
	c, err := Parse([]byte("sharedCpus: \"0,1\"\nnamespaces: [trading]\n"))
	if err != nil {
		t.Fatal(err)
	}
	offline := fakeSys(t, "1-9\n", "\n", "\n")
	_, err = CheckHost(c, offline, fakeCgroup(t, true), "6.12.0")
	if err == nil || !strings.Contains(err.Error(), "not online") {
		t.Fatalf("startup offline: %v", err)
	}
	t.Log("refusal: offline shared CPUs")
	isolated := fakeSys(t, "0-9\n", "0,1\n", "\n")
	_, err = CheckHost(c, isolated, fakeCgroup(t, true), "6.12.0")
	if err == nil || !strings.Contains(err.Error(), "isolated") {
		t.Fatalf("startup isolcpus: %v", err)
	}
	t.Log("refusal: isolcpus shared CPUs")
}

func TestIRQResetAndRepin(t *testing.T) {
	fx := tradingFixture(t)
	shared := mustShared(t, "0-1")
	path := filepath.Join(fx.src.ProcRoot, "irq/40/smp_affinity_list")
	eff := filepath.Join(fx.src.ProcRoot, "irq/40/effective_affinity_list")
	// A reset spreads the NIC onto every CPU, including helpers and busy CPUs.
	if err := os.WriteFile(path, []byte("0-47\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(eff, []byte("0-47\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	rep, err := Evaluate(fx.src, shared)
	if err != nil {
		t.Fatal(err)
	}
	if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "NIC requested/effective affinity overlap") {
		t.Fatalf("after reset: %v", rep.Err())
	}
	// The watcher re-pins onto the IRQ destination. Both masks have to move.
	if err := os.WriteFile(path, []byte("2\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(eff, []byte("2\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	rep, err = Evaluate(fx.src, shared)
	if err != nil || rep.Err() != nil {
		t.Fatalf("after re-pin: err %v refusal %v", err, rep.Err())
	}
	t.Log("NIC reset then re-pin: requested and effective affinity back on the IRQ destination")
}

func TestContractEdges(t *testing.T) {
	shared := mustShared(t, "0-1")
	t.Run("missing kubelet configuration", func(t *testing.T) {
		fx := tradingFixture(t)
		fx.src.KubeletConfig = ""
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "kubelet configuration is not mounted") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
	t.Run("malformed kubelet configuration", func(t *testing.T) {
		fx := tradingFixture(t)
		if err := os.WriteFile(fx.src.KubeletConfig, []byte("reservedSystemCPUs: [\n"), 0o644); err != nil {
			t.Fatal(err)
		}
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "kubelet configuration") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
	t.Run("conflicting command-line override", func(t *testing.T) {
		fx := tradingFixture(t)
		if err := os.WriteFile(fx.src.Overrides, []byte("--cpu-manager-policy=none\n"), 0o644); err != nil {
			t.Fatal(err)
		}
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "conflicting command-line override") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
	t.Run("configz disagrees", func(t *testing.T) {
		fx := tradingFixture(t)
		if err := os.WriteFile(fx.src.ConfigZ, []byte(staticConfigZ("0")), 0o644); err != nil {
			t.Fatal(err)
		}
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "configz disagrees") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
	t.Run("feature gate false", func(t *testing.T) {
		fx := tradingFixture(t)
		body := strings.Replace(staticKubelet("0-2", "true"), "DisableCPUQuotaWithExclusiveCPUs: true", "DisableCPUQuotaWithExclusiveCPUs: false", 1)
		if err := os.WriteFile(fx.src.KubeletConfig, []byte(body), 0o644); err != nil {
			t.Fatal(err)
		}
		z := strings.Replace(staticConfigZ("0-2"), "\"DisableCPUQuotaWithExclusiveCPUs\":true", "\"DisableCPUQuotaWithExclusiveCPUs\":false", 1)
		if err := os.WriteFile(fx.src.ConfigZ, []byte(z), 0o644); err != nil {
			t.Fatal(err)
		}
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "DisableCPUQuotaWithExclusiveCPUs is false") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
	t.Run("no NIC inventory", func(t *testing.T) {
		fx := writeFixture(t, "0-47\n", "3-47\n", "3-47\n", staticKubelet("0-2", "true"), "", "", nil, nil)
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "no NIC inventory") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
	t.Run("unreadable NIC mask", func(t *testing.T) {
		fx := tradingFixture(t)
		dir := filepath.Join(fx.src.ProcRoot, "irq/40")
		for _, name := range []string{"smp_affinity_list", "smp_affinity", "effective_affinity_list", "effective_affinity"} {
			if err := os.Remove(filepath.Join(dir, name)); err != nil && !os.IsNotExist(err) {
				t.Fatal(err)
			}
		}
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "unreadable affinity mask") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
	t.Run("shared/nohz overlap", func(t *testing.T) {
		fx := tradingFixture(t)
		path := filepath.Join(fx.src.SysRoot, "devices/system/cpu/nohz_full")
		if err := os.WriteFile(path, []byte("0-47\n"), 0o644); err != nil {
			t.Fatal(err)
		}
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "shared/nohz overlap") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
	t.Run("one-core lab allows the housekeeping CPU", func(t *testing.T) {
		irq := fixIRQ{n: 12, requested: "0\n", effective: "0\n"}
		fx := writeFixture(t, "0-3\n", "1-3\n", "1-3\n", staticKubelet("0", "true"), "", "",
			[]fixNIC{{name: "eth0", irqs: []fixIRQ{irq}}}, nil)
		rep, err := Evaluate(fx.src, mustShared(t, "0"))
		if err != nil || rep.Err() != nil {
			t.Fatalf("one-core: err %v refusal %v", err, rep.Err())
		}
		if rep.AllowedIRQ.String() != "0" {
			t.Fatalf("allowed = %s, want 0", rep.AllowedIRQ)
		}
	})
	t.Run("one-core lab refuses a busy CPU", func(t *testing.T) {
		irq := fixIRQ{n: 12, requested: "1\n", effective: "1\n"}
		fx := writeFixture(t, "0-3\n", "1-3\n", "1-3\n", staticKubelet("0", "true"), "", "",
			[]fixNIC{{name: "eth0", irqs: []fixIRQ{irq}}}, nil)
		rep, err := Evaluate(fx.src, mustShared(t, "0"))
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "NIC requested/effective affinity overlap") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
	t.Run("hex affinity mask", func(t *testing.T) {
		irq := fixIRQ{n: 12, requestedMask: "4\n", effectiveMask: "4\n"}
		fx := writeFixture(t, "0-3\n", "1-3\n", "\n", staticKubelet("0", ""), "", "",
			[]fixNIC{{name: "eth0", irqs: []fixIRQ{irq}}}, nil)
		// Bit 2 is CPU 2, which is a busy CPU on this node. Must refuse.
		rep, err := Evaluate(fx.src, mustShared(t, "0"))
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "NIC requested/effective affinity overlap") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
}

func TestParseAffinityMask(t *testing.T) {
	got, err := parseAffinityMask("00000000,0000000f")
	if err != nil {
		t.Fatal(err)
	}
	if got.String() != "0-3" {
		t.Fatalf("mask = %s, want 0-3", got)
	}
	got, err = parseAffinityMask("4")
	if err != nil {
		t.Fatal(err)
	}
	if got.String() != "2" {
		t.Fatalf("mask = %s, want 2", got)
	}
}

// vmbusNIC writes an Azure synthetic NIC (hv_netvsc): a device directory
// with no msi_irqs and no irq file, only VMBus channels, each with the CPU
// the host interrupts for it. cpus maps channel relid to its cpu file body;
// an empty body leaves the file out.
func vmbusNIC(t *testing.T, sysRoot, name string, cpus map[int]string) {
	t.Helper()
	for relid, body := range cpus {
		dir := filepath.Join(sysRoot, "class/net", name, "device/channels", strconv.Itoa(relid))
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
		if body == "" {
			continue
		}
		if err := os.WriteFile(filepath.Join(dir, "cpu"), []byte(body), 0o644); err != nil {
			t.Fatal(err)
		}
	}
}

// An Azure VM: eth0 is hv_netvsc (VMBus channels, no PCI interrupt) and the
// accelerated-networking VF is a PCI NIC with MSI-X. Reserved 0-1, shared 1,
// so the IRQ destination is CPU 0.
func azureFixture(t *testing.T) fixture {
	t.Helper()
	vf := fixIRQ{n: 30, requested: "0\n", effective: "0\n"}
	return writeFixture(t, "0-7\n", "2-7\n", "2-7\n", staticKubelet("0-1", "true"), "", "",
		[]fixNIC{{name: "enP30832s1", irqs: []fixIRQ{vf}}}, nil)
}

func TestVMBusNIC(t *testing.T) {
	shared := mustShared(t, "1")
	t.Run("channels on the IRQ CPU pass", func(t *testing.T) {
		fx := azureFixture(t)
		vmbusNIC(t, fx.src.SysRoot, "eth0", map[int]string{14: "0\n", 15: "0\n"})
		rep, err := Evaluate(fx.src, shared)
		if err != nil || rep.Err() != nil {
			t.Fatalf("err %v refusal %v", err, rep.Err())
		}
	})
	t.Run("a channel off the IRQ CPU is refused", func(t *testing.T) {
		fx := azureFixture(t)
		vmbusNIC(t, fx.src.SysRoot, "eth0", map[int]string{14: "0\n", 15: "3\n"})
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		want := "NIC eth0 vmbus channel 15 on CPU 3, allowed 0"
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), want) {
			t.Fatalf("err = %v, want %q", rep.Err(), want)
		}
	})
	t.Run("an unreadable channel CPU is refused", func(t *testing.T) {
		fx := azureFixture(t)
		vmbusNIC(t, fx.src.SysRoot, "eth0", map[int]string{14: "0\n", 15: ""})
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		want := "NIC eth0 vmbus channel 15: unreadable cpu"
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), want) {
			t.Fatalf("err = %v, want %q", rep.Err(), want)
		}
	})
	t.Run("a device with no interrupt and no channel is refused", func(t *testing.T) {
		fx := azureFixture(t)
		if err := os.MkdirAll(filepath.Join(fx.src.SysRoot, "class/net/eth0/device"), 0o755); err != nil {
			t.Fatal(err)
		}
		rep, err := Evaluate(fx.src, shared)
		if err != nil {
			t.Fatal(err)
		}
		if rep.Err() == nil || !strings.Contains(rep.Err().Error(), "NIC eth0: no NIC interrupt found") {
			t.Fatalf("err = %v", rep.Err())
		}
	})
}
