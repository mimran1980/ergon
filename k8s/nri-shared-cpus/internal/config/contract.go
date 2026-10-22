// SPDX-License-Identifier: Apache-2.0

package config

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"sigs.k8s.io/yaml"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
)

// Sources are the files a hot-create check reads. Tests point them at a
// fixture tree. An empty KubeletConfig means the final kubelet configuration
// is not mounted; that refuses hot creates and does not stop the process.
type Sources struct {
	// SysRoot is a sysfs root (devices/system/cpu and class/net).
	SysRoot string
	// ProcRoot is a proc root (irq/<n>/smp_affinity_list).
	ProcRoot string
	// KubeletConfig is the final kubelet configuration file.
	KubeletConfig string
	// ConfigZ is an optional copy of the effective /configz document.
	// When the path is set, the document must exist and agree with the file.
	ConfigZ string
	// Overrides is an optional kubelet argv file or a directory of drop-ins.
	// A checked flag that disagrees with the file is a conflicting override.
	Overrides string
}

// Report is one evaluation of Sources against a shared helper set.
type Report struct {
	// Refusals stop a new hot container. They do not stop the plugin.
	Refusals []string
	// Notes are managed or per-CPU interrupts that are not NIC placement.
	Notes []string
	// AllowedIRQ is the IRQ destination: reserved minus shared, or the whole
	// reserved set when that difference is empty (a one-CPU housekeeping node).
	AllowedIRQ cpuset.Set
	// Isolated is the kernel isolcpus set, used for the exclusive-CPU check.
	Isolated cpuset.Set
}

// Err joins the refusals. A clean report returns nil.
func (r Report) Err() error {
	if len(r.Refusals) == 0 {
		return nil
	}
	return errors.New(strings.Join(r.Refusals, "; "))
}

// kubeletBody is the subset of KubeletConfiguration this check reads.
// Pointers distinguish "absent" from an explicit false.
type kubeletBody struct {
	CPUManagerPolicy   string          `json:"cpuManagerPolicy"`
	CPUCFSQuota        *bool           `json:"cpuCFSQuota"`
	ReservedSystemCPUs string          `json:"reservedSystemCPUs"`
	FeatureGates       map[string]bool `json:"featureGates"`
}

// Evaluate reads Sources and checks shared against the node contract:
// shared CPUs online, not isolated and not in nohz_full; shared ⊆
// reservedSystemCPUs; static CPU manager; cpuCFSQuota not disabled;
// DisableCPUQuotaWithExclusiveCPUs not explicitly false; NIC requested and
// effective affinity inside the IRQ destination. Other interrupts are notes.
func Evaluate(src Sources, shared cpuset.Set) (Report, error) {
	var rep Report
	cpuDir := filepath.Join(src.SysRoot, "devices/system/cpu")
	online, err := readCPUFile(filepath.Join(cpuDir, "online"))
	if err != nil {
		return rep, fmt.Errorf("node contract: %w", err)
	}
	isolated, err := readCPUFile(filepath.Join(cpuDir, "isolated"))
	if err != nil {
		return rep, fmt.Errorf("node contract: %w", err)
	}
	nohz, err := readCPUFile(filepath.Join(cpuDir, "nohz_full"))
	if err != nil {
		return rep, fmt.Errorf("node contract: %w", err)
	}
	rep.Isolated = isolated
	if missing := shared.Difference(online); !missing.IsEmpty() {
		rep.Refusals = append(rep.Refusals, fmt.Sprintf("offline shared CPUs: %s", missing))
	}
	if both := shared.Intersection(isolated); !both.IsEmpty() {
		rep.Refusals = append(rep.Refusals, fmt.Sprintf("isolcpus shared CPUs: %s", both))
	}
	if both := shared.Intersection(nohz); !both.IsEmpty() {
		rep.Refusals = append(rep.Refusals, fmt.Sprintf("shared/nohz overlap: %s", both))
	}

	body, reserved, kubeErr := loadKubelet(src.KubeletConfig)
	if kubeErr != "" {
		rep.Refusals = append(rep.Refusals, kubeErr)
	} else {
		rep.Refusals = append(rep.Refusals, kubeletRefusals(body, reserved, shared)...)
		rep.AllowedIRQ = irqDestination(reserved, shared)
		if src.ConfigZ != "" {
			if msg := configZAgrees(src.ConfigZ, body, reserved); msg != "" {
				rep.Refusals = append(rep.Refusals, msg)
			}
		}
		if src.Overrides != "" {
			if msg := overridesAgree(src.Overrides, body, reserved); msg != "" {
				rep.Refusals = append(rep.Refusals, msg)
			}
		}
		nicRefusals, notes, nicErr := checkNICAffinity(src, rep.AllowedIRQ)
		if nicErr != nil {
			return rep, fmt.Errorf("node contract: %w", nicErr)
		}
		rep.Refusals = append(rep.Refusals, nicRefusals...)
		rep.Notes = notes
	}
	return rep, nil
}

