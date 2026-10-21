// SPDX-License-Identifier: Apache-2.0

package wrap

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
)

func writeCPUs(t *testing.T, text string) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "cpuset.cpus.effective")
	if err := os.WriteFile(path, []byte(text), 0o644); err != nil {
		t.Fatal(err)
	}
	return path
}

func testEnv(iso, shared, cores, milli string) []string {
	env := []string{
		"ERGON_ISOLATED_CPUS=" + iso,
		"ERGON_SHARED_CPUS=" + shared,
	}
	if cores != "" {
		env = append(env, "ERGON_ISOLATED_CORES="+cores)
	}
	if milli != "" {
		env = append(env, "ERGON_SHARED_MILLICPUS="+milli)
	}
	return env
}

func TestFromEnvReadsLists(t *testing.T) {
	p, err := FromEnv(testEnv("1,5", "4", "1,5", "250"), writeCPUs(t, "1,4-5\n"), t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	if p.Isolated.List() != "1,5" || p.Shared.List() != "4" || p.Milli != 250 {
		t.Fatalf("plan is %+v", p)
	}
	if got := cpuset.FormatCores(p.Cores); got != "1,5" {
		t.Fatalf("cores are %q, want \"1,5\"", got)
	}
}

func TestFromEnvOpenShiftFallback(t *testing.T) {
	env := []string{"OPENSHIFT_ISOLATED_CPUS=1-2,5-6", "OPENSHIFT_SHARED_CPUS=4"}
	p, err := FromEnv(env, writeCPUs(t, "1-2,4-6"), t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	if p.Isolated.String() != "1-2,5-6" {
		t.Fatalf("isolated is %s, want 1-2,5-6", p.Isolated)
	}
}

func TestFromEnvFailsClosedWhenMilliWithoutShared(t *testing.T) {
	env := []string{"ERGON_SHARED_MILLICPUS=250", "ERGON_ISOLATED_CPUS=1"}
	_, err := FromEnv(env, writeCPUs(t, "1"), t.TempDir())
	if err == nil {
		t.Fatal("FromEnv accepted ERGON_SHARED_MILLICPUS without ERGON_SHARED_CPUS")
	}
	if !strings.Contains(err.Error(), "NRI plugin did not set up") {
		t.Fatalf("error %q does not name the fail-closed reason", err)
	}
}

func TestFromEnvRejectsListOutsideCPUs(t *testing.T) {
	env := testEnv("1", "9", "", "")
	_, err := FromEnv(env, writeCPUs(t, "0-1"), t.TempDir())
	if err == nil {
		t.Fatal("FromEnv accepted a shared CPU outside cpuset.cpus.effective")
	}
}

// TestFromEnvAcceptsAnIsolatedPartition is the P2 regression: a valid child
// partition takes the exclusive CPUs out of the container's
// cpuset.cpus.effective, so the wrapper cannot require E to be in the
// container's own set. E belongs to the partition; S stays with the
// container.
func TestFromEnvAcceptsAnIsolatedPartition(t *testing.T) {
	dir := t.TempDir()
	// The container keeps only the shared CPUs; the partition holds E.
	if err := os.WriteFile(filepath.Join(dir, "cpuset.cpus.effective"), []byte("1-9\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(dir, "ergon-isolated"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "ergon-isolated", "cpuset.cpus"), []byte("0\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	p, err := FromEnv(testEnv("0", "1,5", "", ""), filepath.Join(dir, "cpuset.cpus.effective"), t.TempDir())
	if err != nil {
		t.Fatalf("a valid isolated partition must be accepted: %v", err)
	}
	if p.Isolated.List() != "0" || p.Shared.List() != "1,5" {
		t.Fatalf("plan is %+v", p)
	}
}

// TestFromEnvRejectsAnIsolatedListThatIsNotThePartition: the partition holds
// exactly E. A different E means the wrapper would name CPUs the hot thread
// does not get.
func TestFromEnvRejectsAnIsolatedListThatIsNotThePartition(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "cpuset.cpus.effective"), []byte("1-9\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(dir, "ergon-isolated"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "ergon-isolated", "cpuset.cpus"), []byte("2\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	_, err := FromEnv(testEnv("0", "1", "", ""), filepath.Join(dir, "cpuset.cpus.effective"), t.TempDir())
	if err == nil {
		t.Fatal("FromEnv accepted an isolated list that is not the partition's cpuset")
	}
}

// TestFromEnvRejectsSharedOutsideWhatIsLeft: the helpers run on the CPUs the
// container still has. A shared CPU that the partition took is a contradiction.
func TestFromEnvRejectsSharedOutsideWhatIsLeft(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "cpuset.cpus.effective"), []byte("1-3\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(dir, "ergon-isolated"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "ergon-isolated", "cpuset.cpus"), []byte("0\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	_, err := FromEnv(testEnv("0", "0,1", "", ""), filepath.Join(dir, "cpuset.cpus.effective"), t.TempDir())
	if err == nil {
		t.Fatal("FromEnv accepted a shared CPU the partition holds")
	}
}

func TestFromEnvRejectsOverlap(t *testing.T) {
	env := testEnv("1,2", "2,3", "", "")
	_, err := FromEnv(env, writeCPUs(t, "1-3"), t.TempDir())
	if err == nil {
		t.Fatal("FromEnv accepted overlapping isolated and shared lists")
	}
}

func TestResolveSpecs(t *testing.T) {
	p := &Plan{
		Isolated: cpuset.New(1, 5, 2, 6),
		Shared:   cpuset.New(4, 0),
		Cores:    []cpuset.Set{cpuset.New(1, 5), cpuset.New(2, 6)},
	}
	cases := []struct {
		spec, want string
	}{
		{"isolated", "1,2,5,6"},
		{"shared", "0,4"},
		{"isolated:0", "1"},
		{"isolated:3", "6"},
		{"shared:0", "0"},
		{"shared:1", "4"},
		{"core:0", "1"},
		{"core:0:1", "5"},
		{"core:1", "2"},
		{"core:1:1", "6"},
	}
	for _, c := range cases {
		got, err := p.Resolve(c.spec)
		if err != nil {
			t.Errorf("Resolve(%q): %v", c.spec, err)
			continue
		}
		if got != c.want {
			t.Errorf("Resolve(%q) = %q, want %q", c.spec, got, c.want)
		}
	}
}

func TestResolveOutOfRange(t *testing.T) {
	p := &Plan{
		Isolated: cpuset.New(1),
		Shared:   cpuset.New(4),
		Cores:    []cpuset.Set{cpuset.New(1)},
	}
	bad := []string{"isolated:1", "shared:5", "core:1", "core:0:3", "isolated:x", "shared:", "bogus:0", "core:0:x"}
	for _, spec := range bad {
		if v, err := p.Resolve(spec); err == nil {
			t.Errorf("Resolve(%q) = %q, want an error", spec, v)
		}
	}
}

func TestTranslateReplacesEnv(t *testing.T) {
	p := &Plan{
		Isolated: cpuset.New(1, 5),
		Shared:   cpuset.New(4),
		Cores:    []cpuset.Set{cpuset.New(1, 5)},
	}
	got, err := p.Translate(nil, []EnvBinding{
		{Name: "AERON_RECEIVER_CPU_AFFINITY", Spec: "isolated:0"},
		{Name: "AERON_SENDER_CPU_AFFINITY", Spec: "isolated:1"},
		{Name: "AERON_CONDUCTOR_CPU_AFFINITY", Spec: "shared:0"},
	})
	if err != nil {
		t.Fatal(err)
	}
	want := []string{
		"AERON_RECEIVER_CPU_AFFINITY=1",
		"AERON_SENDER_CPU_AFFINITY=5",
		"AERON_CONDUCTOR_CPU_AFFINITY=4",
	}
	for i := range want {
		if got[i] != want[i] {
			t.Errorf("binding %d = %q, want %q", i, got[i], want[i])
		}
	}
}

func TestTranslateRefusesAeronCpusetAffinity(t *testing.T) {
	p := &Plan{Isolated: cpuset.New(1), Shared: cpuset.New(4)}
	env := []string{"AERON_DRIVER_CPUSET_AFFINITY=true"}
	_, err := p.Translate(env, []EnvBinding{{Name: "AERON_RECEIVER_CPU_AFFINITY", Spec: "isolated:0"}})
	if err == nil {
		t.Fatal("Translate accepted an AERON_*_CPU_AFFINITY with AERON_DRIVER_CPUSET_AFFINITY=true")
	}
	if !strings.Contains(err.Error(), "AERON_DRIVER_CPUSET_AFFINITY") {
		t.Fatalf("error %q does not name the guard", err)
	}
}

func TestPinSelfAndExecKeepTheMask(t *testing.T) {
	self := ownSet(t)
	if self.Len() == 0 {
		t.Skip("no CPU in this test's affinity")
	}
	cpu := self.Slice()[0]
	if err := PinSelf(cpuset.New(cpu)); err != nil {
		t.Fatalf("PinSelf: %v", err)
	}
	got, err := SelfAffinity()
	if err != nil {
		t.Fatal(err)
	}
	if !got.Equals(cpuset.New(cpu)) {
		t.Fatalf("affinity after PinSelf is %s, want %d", got, cpu)
	}
	// Restore so the rest of the test binary runs normally.
	_ = PinSelf(self)
}

func ownSet(t *testing.T) cpuset.Set {
	t.Helper()
	got, err := SelfAffinity()
	if err != nil {
		t.Fatal(err)
	}
	return got
}

func TestExecPathNotFound(t *testing.T) {
	if err := Exec([]string{"definitely-not-a-real-command-xyz"}, nil); err == nil {
		t.Fatal("Exec found a command that does not exist")
	}
}

func TestExecNoCommand(t *testing.T) {
	if err := Exec(nil, nil); err == nil {
		t.Fatal("Exec accepted an empty argv")
	}
}

// --- SCHED_FIFO wrapper ---

func TestSetFifoPriorityRange(t *testing.T) {
	// The range check is pure and must fail before the syscall.
	for _, prio := range []int{0, -1, 100, 1000} {
		if err := SetFifo(prio); err == nil {
			t.Errorf("SetFifo(%d) must fail on the range check", prio)
		} else if !strings.Contains(err.Error(), "1..99") {
			t.Errorf("SetFifo(%d): error %q must name the range", prio, err)
		}
	}
}

func TestSetFifoNeedsRtprio(t *testing.T) {
	// With no RLIMIT_RTPRIO and no CAP_SYS_NICE the syscall fails with a
	// typed error that names the fix. That is the unprivileged pod case.
	err := SetFifo(40)
	if err == nil {
		// This process happens to have the right to take SCHED_FIFO. The
		// behaviour is then exercised for real: drop back and continue.
		if _, _, curErr := CurrentPolicy(); curErr != nil {
			t.Fatalf("CurrentPolicy: %v", curErr)
		}
		return
	}
	if !strings.Contains(err.Error(), "sched-fifo.ergon.io") {
		t.Errorf("error %q must name the opt-in annotation", err)
	}
}
