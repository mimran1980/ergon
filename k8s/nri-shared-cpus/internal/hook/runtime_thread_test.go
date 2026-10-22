// SPDX-License-Identifier: Apache-2.0

package hook

import (
	"os"
	"runtime"
	"testing"

	"golang.org/x/sys/unix"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
)

// TestGoRuntimeThreadAffinityAfterHotPin pins this process with the shipped
// hook, moves the locked thread onto another CPU, and reads the mask of a
// new Go-created OS thread. The result is logged. Go's runtime creates that
// thread; it is not pthread_create from the hot thread.
func TestGoRuntimeThreadAffinityAfterHotPin(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("sched_setaffinity is a Linux test")
	}
	self := ownCPUs(t)
	cpus := self.Slice()
	if len(cpus) < 2 {
		t.Skip("need two CPUs in this process mask")
	}
	saved := self
	t.Cleanup(func() {
		if err := PinProcess(os.Getpid(), saved); err != nil {
			t.Errorf("restore affinity: %v", err)
		}
	})

	shared := cpuset.New(cpus[0])
	hot := cpus[1]
	if err := PinProcess(os.Getpid(), shared); err != nil {
		t.Fatalf("PinProcess: %v", err)
	}
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	var hotSet unix.CPUSet
	hotSet.Set(hot)
	if err := unix.SchedSetaffinity(0, &hotSet); err != nil {
		t.Fatalf("pin this thread to %d: %v", hot, err)
	}
	var mine unix.CPUSet
	if err := unix.SchedGetaffinity(0, &mine); err != nil {
		t.Fatal(err)
	}
	if !mine.IsSet(hot) || mine.Count() != 1 {
		t.Fatalf("locked thread mask is not only CPU %d", hot)
	}

	got := make(chan []int, 1)
	go func() {
		runtime.LockOSThread()
		var mask unix.CPUSet
		if err := unix.SchedGetaffinity(0, &mask); err != nil {
			got <- nil
			return
		}
		var cpus []int
		for c := 0; c < 256; c++ {
			if mask.IsSet(c) {
				cpus = append(cpus, c)
			}
		}
		got <- cpus
	}()
	child := <-got
	if len(child) != 1 || child[0] != cpus[0] {
		t.Fatalf("go new thread allowed %v, want only the shared CPU %d (hot CPU %d)", child, cpus[0], hot)
	}
	t.Logf("go new thread allowed %v; hot CPU %d; shared CPU %d; inherited hot mask false", child, hot, cpus[0])
}