// CheckContract is the hot-create check: Evaluate plus "every exclusive CPU
// is in the kernel isolated set". An empty exclusive set has nothing to check.
func CheckContract(src Sources, shared, exclusive cpuset.Set) error {
	rep, err := Evaluate(src, shared)
	if err != nil {
		return err
	}
	if !exclusive.IsEmpty() {
		if missing := exclusive.Difference(rep.Isolated); !missing.IsEmpty() {
			rep.Refusals = append(rep.Refusals, fmt.Sprintf("exclusive CPU not isolated: %s", missing))
		}
	}
	return rep.Err()
}

func loadKubelet(path string) (kubeletBody, cpuset.Set, string) {
	if path == "" {
		return kubeletBody{}, cpuset.Set{}, "kubelet configuration is not mounted"
	}
	body, err := readKubeletFile(path)
	if err != nil {
		return kubeletBody{}, cpuset.Set{}, fmt.Sprintf("kubelet configuration: %v", err)
	}
	reserved, err := parseMaybeCPUList(body.ReservedSystemCPUs)
	if err != nil {
		return kubeletBody{}, cpuset.Set{}, fmt.Sprintf("kubelet configuration: reservedSystemCPUs: %v", err)
	}
	return body, reserved, ""
}

func readKubeletFile(path string) (kubeletBody, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return kubeletBody{}, err
	}
	var wrapped struct {
		KubeletConfig *kubeletBody `json:"kubeletconfig"`
	}
	if err := yaml.Unmarshal(data, &wrapped); err != nil {
		return kubeletBody{}, err
	}
	if wrapped.KubeletConfig != nil {
		return *wrapped.KubeletConfig, nil
	}
	var body kubeletBody
	if err := yaml.Unmarshal(data, &body); err != nil {
		return kubeletBody{}, err
	}
	return body, nil
}

func kubeletRefusals(body kubeletBody, reserved, shared cpuset.Set) []string {
	var out []string
	if body.CPUManagerPolicy != "static" {
		out = append(out, fmt.Sprintf("cpuManagerPolicy is %q, want static", body.CPUManagerPolicy))
	}
	if body.CPUCFSQuota != nil && !*body.CPUCFSQuota {
		out = append(out, "cpuCFSQuota disabled")
	}
	if body.FeatureGates != nil {
		if v, ok := body.FeatureGates["DisableCPUQuotaWithExclusiveCPUs"]; ok && !v {
			out = append(out, "DisableCPUQuotaWithExclusiveCPUs is false")
		}
	}
	if !shared.IsSubsetOf(reserved) {
		out = append(out, fmt.Sprintf("reserved-set mismatch: shared %s is outside reservedSystemCPUs %s", shared, reserved))
	}
	return out
}

// irqDestination is H \ S. When that is empty the only housekeeping CPU is
// also the helper CPU, and NIC affinity may sit on it.
func irqDestination(reserved, shared cpuset.Set) cpuset.Set {
	dest := reserved.Difference(shared)
	if dest.IsEmpty() {
		return reserved
	}
	return dest
}

