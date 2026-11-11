// SPDX-License-Identifier: Apache-2.0

package main

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/wrap"
)

// readiness answers an httpGet readiness probe from inside the container.
// Placement is verified when cpuset.cpus.effective still holds the exclusive
// and shared CPUs the plugin recorded at create (wrap.FromEnv) and, for a
// cpu-partition pod, the isolated partition is valid. While the
// plugin is down a kubelet restart rewrites the cpuset to the exclusive CPUs
// only, and every helper falls onto them; the probe then fails until the
// plugin is back and has re-widened the cpuset. It checks on every request,
// so it never reports a stale verdict and does no work between probes.
type readiness struct {
	env        []string
	cgroupCPUs string
	sysRoot    string
	log        io.Writer

	mu sync.Mutex
	// ok is the last verdict logged; nil until the first probe.
	ok *bool
}

func newReadiness(env []string, cgroupCPUs, sysRoot string, log io.Writer) *readiness {
	return &readiness{env: env, cgroupCPUs: cgroupCPUs, sysRoot: sysRoot, log: log}
}

func (r *readiness) ServeHTTP(w http.ResponseWriter, _ *http.Request) {
	plan, err := wrap.FromEnv(r.env, r.cgroupCPUs, r.sysRoot)
	if err == nil {
		err = partitionValid(filepath.Join(filepath.Dir(r.cgroupCPUs), names.CpuPartitionChild))
	}
	var msg string
	if err != nil {
		msg = "placement broken: " + err.Error()
	} else {
		msg = fmt.Sprintf("placement verified: isolated=%s shared=%s cpuset=%s",
			plan.Isolated.List(), plan.Shared.List(), plan.Allowed.List())
	}
	ok := err == nil
	r.mu.Lock()
	if r.ok == nil || *r.ok != ok {
		r.ok = &ok
		// One line per change of state: the container log (with timestamps)
		// is the record of when placement broke and when it was verified.
		fmt.Fprintf(r.log, "ergon-cpus ready: %s\n", msg)
	}
	r.mu.Unlock()
	if !ok {
		w.WriteHeader(http.StatusServiceUnavailable)
	}
	fmt.Fprintln(w, msg)
}

// partitionValid fails while a cpu-partition pod's isolated child is not a
// valid partition. After an outage the container's cpuset can hold both lists
// again while the kernel still reports "isolated invalid (...)". A child with
// no CPUs is no partition, as in wrap.FromEnv.
func partitionValid(child string) error {
	cpus, err := os.ReadFile(filepath.Join(child, "cpuset.cpus"))
	if err != nil || strings.TrimSpace(string(cpus)) == "" {
		return nil
	}
	path := filepath.Join(child, "cpuset.cpus.partition")
	state, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("read the partition state: %w", err)
	}
	if s := strings.TrimSpace(string(state)); s != "isolated" {
		return fmt.Errorf("%s is %q, not isolated", path, s)
	}
	return nil
}

func runReady(args, env []string, stderr *os.File) int {
	if len(args) != 2 || args[0] != "--addr" || args[1] == "" {
		fmt.Fprintln(stderr, "ergon-cpus ready: usage: ergon-cpus ready --addr HOST:PORT")
		return 2
	}
	srv := &http.Server{
		Addr:              args[1],
		Handler:           newReadiness(env, wrap.CgroupCPUs, "/sys", stderr),
		ReadHeaderTimeout: 5 * time.Second,
	}
	fmt.Fprintf(stderr, "ergon-cpus ready: serving placement on %s\n", args[1])
	err := srv.ListenAndServe()
	fmt.Fprintf(stderr, "ergon-cpus ready: %v\n", err)
	return 1
}

// startReady starts `ergon-cpus ready` as a child before the wrapper execs
// the command. It is started from the wrapper's locked thread after that
// thread is on the shared CPUs, so the child and all its threads are there
// too. The child is not waited for: the command becomes its parent, and the
// kernel stops it with the rest of the container when PID 1 exits. If it
// dies, the probe is refused and the pod stays not ready: it fails closed.
func startReady(addr string, env []string) error {
	self, err := os.Executable()
	if err != nil {
		return fmt.Errorf("--ready-addr: find own binary: %w", err)
	}
	devNull, err := os.Open(os.DevNull)
	if err != nil {
		return fmt.Errorf("--ready-addr: %w", err)
	}
	defer devNull.Close()
	proc, err := os.StartProcess(self, []string{self, "ready", "--addr", addr}, &os.ProcAttr{
		Env:   env,
		Files: []*os.File{devNull, os.Stdout, os.Stderr},
	})
	if err != nil {
		return fmt.Errorf("--ready-addr: start the placement probe: %w", err)
	}
	return proc.Release()
}
