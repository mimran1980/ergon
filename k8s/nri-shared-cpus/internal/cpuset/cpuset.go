// SPDX-License-Identifier: Apache-2.0

// Package cpuset parses and formats Linux CPU lists ("0-3,8" or "1,5") and
// groups CPUs by physical core.
package cpuset

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"slices"
	"strconv"
	"strings"
)

// MaxCPU is the largest CPU number accepted (the kernel's NR_CPUS limit).
const MaxCPU = 8191

// Set is an immutable, sorted set of CPU numbers. The zero value is empty.
type Set struct {
	cpus []int
}

// New returns the set of the given CPUs.
func New(cpus ...int) Set {
	c := slices.Clone(cpus)
	slices.Sort(c)
	return Set{cpus: slices.Compact(c)}
}

// Parse reads a kernel CPU list such as "0-3,8" or "1,5". White space around
// the list (a newline from sysfs) is ignored. An empty or malformed list is an
// error, never an empty set.
func Parse(s string) (Set, error) {
	text := strings.TrimSpace(s)
	if text == "" {
		return Set{}, fmt.Errorf("cpu list %q is empty", s)
	}
	var cpus []int
	for _, part := range strings.Split(text, ",") {
		lo, hi, isRange := strings.Cut(part, "-")
		first, err := parseCPU(lo)
		if err != nil {
			return Set{}, fmt.Errorf("cpu list %q: %w", s, err)
		}
		last := first
		if isRange {
			if last, err = parseCPU(hi); err != nil {
				return Set{}, fmt.Errorf("cpu list %q: %w", s, err)
			}
			if last < first {
				return Set{}, fmt.Errorf("cpu list %q: range %s is backwards", s, part)
			}
		}
		for c := first; c <= last; c++ {
			cpus = append(cpus, c)
		}
	}
	return New(cpus...), nil
}

func parseCPU(s string) (int, error) {
	if s == "" {
		return 0, errors.New("missing cpu number")
	}
	for _, r := range s {
		if r < '0' || r > '9' {
			return 0, fmt.Errorf("%q is not a cpu number", s)
		}
	}
	n, err := strconv.Atoi(s)
	if err != nil || n > MaxCPU {
		return 0, fmt.Errorf("cpu %q is out of range 0-%d", s, MaxCPU)
	}
	return n, nil
}

// String formats the set as a kernel CPU list with ranges: "1-2,5-6".
func (s Set) String() string {
	var b strings.Builder
	for i := 0; i < len(s.cpus); {
		j := i
		for j+1 < len(s.cpus) && s.cpus[j+1] == s.cpus[j]+1 {
			j++
		}
		if b.Len() > 0 {
			b.WriteByte(',')
		}
		b.WriteString(strconv.Itoa(s.cpus[i]))
		if j > i {
			b.WriteByte('-')
			b.WriteString(strconv.Itoa(s.cpus[j]))
		}
		i = j + 1
	}
	return b.String()
}

// List formats the set as a sorted comma list without ranges: "1,2,5,6".
func (s Set) List() string {
	parts := make([]string, len(s.cpus))
	for i, c := range s.cpus {
		parts[i] = strconv.Itoa(c)
	}
	return strings.Join(parts, ",")
}

// Slice returns the CPUs in ascending order, as a copy.
func (s Set) Slice() []int { return slices.Clone(s.cpus) }

// Len is the number of CPUs.
func (s Set) Len() int { return len(s.cpus) }

// IsEmpty reports whether the set has no CPU.
func (s Set) IsEmpty() bool { return len(s.cpus) == 0 }

// Contains reports whether cpu is in the set.
func (s Set) Contains(cpu int) bool {
	_, found := slices.BinarySearch(s.cpus, cpu)
	return found
}

// Union returns the CPUs in s or o.
func (s Set) Union(o Set) Set {
	return New(append(slices.Clone(s.cpus), o.cpus...)...)
}

// Intersection returns the CPUs in both s and o.
func (s Set) Intersection(o Set) Set {
	var out []int
	for _, c := range s.cpus {
		if o.Contains(c) {
			out = append(out, c)
		}
	}
	return Set{cpus: out}
}

// Difference returns the CPUs in s and not in o.
func (s Set) Difference(o Set) Set {
	var out []int
	for _, c := range s.cpus {
		if !o.Contains(c) {
			out = append(out, c)
		}
	}
	return Set{cpus: out}
}

// IsSubsetOf reports whether every CPU of s is in o.
func (s Set) IsSubsetOf(o Set) bool {
	for _, c := range s.cpus {
		if !o.Contains(c) {
			return false
		}
	}
	return true
}

// Equals reports whether s and o hold the same CPUs.
func (s Set) Equals(o Set) bool { return slices.Equal(s.cpus, o.cpus) }