func configZAgrees(path string, file kubeletBody, reserved cpuset.Set) string {
	body, err := readKubeletFile(path)
	if err != nil {
		return fmt.Sprintf("configz: %v", err)
	}
	live, err := parseMaybeCPUList(body.ReservedSystemCPUs)
	if err != nil {
		return fmt.Sprintf("configz: reservedSystemCPUs: %v", err)
	}
	var diff []string
	if body.CPUManagerPolicy != file.CPUManagerPolicy {
		diff = append(diff, "cpuManagerPolicy")
	}
	if quotaDisabled(body.CPUCFSQuota) != quotaDisabled(file.CPUCFSQuota) {
		diff = append(diff, "cpuCFSQuota")
	}
	if !live.Equals(reserved) {
		diff = append(diff, "reservedSystemCPUs")
	}
	if gateDisabled(body.FeatureGates) != gateDisabled(file.FeatureGates) {
		diff = append(diff, "DisableCPUQuotaWithExclusiveCPUs")
	}
	if len(diff) == 0 {
		return ""
	}
	return "configz disagrees with the kubelet configuration: " + strings.Join(diff, ", ")
}

func quotaDisabled(v *bool) bool { return v != nil && !*v }

func gateDisabled(m map[string]bool) bool {
	if m == nil {
		return false
	}
	v, ok := m["DisableCPUQuotaWithExclusiveCPUs"]
	return ok && !v
}

func overridesAgree(path string, file kubeletBody, reserved cpuset.Set) string {
	policy, reservedText, quota, err := scanOverrides(path)
	if err != nil {
		return fmt.Sprintf("kubelet overrides: %v", err)
	}
	var diff []string
	if policy != "" && policy != file.CPUManagerPolicy {
		diff = append(diff, "--cpu-manager-policy")
	}
	if reservedText != "" {
		got, err := parseMaybeCPUList(reservedText)
		if err != nil {
			return fmt.Sprintf("kubelet overrides: reserved-cpus: %v", err)
		}
		if !got.Equals(reserved) {
			diff = append(diff, "--reserved-cpus")
		}
	}
	if quota != nil && quotaDisabled(quota) != quotaDisabled(file.CPUCFSQuota) {
		diff = append(diff, "--cpu-cfs-quota")
	}
	if len(diff) == 0 {
		return ""
	}
	return "conflicting command-line override: " + strings.Join(diff, ", ")
}

// scanOverrides reads one argv file or every regular file in a directory.
// The last occurrence of each checked flag wins. YAML drop-ins that set the
// same fields count as overrides too.
func scanOverrides(path string) (policy, reserved string, quota *bool, err error) {
	info, err := os.Stat(path)
	if err != nil {
		return "", "", nil, err
	}
	var files []string
	if info.IsDir() {
		entries, err := os.ReadDir(path)
		if err != nil {
			return "", "", nil, err
		}
		for _, e := range entries {
			if e.IsDir() {
				continue
			}
			files = append(files, filepath.Join(path, e.Name()))
		}
		sort.Strings(files)
	} else {
		files = []string{path}
	}
	for _, f := range files {
		data, err := os.ReadFile(f)
		if err != nil {
			return "", "", nil, err
		}
		text := string(data)
		p, r, q := flagsIn(text)
		if p != "" {
			policy = p
		}
		if r != "" {
			reserved = r
		}
		if q != nil {
			quota = q
		}
		var body kubeletBody
		if yaml.Unmarshal(data, &body) == nil {
			if body.CPUManagerPolicy != "" {
				policy = body.CPUManagerPolicy
			}
			if strings.Contains(text, "reservedSystemCPUs") {
				reserved = body.ReservedSystemCPUs
			}
			if body.CPUCFSQuota != nil {
				quota = body.CPUCFSQuota
			}
		}
	}
	return policy, reserved, quota, nil
}

