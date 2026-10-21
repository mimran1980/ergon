// SPDX-License-Identifier: Apache-2.0

package cpuset

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestParseFormats(t *testing.T) {
	cases := []struct {
		in, ranges, list string
	}{
		{"0", "0", "0"},
		{"1,5", "1,5", "1,5"},
		{"5,1", "1,5", "1,5"},
		{"0-3,8", "0-3,8", "0,1,2,3,8"},
		{"1-2,5-6", "1-2,5-6", "1,2,5,6"},
		{"1,2,5,6", "1-2,5-6", "1,2,5,6"},
		{"3,3,3", "3", "3"},
		{"0-2,1-3", "0-3", "0,1,2,3"},
		{"4-4", "4", "4"},
		{" 0-1\n", "0-1", "0,1"},
	}
	for _, c := range cases {
		s, err := Parse(c.in)
		if err != nil {
			t.Fatalf("Parse(%q): %v", c.in, err)
		}
		if got := s.String(); got != c.ranges {
			t.Errorf("Parse(%q).String() = %q, want %q", c.in, got, c.ranges)
		}
		if got := s.List(); got != c.list {
			t.Errorf("Parse(%q).List() = %q, want %q", c.in, got, c.list)
		}
	}
}

func TestParseRejects(t *testing.T) {
	for _, in := range []string{"", " ", "\n", "a", "1,", ",1", "1,,2", "3-1", "-1", "1-", "1-2-3", "+1", "1 ,2", "0x1", "8192", "1-99999"} {
		if s, err := Parse(in); err == nil {
			t.Errorf("Parse(%q) = %v, want an error", in, s)
		}
	}
}

func TestSetOperations(t *testing.T) {
	a := New(1, 5, 2)
	b := New(5, 6)
	if got := a.Union(b).List(); got != "1,2,5,6" {
		t.Errorf("union = %s", got)
	}
	if got := a.Intersection(b).List(); got != "5" {
		t.Errorf("intersection = %s", got)
	}
	if got := a.Difference(b).List(); got != "1,2" {
		t.Errorf("difference = %s", got)
	}
	if !New(1, 2).IsSubsetOf(a) || a.IsSubsetOf(b) {
		t.Error("subset")
	}
	if !New().IsSubsetOf(a) || !New().IsEmpty() || New().Len() != 0 {
		t.Error("empty set")
	}
	if !a.Equals(New(2, 1, 5)) || a.Equals(b) {
		t.Error("equals")
	}
	if !a.Contains(5) || a.Contains(3) {
		t.Error("contains")
	}
	if got := a.Slice(); !reflect.DeepEqual(got, []int{1, 2, 5}) {
		t.Errorf("slice = %v", got)
	}
	// Slice is a copy: changing it does not change the set.
	sl := a.Slice()
	sl[0] = 99
	if a.List() != "1,2,5" {
		t.Error("Slice shares memory with the set")
	}
	if New().String() != "" || New().List() != "" {
		t.Error("empty set formats to text")
	}
}

func TestCores(t *testing.T) {
	// x86 c7i: siblings n and n+4.
	x86 := map[int]Set{}
	for c := 0; c < 4; c++ {
		x86[c] = New(c, c+4)
		x86[c+4] = New(c, c+4)
	}
	// x86 with siblings n and n+1.
	adj := map[int]Set{0: New(0, 1), 1: New(0, 1), 2: New(2, 3), 3: New(2, 3)}
	cases := []struct {
		name     string
		cpus     Set
		siblings map[int]Set
		want     string
	}{
		{"x86 n/n+4, one core", New(1, 5), x86, "1,5"},
		{"x86 n/n+4, two cores", New(1, 2, 5, 6), x86, "1,5;2,6"},
		{"x86 n/n+1", New(0, 1, 2, 3), adj, "0,1;2,3"},
		{"partial cores", New(1, 2), x86, "1;2"},
		{"graviton: no siblings", New(2, 3), map[int]Set{2: New(2), 3: New(3)}, "2;3"},
		{"no topology at all", New(2, 3), nil, "2;3"},
	}
	for _, c := range cases {
		if got := FormatCores(Cores(c.cpus, c.siblings)); got != c.want {
			t.Errorf("%s: cores = %q, want %q", c.name, got, c.want)
		}
	}
}

