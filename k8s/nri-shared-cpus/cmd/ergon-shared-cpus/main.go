// SPDX-License-Identifier: Apache-2.0

// Command ergon-shared-cpus is one daemon with two faces:
//
//   - a containerd NRI plugin that widens an opted-in container's cpuset with
//     the node's shared housekeeping CPUs, sets the CPU env and injects the
//     start-affinity hook;
//   - the kubelet device plugin for ergon.io/shared-millicpus.
//
// A Guaranteed container asks cpu: N (N isolated CPUs from the kubelet's
// static CPU manager) and ergon.io/shared-millicpus: M (ceil(M/1000) shared
// CPUs). Every thread starts on the shared CPUs; only threads the app pins go
// to the isolated ones.
package main

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"runtime"
	"strings"
	"syscall"
	"time"

	"github.com/containerd/nri/pkg/stub"
	"golang.org/x/sys/unix"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/config"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/cpuset"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/deviceplugin"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/metrics"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/nriplugin"
)

// version and gitDescribe are set with -ldflags at build time.
var (
	version     = "dev"
	gitDescribe = ""
	// nriModuleVersion is the containerd/nri module version this binary was
	// built against. Changing it is a contract change; see the README.
	nriModuleVersion = "v0.12.3"
)

func main() {
	configPath := flag.String("config", "/etc/ergon-shared-cpus/config.yaml", "config file")
	showVersion := flag.Bool("version", false, "print the version and exit")
	flag.Parse()
	log := slog.New(slog.NewJSONHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelInfo}))
	if *showVersion {
		fmt.Printf("ergon-shared-cpus %s %s nri=%s\n", version, gitDescribe, nriModuleVersion)
		return
	}
	if err := run(*configPath, log); err != nil {
		log.Error("exit", "error", err.Error())
		os.Exit(1)
	}
}

func run(configPath string, log *slog.Logger) error {
	cfg, err := config.Load(configPath)
	if err != nil {
		return err
	}
	warnings, err := config.CheckHost(cfg, "/sys", "/sys/fs/cgroup", kernelRelease())
	if err != nil {
		return err
	}
	for _, w := range warnings {
		log.Warn("config warning", "warning", w)
	}
	// The device grants need the node's PHC and PM QoS nodes and the arch.
	// Fail closed: a plugin that cannot see the host's /dev must not pretend it
	// can grant a device (the numbers would be 0,0 and the open would fail).
	if err := config.DiscoverHostDevices(cfg, runtime.GOARCH); err != nil {
		return err
	}
	log.Info("host devices", "devRoot", cfg.DevRoot, "ptp", cfg.HostPtp,
		"cpu_dma_latency", cfg.HostCpuDmaLatency, "goarch", cfg.GOARCH)
	// The numa.ergon.io grant is chosen from the node's NUMA nodes. Without
	// this the list stays empty and every such request is answered with
	// nothing bound, in silence.
	if err := config.DiscoverHostNuma(cfg, "/sys"); err != nil {
		return err
	}
	log.Info("host numa", "nodes", cfg.NodeNumaNodes, "nicNode", cfg.NicNumaNode)

	if err := installHook(cfg.HookPath, log); err != nil {
		return err
	}
	pinSelfToShared(cfg, log)

	// The topology covers every online CPU, not just the shared ones: the
	// plugin groups a container's exclusive CPUs by core too, and a map built
	// from shared alone has no entry for any of them. Cores then calls each
	// exclusive CPU a physical core of its own, so two threads of one core
	// look like two cores and an app can put two "separate" hot cores on one.
	onlineData, err := os.ReadFile("/sys/devices/system/cpu/online")
	if err != nil {
		return fmt.Errorf("core topology: %w", err)
	}
	online, err := cpuset.Parse(strings.TrimSpace(string(onlineData)))
	if err != nil {
		return fmt.Errorf("core topology: /sys/devices/system/cpu/online %q: %w", onlineData, err)
	}
	siblings, err := cpuset.ReadSiblings("/sys", online)
	if err != nil {
		return fmt.Errorf("core topology: %w", err)
	}
	m := metrics.New()
	p := nriplugin.New(cfg, siblings, log, m)
	dp := deviceplugin.New(cfg.Capacity(), log)
	// Readiness and device health follow the two connections: the kubelet
	// registration opens /readyz's registered half and keeps it open across
	// re-serves; each NRI connection opens the connected half and marks the
	// devices healthy again.
	dp.OnRegistered(func() { m.SetRegistered(true) })
	p.OnConnected(func() { dp.SetHealthy(true) })
	dp.SetHealthy(false)

	ctx, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()

	go serveHTTP(ctx, cfg.MetricsAddress, m, log)
	return runNRI(ctx, cfg, p, dp, m, log)
}

