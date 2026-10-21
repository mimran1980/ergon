// SPDX-License-Identifier: Apache-2.0

// Package assign picks which shared CPUs a container gets and keeps the load
// table. It is not safe for concurrent use; the caller holds a lock.
package assign

import (
	"fmt"
	"slices"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
)

// Key names a container across restarts: its pod's UID and its name.
type Key struct {
	PodUID    string
	Container string
}

type holder struct {
	key   Key
	cpus  cpuset.Set
	milli int
}

// Table is the load on each shared CPU: the millicpus of every container
// that holds it.
type Table struct {
	shared cpuset.Set
	byID   map[string]holder
	sticky map[Key]cpuset.Set
}

// New returns an empty table over the node's shared CPUs.
func New(shared cpuset.Set) *Table {
	return &Table{shared: shared, byID: map[string]holder{}, sticky: map[Key]cpuset.Set{}}
}

// Need is the number of shared CPUs for milli millicpus: ceil(milli/1000),
// at least one.
func Need(milli int) int {
	if milli <= 1000 {
		return 1
	}
	return (milli + 999) / 1000
}

// Split spreads milli over cpus: each gets milli/len, and the lowest
// milli%len CPUs get one more.
func Split(milli int, cpus cpuset.Set) map[int]int {
	out := map[int]int{}
	n := cpus.Len()
	if n == 0 {
		return out
	}
	for i, c := range cpus.Slice() {
		out[c] = milli / n
		if i < milli%n {
			out[c]++
		}
	}
	return out
}

// Load returns the millicpus on every shared CPU, and on any other CPU a
// running container still holds (after a config change).
func (t *Table) Load() map[int]int {
	load := map[int]int{}
	for _, c := range t.shared.Slice() {
		load[c] = 0
	}
	for _, h := range t.byID {
		for c, m := range Split(h.milli, h.cpus) {
			load[c] += m
		}
	}
	return load
}

// Len is the number of containers held.
func (t *Table) Len() int { return len(t.byID) }

// Choose returns k shared CPUs for key: the set it had before, if the pod
// still exists and the set fits; else the k least-loaded CPUs, ties to the
// lower CPU number.
func (t *Table) Choose(key Key, k int) (cpuset.Set, error) {
	if k < 1 || k > t.shared.Len() {
		return cpuset.Set{}, fmt.Errorf("%d shared CPUs needed; this node has %d shared CPUs (%s)", k, t.shared.Len(), t.shared)
	}
	if s, ok := t.sticky[key]; ok && s.Len() == k && s.IsSubsetOf(t.shared) {
		return s, nil
	}
	load := t.Load()
	cpus := t.shared.Slice()
	slices.SortStableFunc(cpus, func(a, b int) int {
		if load[a] != load[b] {
			return load[a] - load[b]
		}
		return a - b
	})
	return cpuset.New(cpus[:k]...), nil
}

// Hold records that container id (key) holds cpus with milli millicpus.
// Holding the same id again replaces the old record.
func (t *Table) Hold(id string, key Key, cpus cpuset.Set, milli int) {
	t.byID[id] = holder{key: key, cpus: cpus, milli: milli}
	t.sticky[key] = cpus
}

// Release drops container id's load. It reports whether id was held.
func (t *Table) Release(id string) bool {
	_, ok := t.byID[id]
	delete(t.byID, id)
	return ok
}

// ForgetPod drops the remembered sets of every container of the pod.
func (t *Table) ForgetPod(podUID string) {
	for k := range t.sticky {
		if k.PodUID == podUID {
			delete(t.sticky, k)
		}
	}
}
