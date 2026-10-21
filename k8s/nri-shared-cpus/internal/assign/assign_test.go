// SPDX-License-Identifier: Apache-2.0

package assign

import (
	"reflect"
	"strings"
	"testing"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
)

func TestNeed(t *testing.T) {
	for milli, want := range map[int]int{1: 1, 250: 1, 999: 1, 1000: 1, 1001: 2, 1500: 2, 2000: 2, 2001: 3} {
		if got := Need(milli); got != want {
			t.Errorf("Need(%d) = %d, want %d", milli, got, want)
		}
	}
}

func TestSplit(t *testing.T) {
	got := Split(1001, cpuset.New(4, 2))
	if !reflect.DeepEqual(got, map[int]int{2: 501, 4: 500}) {
		t.Errorf("split = %v", got)
	}
	if got := Split(250, cpuset.New(0)); !reflect.DeepEqual(got, map[int]int{0: 250}) {
		t.Errorf("split = %v", got)
	}
}

func TestChooseBalancesAndTiesToLowerCPU(t *testing.T) {
	tb := New(cpuset.New(0, 1))
	pick := func(id, pod string, milli int) string {
		t.Helper()
		key := Key{PodUID: pod, Container: "app"}
		s, err := tb.Choose(key, Need(milli))
		if err != nil {
			t.Fatal(err)
		}
		tb.Hold(id, key, s, milli)
		return s.List()
	}
	// The spec's example: A -> 0, B -> 1, C -> 0.
	if got := pick("a", "pa", 250); got != "0" {
		t.Errorf("A = %s", got)
	}
	if got := pick("b", "pb", 250); got != "1" {
		t.Errorf("B = %s", got)
	}
	if got := pick("c", "pc", 250); got != "0" {
		t.Errorf("C = %s", got)
	}
	if got := tb.Load(); !reflect.DeepEqual(got, map[int]int{0: 500, 1: 250}) {
		t.Errorf("load = %v", got)
	}
	// Two CPUs for 1500.
	if got := pick("d", "pd", 1500); got != "0,1" {
		t.Errorf("D = %s", got)
	}
	if tb.Len() != 4 {
		t.Errorf("len = %d", tb.Len())
	}
}

func TestChooseRefusesTooMany(t *testing.T) {
	tb := New(cpuset.New(0, 1))
	if _, err := tb.Choose(Key{"p", "c"}, 3); err == nil || !strings.Contains(err.Error(), "2 shared") {
		t.Errorf("err = %v", err)
	}
	if _, err := tb.Choose(Key{"p", "c"}, 0); err == nil {
		t.Error("k = 0 accepted")
	}
}

func TestStickyAndRelease(t *testing.T) {
	tb := New(cpuset.New(0, 1))
	a := Key{"pa", "app"}
	s, _ := tb.Choose(a, 1)
	tb.Hold("a1", a, s, 250) // A on 0
	b := Key{"pb", "app"}
	s, _ = tb.Choose(b, 1)
	tb.Hold("b1", b, s, 250) // B on 1
	// A's container stops; its load goes away.
	if !tb.Release("a1") || tb.Release("a1") {
		t.Error("release is not idempotent")
	}
	if got := tb.Load(); !reflect.DeepEqual(got, map[int]int{0: 0, 1: 250}) {
		t.Errorf("load after release = %v", got)
	}
	// Load now favours 0 anyway; make 0 busier to prove stickiness wins.
	c := Key{"pc", "app"}
	tb.Hold("c1", c, cpuset.New(0), 900)
	s, err := tb.Choose(a, 1)
	if err != nil || s.List() != "0" {
		t.Errorf("sticky restart got %v, %v; want 0", s, err)
	}
	// A different container count does not reuse the sticky set.
	s, _ = tb.Choose(a, 2)
	if s.List() != "0,1" {
		t.Errorf("k=2 got %v", s)
	}
	// RemovePodSandbox forgets the pod: A now goes to the least loaded CPU.
	tb.ForgetPod("pa")
	s, _ = tb.Choose(a, 1)
	if s.List() != "1" {
		t.Errorf("after forget got %v, want 1", s)
	}
}

func TestStickyIgnoredWhenOutsideShared(t *testing.T) {
	tb := New(cpuset.New(0, 1))
	tb.Hold("x", Key{"p", "c"}, cpuset.New(5), 250)
	s, err := tb.Choose(Key{"p", "c"}, 1)
	if err != nil || s.List() != "0" {
		t.Errorf("got %v, %v", s, err)
	}
	if got := tb.Load(); got[5] != 250 {
		t.Errorf("a held CPU outside the shared set must still show its load: %v", got)
	}
}