// runNRI connects to the NRI socket, reconnects on a drop and serves the
// device plugin for as long as the process lives. Device health follows the
// NRI connection: while it is down the kubelet allocates nothing.
func runNRI(ctx context.Context, cfg *config.Config, p *nriplugin.Plugin, dp *deviceplugin.Plugin, m *metrics.Metrics, log *slog.Logger) error {
	stop, err := dp.Serve(ctx, names.Resource)
	if err != nil {
		return fmt.Errorf("device plugin: %w", err)
	}
	defer stop()
	go dp.WatchSocket(ctx, names.Resource)

	s, err := stub.New(p,
		stub.WithPluginName(names.PluginName),
		stub.WithPluginIdx(names.PluginIdx),
		stub.WithOnClose(func() {
			m.SetNRIConnected(false)
			dp.SetHealthy(false)
			log.Warn("nri disconnected")
		}),
	)
	if err != nil {
		return fmt.Errorf("nri stub: %w", err)
	}
	p.SetResender(s.UpdateContainers)

	for {
		if err := ctx.Err(); err != nil {
			return nil
		}
		if err := s.Run(ctx); err != nil && !errors.Is(err, context.Canceled) {
			log.Error("nri run", "error", err.Error())
		}
		m.SetNRIConnected(false)
		dp.SetHealthy(false)
		select {
		case <-ctx.Done():
			return nil
		case <-time.After(time.Second):
		}
		log.Info("nri reconnecting")
	}
}

func serveHTTP(ctx context.Context, addr string, m *metrics.Metrics, log *slog.Logger) {
	srv := &http.Server{Addr: addr, Handler: m.Handler()}
	go func() {
		<-ctx.Done()
		_ = srv.Close()
	}()
	log.Info("metrics listening", "address", addr)
	if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
		log.Error("metrics server", "error", err.Error())
	}
}

// installHook writes the ergon-cpus binary to hookPath atomically and logs
// its SHA-256. runc execs it as root on every opted-in create.
func installHook(hookPath string, log *slog.Logger) error {
	self, err := os.Executable()
	if err != nil {
		return fmt.Errorf("find own binary: %w", err)
	}
	src := filepath.Join(filepath.Dir(self), "ergon-cpus")
	if filepath.Base(self) == "ergon-cpus" {
		src = self
	}
	data, err := os.ReadFile(src)
	if err != nil {
		return fmt.Errorf("read hook binary %s: %w", src, err)
	}
	if err := os.MkdirAll(filepath.Dir(hookPath), 0o755); err != nil {
		return fmt.Errorf("hook dir: %w", err)
	}
	tmp := hookPath + ".tmp"
	if err := os.WriteFile(tmp, data, 0o755); err != nil {
		return fmt.Errorf("write hook: %w", err)
	}
	if f, err := os.Open(tmp); err == nil {
		_ = f.Sync()
		_ = f.Close()
	}
	if err := os.Rename(tmp, hookPath); err != nil {
		return fmt.Errorf("install hook: %w", err)
	}
	sum := sha256.Sum256(data)
	log.Info("hook installed", "path", hookPath, "sha256", hex.EncodeToString(sum[:]), "bytes", len(data))
	return nil
}

// pinSelfToShared keeps our own threads off the exclusive CPUs. If the shared
// set is outside our cpuset (a misconfigured pod) we only log it.
func pinSelfToShared(cfg *config.Config, log *slog.Logger) {
	var mask unix.CPUSet
	for _, c := range cfg.Shared.Slice() {
		if c >= 1024 {
			log.Warn("shared CPU is outside cpu_set_t", "cpu", c)
			return
		}
		mask.Set(c)
	}
	if err := unix.SchedSetaffinity(0, &mask); err != nil {
		log.Warn("cannot pin to the shared CPUs", "shared", cfg.Shared.List(), "error", err.Error())
		return
	}
	log.Info("pinned to the shared CPUs", "shared", cfg.Shared.List())
}

func kernelRelease() string {
	var uts unix.Utsname
	if err := unix.Uname(&uts); err != nil {
		return ""
	}
	var b strings.Builder
	for _, c := range uts.Release {
		if c == 0 {
			break
		}
		b.WriteByte(byte(c))
	}
	return b.String()
}