func flagsIn(text string) (policy, reserved string, quota *bool) {
	toks := strings.Fields(text)
	for i := 0; i < len(toks); i++ {
		key, val, has := strings.Cut(toks[i], "=")
		take := func() (string, bool) {
			if has {
				return val, true
			}
			if i+1 < len(toks) && !strings.HasPrefix(toks[i+1], "-") {
				i++
				return toks[i], true
			}
			return "", false
		}
		switch key {
		case "--cpu-manager-policy":
			if v, ok := take(); ok {
				policy = v
			}
		case "--reserved-cpus":
			if v, ok := take(); ok {
				reserved = v
			}
		case "--cpu-cfs-quota":
			if v, ok := take(); ok {
				if b, ok := parseBoolFlag(v); ok {
					quota = &b
				}
			} else {
				on := true
				quota = &on
			}
		}
	}
	return policy, reserved, quota
}

func parseBoolFlag(s string) (bool, bool) {
	switch strings.ToLower(s) {
	case "1", "t", "true", "y", "yes":
		return true, true
	case "0", "f", "false", "n", "no":
		return false, true
	default:
		return false, false
	}
}

func parseMaybeCPUList(s string) (cpuset.Set, error) {
	if strings.TrimSpace(s) == "" {
		return cpuset.Set{}, nil
	}
	return cpuset.Parse(s)
}

// checkNICAffinity checks requested and effective affinity of every hardware
// NIC under sys/class/net. Interfaces with no device directory (bridges,
// veth, lo) are not NIC inventory. Missing inventory or an unreadable NIC
// mask is a refusal. Other /proc/irq entries are notes.
func checkNICAffinity(src Sources, allowed cpuset.Set) (refusals, notes []string, err error) {
	nics, err := hardwareNICs(src.SysRoot)
	if err != nil {
		return []string{err.Error()}, nil, nil
	}
	nicIRQs := map[int]bool{}
	for _, n := range nics {
		for _, irq := range n.irqs {
			nicIRQs[irq] = true
			req, reqErr := readIRQAffinity(src.ProcRoot, irq, false)
			eff, effErr := readIRQAffinity(src.ProcRoot, irq, true)
			if reqErr != nil || effErr != nil {
				refusals = append(refusals, fmt.Sprintf("NIC %s irq %d: unreadable affinity mask", n.name, irq))
				continue
			}
			if !req.IsSubsetOf(allowed) || !eff.IsSubsetOf(allowed) {
				refusals = append(refusals, fmt.Sprintf(
					"NIC requested/effective affinity overlap: %s irq %d requested %s effective %s, allowed %s",
					n.name, irq, req, eff, allowed))
			}
		}
	}
	notes, err = otherIRQNotes(src.ProcRoot, nicIRQs)
	if err != nil {
		return nil, nil, err
	}
	return refusals, notes, nil
}

type nicInfo struct {
	name string
	irqs []int
}

func hardwareNICs(sysRoot string) ([]nicInfo, error) {
	netDir := filepath.Join(sysRoot, "class/net")
	entries, err := os.ReadDir(netDir)
	if err != nil {
		return nil, fmt.Errorf("no NIC inventory: %w", err)
	}
	var out []nicInfo
	for _, e := range entries {
		name := e.Name()
		if name == "lo" {
			continue
		}
		dev := filepath.Join(netDir, name, "device")
		st, err := os.Stat(dev)
		if err != nil || !st.IsDir() {
			continue
		}
		irqs, err := ifaceIRQs(filepath.Join(netDir, name))
		if err != nil {
			return nil, fmt.Errorf("NIC %s: %w", name, err)
		}
		if len(irqs) == 0 {
			return nil, fmt.Errorf("NIC %s: no NIC interrupt found", name)
		}
		out = append(out, nicInfo{name: name, irqs: irqs})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].name < out[j].name })
	if len(out) == 0 {
		return nil, errors.New("no NIC inventory")
	}
	return out, nil
}

