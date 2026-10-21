// SPDX-License-Identifier: Apache-2.0

// Command ergon-cpus is the small tool the DaemonSet installs on the node and
// app images carry as an entrypoint.
//
//	ergon-cpus hook --shared S          the OCI createRuntime hook
//	ergon-cpus hook --isolated E --partition-child P   build the isolated child
//	ergon-cpus hook --isolated E --release-child P     take it down (poststop)
//	ergon-cpus [--env NAME=SPEC]... [--affinity shared|keep] [--fifo PRIO] -- cmd [args...]
//	ergon-cpus show                     print the container's CPU plan
//
// An app never writes a real CPU number: it pins its hot threads by index into
// ERGON_ISOLATED_CPUS and leaves every helper on the shared CPUs. This tool
// turns index specs into real numbers for programs that want them (the Aeron
// media driver, Java affinity libraries).
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"runtime"
	"strconv"
	"strings"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/hook"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/wrap"
)

// version is set with -ldflags at build time.
var version = "dev"

// init locks the main goroutine to its OS thread: execve keeps the affinity
// mask of the calling thread, so the whole wrapper must stay on one thread.
func init() { runtime.LockOSThread() }

func main() {
	os.Exit(run(os.Args[1:], os.Environ(), os.Stdin, os.Stdout, os.Stderr))
}

func run(args, env []string, stdin *os.File, stdout, stderr *os.File) int {
	if len(args) == 0 {
		usage(stderr)
		return 2
	}
	switch args[0] {
	case "hook":
		return runHook(args[1:], stdin, stderr)
	case "show":
		return runShow(env, stdout, stderr)
	case "version", "--version":
		fmt.Fprintf(stdout, "ergon-cpus %s\n", version)
		return 0
	case "help", "-h", "--help":
		usage(stdout)
		return 0
	case "--":
		return runWrap(args[1:], env, stderr)
	default:
		if strings.HasPrefix(args[0], "-") {
			return runWrap(args, env, stderr)
		}
		fmt.Fprintf(stderr, "ergon-cpus: unknown command %q\n", args[0])
		usage(stderr)
		return 2
	}
}

func usage(w *os.File) {
	fmt.Fprint(w, `ergon-cpus: pin by index, run helpers on the shared CPUs

  ergon-cpus hook --shared S                    OCI createRuntime hook (on the node)
  ergon-cpus hook --isolated E --partition-child P   build the isolated child cgroup
  ergon-cpus hook --isolated E --release-child P     take it back down (poststop)
  ergon-cpus show                               print the container's CPU plan as JSON
  ergon-cpus [--env NAME=SPEC]... [--affinity shared|keep] [--fifo PRIO] -- cmd [args...]

SPEC values for --env:
  isolated:<i>    the i-th CPU of ERGON_ISOLATED_CPUS
  shared:<i>      the i-th CPU of ERGON_SHARED_CPUS
  core:<j>        the first CPU of core group j
  core:<j>:<t>    CPU t of core group j
  isolated        the whole isolated list
  shared          the whole shared list

Example (Aeron media driver, Graviton, cpu: 2):
  ergon-cpus --env AERON_SENDER_CPU_AFFINITY=isolated:0 \
             --env AERON_RECEIVER_CPU_AFFINITY=isolated:1 -- aeronmd
`)
}

func runHook(args []string, stdin *os.File, stderr *os.File) int {
	shared, part, err := hook.ParseArgs(args)
	if err != nil {
		fmt.Fprintln(stderr, err)
		return 2
	}
	if err := hook.Run(stdin, stderr, shared, part); err != nil {
		fmt.Fprintln(stderr, err)
		return 1
	}
	return 0
}