// TestCheckCoreLayout: an exclusive CPU whose SMT sibling is shared is not
// isolated, and a CPU with no topology cannot be checked at all. Both must be
// refused; Cores would guess "a core of its own" for the second and silently
// accept the first.
func TestCheckCoreLayout(t *testing.T) {
	x86 := map[int]Set{}
	for c := 0; c < 4; c++ {
		x86[c] = New(c, c+4)
		x86[c+4] = New(c, c+4)
	}
	sibs := map[int]Set{1: New(1, 5), 2: New(2, 6), 5: New(1, 5), 6: New(2, 6)}
	cases := []struct {
		name      string
		exclusive Set
		shared    Set
		siblings  map[int]Set
		want      string
	}{
		{"whole cores, shared elsewhere", New(1, 5), New(0, 4), x86, ""},
		{"two cores", New(1, 2, 5, 6), New(0, 4), x86, ""},
		{"graviton: no SMT at all", New(2, 3), New(0), map[int]Set{0: New(0), 2: New(2), 3: New(3)}, ""},
		{"nothing to keep", Set{}, New(0, 4), x86, ""},
		{"one thread of a core, its sibling shared", New(1), New(5), x86, "split"},
		{"a core split the other way", New(5), New(1), x86, "split"},
		{"no topology for an exclusive CPU", New(1, 5), New(0), map[int]Set{0: New(0)}, "thread_siblings_list"},
		{"no topology for a shared CPU", New(1), New(0), map[int]Set{1: New(1, 5), 5: New(1, 5)}, "thread_siblings_list"},
		// SMT siblings 1/5 and 2/6; 5 and 6 are in neither set below.
		{"one thread of a core, its sibling unmanaged", New(1), New(2), sibs, "neither exclusive nor shared"},
		{"whole core exclusive, the other core's sibling unmanaged", New(1, 5), New(2), sibs, ""},
		{"whole core exclusive, nothing unmanaged", New(1, 5), New(2, 6), sibs, ""},
	}
	for _, c := range cases {
		err := CheckCoreLayout(c.exclusive, c.shared, c.siblings)
		if c.want == "" {
			if err != nil {
				t.Errorf("%s: err = %v, want nil", c.name, err)
			}
			continue
		}
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("%s: err = %v, want %q", c.name, err, c.want)
		}
	}
}

// TestCheckCoreLayoutNamesTheSplitCore: the refusal has to say which core and
// which CPUs, or an operator cannot pick another CPU without guessing.
func TestCheckCoreLayoutNamesTheSplitCore(t *testing.T) {
	x86 := map[int]Set{1: New(1, 5), 5: New(1, 5)}
	err := CheckCoreLayout(New(1), New(5), x86)
	if err == nil {
		t.Fatal("a split core must be refused")
	}
	for _, want := range []string{"1,5", "1", "5"} {
		if !strings.Contains(err.Error(), want) {
			t.Errorf("error %q does not mention %q", err, want)
		}
	}
}

// TestCheckCoreLayoutNamesTheUnmanagedSibling: an exclusive CPU whose SMT
// sibling is in neither set is not isolated either: the scheduler, the shared
// pool or another pod can run on the sibling and contend on the same physical
// core. The refusal has to name that CPU and say it is neither exclusive nor
// shared, or an operator cannot tell which CPU to look at.
func TestCheckCoreLayoutNamesTheUnmanagedSibling(t *testing.T) {
	sibs := map[int]Set{1: New(1, 5), 2: New(2, 6), 5: New(1, 5), 6: New(2, 6)}
	err := CheckCoreLayout(New(1), New(2), sibs)
	if err == nil {
		t.Fatal("an exclusive core with an unmanaged sibling must be refused")
	}
	for _, want := range []string{"1", "5", "neither exclusive nor shared", "not its own"} {
		if !strings.Contains(err.Error(), want) {
			t.Errorf("error %q does not mention %q", err, want)
		}
	}
}

// TestCheckCoreLayoutLeavesSharedAlone: only exclusive cores must be clean.
// A shared CPU is housekeeping, contention there is expected and fine, so a
// shared core whose sibling is unmanaged must be accepted.
func TestCheckCoreLayoutLeavesSharedAlone(t *testing.T) {
	sibs := map[int]Set{1: New(1, 5), 2: New(2, 6), 5: New(1, 5), 6: New(2, 6)}
	if err := CheckCoreLayout(New(1, 5), New(2), sibs); err != nil {
		t.Errorf("a shared core with an unmanaged sibling must be allowed: %v", err)
	}
}

func TestParseCores(t *testing.T) {
	cores, err := ParseCores("1,5;2,6")
	if err != nil {
		t.Fatal(err)
	}
	if len(cores) != 2 || cores[0].List() != "1,5" || cores[1].List() != "2,6" {
		t.Errorf("cores = %v", cores)
	}
	for _, bad := range []string{"", ";", "1,5;", "1;;2", "1;1", "a"} {
		if _, err := ParseCores(bad); err == nil {
			t.Errorf("ParseCores(%q) accepted", bad)
		}
	}
}

func TestReadSiblings(t *testing.T) {
	root := t.TempDir()
	write := func(cpu, text string) {
		dir := filepath.Join(root, "devices/system/cpu/cpu"+cpu+"/topology")
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "thread_siblings_list"), []byte(text), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	write("1", "1,5\n")
	write("5", "1,5\n")
	write("2", "2,6\n")
	// cpu3 has no topology file: it counts as a core of its own.
	sib, err := ReadSiblings(root, New(1, 2, 3, 5, 6))
	if err != nil {
		t.Fatal(err)
	}
	if got := FormatCores(Cores(New(1, 2, 3, 5, 6), sib)); got != "1,5;2,6;3" {
		t.Errorf("cores = %q", got)
	}
	write("7", "garbage\n")
	if _, err := ReadSiblings(root, New(7)); err == nil || !strings.Contains(err.Error(), "cpu7") {
		t.Errorf("bad sysfs file: err = %v", err)
	}
}