// Cores groups the CPUs of s by physical core. siblings maps a CPU to all CPUs
// of its core (sysfs thread_siblings_list); a CPU missing from it is a core of
// its own. Each group holds only CPUs of s. Groups are ordered by their lowest
// CPU.
func Cores(s Set, siblings map[int]Set) []Set {
	var cores []Set
	seen := map[int]bool{}
	for _, c := range s.cpus {
		if seen[c] {
			continue
		}
		group := New(c)
		if sib, ok := siblings[c]; ok {
			group = sib.Intersection(s).Union(group)
		}
		for _, g := range group.cpus {
			seen[g] = true
		}
		cores = append(cores, group)
	}
	return cores
}

// CheckCoreLayout reports whether exclusive can be given its own physical
// cores alongside shared. On SMT the two threads of one core share every
// execution unit, so an exclusive CPU whose sibling is shared is not isolated
// at all: the helpers still contend with the hot thread on the same core, and
// that is the very noise a partition exists to remove.
//
// The same is true of a sibling in neither set. If CPU 1 is exclusive and its
// sibling CPU 5 is neither exclusive nor shared, then CPU 5 is unmanaged: the
// scheduler, the shared pool or any other pod can run work there and contend
// with the hot thread on the same physical core. So an exclusive core is only
// accepted when every thread of it is exclusive.
//
// Only exclusive cores are held to that. A core of shared whose sibling is in
// neither set is fine: shared CPUs are housekeeping, and contention there is
// expected. Do not "fix" that.
//
// A CPU with no thread_siblings_list is refused too. Cores would call it a
// core of its own, which is a guess: that file is how the kernel says which
// threads share a core, and without it the layout cannot be checked.
//
// siblings maps a CPU to every CPU of its core. Build it with ReadSiblings
// over all the CPUs under consideration — exclusive and shared both. A map
// built from shared alone leaves every exclusive CPU unlisted, and Cores then
// calls each of them a separate physical core even when two of them are the
// two threads of one.
func CheckCoreLayout(exclusive, shared Set, siblings map[int]Set) error {
	for _, c := range exclusive.Union(shared).Slice() {
		if _, ok := siblings[c]; !ok {
			return fmt.Errorf("cpu %d has no thread_siblings_list, so its core cannot be found: the topology is incomplete", c)
		}
	}
	for _, core := range Cores(exclusive, siblings) {
		for _, c := range core.Slice() {
			sib := siblings[c]
			if both := sib.Intersection(shared); !both.IsEmpty() {
				return fmt.Errorf("core %s is split: CPU %d is exclusive and CPU %s is shared, so the hot thread and the helpers time-share one physical core", FormatCores([]Set{sib}), c, both.List())
			}
			if unmanaged := sib.Difference(exclusive).Difference(shared); !unmanaged.IsEmpty() {
				return fmt.Errorf("core %s is not its own: CPU %d is exclusive and CPU %s is neither exclusive nor shared, so anything can run on the same physical core as the hot thread", FormatCores([]Set{sib}), c, unmanaged.List())
			}
		}
	}
	return nil
}

// FormatCores writes core groups as "1,5;2,6": groups by ';', CPUs by ','.
func FormatCores(cores []Set) string {
	parts := make([]string, len(cores))
	for i, c := range cores {
		parts[i] = c.List()
	}
	return strings.Join(parts, ";")
}

// ParseCores reads the FormatCores format. A CPU may appear in one group only.
func ParseCores(s string) ([]Set, error) {
	if strings.TrimSpace(s) == "" {
		return nil, fmt.Errorf("core list %q is empty", s)
	}
	var cores []Set
	seen := map[int]bool{}
	for _, part := range strings.Split(strings.TrimSpace(s), ";") {
		group, err := Parse(part)
		if err != nil {
			return nil, fmt.Errorf("core list %q: %w", s, err)
		}
		for _, c := range group.cpus {
			if seen[c] {
				return nil, fmt.Errorf("core list %q: cpu %d is in two groups", s, c)
			}
			seen[c] = true
		}
		cores = append(cores, group)
	}
	return cores, nil
}

// ReadSiblings reads devices/system/cpu/cpu<N>/topology/thread_siblings_list
// under sysRoot for every CPU of cpus. A CPU without the file is left out (it
// then counts as a core of its own); a file that does not parse is an error.
func ReadSiblings(sysRoot string, cpus Set) (map[int]Set, error) {
	out := map[int]Set{}
	for _, c := range cpus.cpus {
		path := filepath.Join(sysRoot, "devices/system/cpu", "cpu"+strconv.Itoa(c), "topology/thread_siblings_list")
		data, err := os.ReadFile(path)
		if errors.Is(err, fs.ErrNotExist) {
			continue
		}
		if err != nil {
			return nil, fmt.Errorf("cpu%d siblings: %w", c, err)
		}
		sib, err := Parse(string(data))
		if err != nil {
			return nil, fmt.Errorf("cpu%d siblings: %w", c, err)
		}
		out[c] = sib
	}
	return out, nil
}