func runShow(env []string, stdout, stderr *os.File) int {
	plan, err := wrap.FromEnv(env, wrap.CgroupCPUs, "/sys")
	if err != nil {
		fmt.Fprintf(stderr, "ergon-cpus show: %v\n", err)
		return 1
	}
	self, err := wrap.SelfAffinity()
	if err != nil {
		fmt.Fprintf(stderr, "ergon-cpus show: own affinity: %v\n", err)
		return 1
	}
	out := map[string]any{
		"isolated":              plan.Isolated.List(),
		"shared":                plan.Shared.List(),
		"isolated_cores":        cpuset.FormatCores(plan.Cores),
		"shared_millicpus":      plan.Milli,
		"cpuset_cpus_effective": plan.Allowed.String(),
		"own_affinity":          self.List(),
	}
	enc := json.NewEncoder(stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(out); err != nil {
		fmt.Fprintf(stderr, "ergon-cpus show: %v\n", err)
		return 1
	}
	return 0
}

func runWrap(args []string, env []string, stderr *os.File) int {
	var bindings []wrap.EnvBinding
	affinity := "shared"
	fifoPrio := 0
	for i := 0; i < len(args); i++ {
		a := args[i]
		switch {
		case a == "--":
			return execRest(args[i+1:], env, bindings, affinity, fifoPrio, stderr)
		case a == "--env":
			if i+1 >= len(args) {
				fmt.Fprintln(stderr, "ergon-cpus: --env needs NAME=SPEC")
				return 2
			}
			name, spec, ok := strings.Cut(args[i+1], "=")
			if !ok || name == "" {
				fmt.Fprintf(stderr, "ergon-cpus: --env %q is not NAME=SPEC\n", args[i+1])
				return 2
			}
			bindings = append(bindings, wrap.EnvBinding{Name: name, Spec: spec})
			i++
		case a == "--affinity":
			if i+1 >= len(args) {
				fmt.Fprintln(stderr, "ergon-cpus: --affinity needs shared or keep")
				return 2
			}
			affinity = args[i+1]
			if affinity != "shared" && affinity != "keep" {
				fmt.Fprintf(stderr, "ergon-cpus: --affinity %q must be shared or keep\n", affinity)
				return 2
			}
			i++
		case a == "--fifo":
			if i+1 >= len(args) {
				fmt.Fprintln(stderr, "ergon-cpus: --fifo needs a priority 1..99")
				return 2
			}
			p, err := strconv.Atoi(args[i+1])
			if err != nil || p < 1 || p > 99 {
				fmt.Fprintf(stderr, "ergon-cpus: --fifo %q must be a priority 1..99\n", args[i+1])
				return 2
			}
			fifoPrio = p
			i++
		case a == "--version":
			fmt.Printf("ergon-cpus %s\n", version)
			return 0
		default:
			fmt.Fprintf(stderr, "ergon-cpus: unknown argument %q (put the command after --)\n", a)
			return 2
		}
	}
	fmt.Fprintln(stderr, "ergon-cpus: no command after --")
	return 2
}

func execRest(argv, env []string, bindings []wrap.EnvBinding, affinity string, fifoPrio int, stderr *os.File) int {
	if len(argv) == 0 {
		fmt.Fprintln(stderr, "ergon-cpus: no command after --")
		return 2
	}
	plan, err := wrap.FromEnv(env, wrap.CgroupCPUs, "/sys")
	if err != nil {
		fmt.Fprintf(stderr, "ergon-cpus: %v\n", err)
		return 1
	}
	assigns, err := plan.Translate(env, bindings)
	if err != nil {
		fmt.Fprintf(stderr, "ergon-cpus: %v\n", err)
		return 1
	}
	if affinity == "shared" {
		// init() already locked this goroutine to its OS thread.
		if err := wrap.PinSelf(plan.Shared); err != nil {
			fmt.Fprintf(stderr, "ergon-cpus: start on the shared CPUs %s: %v\n", plan.Shared.List(), err)
			return 1
		}
	}
	if fifoPrio > 0 {
		if err := wrap.SetFifo(fifoPrio); err != nil {
			fmt.Fprintf(stderr, "ergon-cpus: %v\n", err)
			return 1
		}
	}
	newEnv := mergeEnv(env, assigns)
	if err := wrap.Exec(argv, newEnv); err != nil {
		fmt.Fprintf(stderr, "ergon-cpus: %v\n", err)
		return 1
	}
	return 0
}

func mergeEnv(env, assigns []string) []string {
	replaced := map[string]string{}
	for _, a := range assigns {
		k, v, _ := strings.Cut(a, "=")
		replaced[k] = v
	}
	out := make([]string, 0, len(env)+len(assigns))
	seen := map[string]bool{}
	for _, e := range env {
		k, _, ok := strings.Cut(e, "=")
		if !ok {
			continue
		}
		if v, hit := replaced[k]; hit {
			out = append(out, k+"="+v)
			seen[k] = true
			continue
		}
		out = append(out, e)
	}
	for _, a := range assigns {
		k, _, _ := strings.Cut(a, "=")
		if !seen[k] {
			out = append(out, a)
		}
	}
	return out
}