func ifaceIRQs(iface string) ([]int, error) {
	msi := filepath.Join(iface, "device/msi_irqs")
	entries, err := os.ReadDir(msi)
	if err == nil {
		var irqs []int
		for _, e := range entries {
			n, conv := strconv.Atoi(e.Name())
			if conv != nil || n < 0 {
				continue
			}
			irqs = append(irqs, n)
		}
		sort.Ints(irqs)
		return irqs, nil
	}
	if !errors.Is(err, fs.ErrNotExist) {
		return nil, err
	}
	data, err := os.ReadFile(filepath.Join(iface, "device/irq"))
	if errors.Is(err, fs.ErrNotExist) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	n, err := strconv.Atoi(strings.TrimSpace(string(data)))
	if err != nil {
		return nil, fmt.Errorf("device/irq: %w", err)
	}
	if n <= 0 {
		return nil, nil
	}
	return []int{n}, nil
}

func readIRQAffinity(procRoot string, irq int, effective bool) (cpuset.Set, error) {
	dir := filepath.Join(procRoot, "irq", strconv.Itoa(irq))
	listName, maskName := "smp_affinity_list", "smp_affinity"
	if effective {
		listName, maskName = "effective_affinity_list", "effective_affinity"
	}
	data, err := os.ReadFile(filepath.Join(dir, listName))
	if err == nil {
		s, perr := cpuset.Parse(string(data))
		if perr != nil {
			return cpuset.Set{}, fmt.Errorf("%s: %w", listName, perr)
		}
		return s, nil
	}
	if !errors.Is(err, fs.ErrNotExist) {
		return cpuset.Set{}, err
	}
	data, err = os.ReadFile(filepath.Join(dir, maskName))
	if err != nil {
		return cpuset.Set{}, fmt.Errorf("unreadable affinity: %w", err)
	}
	return parseAffinityMask(string(data))
}

// parseAffinityMask reads a kernel smp_affinity bitmask. The rightmost
// comma-separated word is CPUs 0-31.
func parseAffinityMask(s string) (cpuset.Set, error) {
	text := strings.TrimSpace(s)
	if text == "" {
		return cpuset.Set{}, errors.New("empty affinity mask")
	}
	parts := strings.Split(text, ",")
	var cpus []int
	group := 0
	for i := len(parts) - 1; i >= 0; i-- {
		word := strings.TrimSpace(parts[i])
		if word == "" || len(word) > 8 {
			return cpuset.Set{}, fmt.Errorf("affinity mask %q", s)
		}
		v, err := strconv.ParseUint(word, 16, 32)
		if err != nil {
			return cpuset.Set{}, fmt.Errorf("affinity mask %q: %w", s, err)
		}
		for b := 0; b < 32; b++ {
			if v&(uint64(1)<<uint(b)) != 0 {
				cpus = append(cpus, group*32+b)
			}
		}
		group++
	}
	if len(cpus) == 0 {
		return cpuset.Set{}, errors.New("empty affinity mask")
	}
	return cpuset.New(cpus...), nil
}

func otherIRQNotes(procRoot string, nicIRQs map[int]bool) ([]string, error) {
	dir := filepath.Join(procRoot, "irq")
	entries, err := os.ReadDir(dir)
	if err != nil {
		if errors.Is(err, fs.ErrNotExist) {
			return nil, nil
		}
		return nil, err
	}
	var notes []string
	for _, e := range entries {
		n, conv := strconv.Atoi(e.Name())
		if conv != nil || nicIRQs[n] {
			continue
		}
		req, reqErr := readIRQAffinity(procRoot, n, false)
		if reqErr != nil {
			notes = append(notes, fmt.Sprintf("other irq %d affinity unreadable", n))
			continue
		}
		notes = append(notes, fmt.Sprintf("other irq %d requested %s", n, req))
	}
	sort.Strings(notes)
	return notes, nil
}
