// SPDX-License-Identifier: Apache-2.0

// Package deviceplugin is the kubelet device plugin for
// ergon.io/shared-millicpus. It advertises one device ID per millicpu of
// shared-CPU capacity and tags each allocated container so the NRI plugin can
// see how many millicpus it asked for.
package deviceplugin

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net"
	"os"
	"path/filepath"
	"sync"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/health"
	healthpb "google.golang.org/grpc/health/grpc_health_v1"

	pluginapi "k8s.io/kubelet/pkg/apis/deviceplugin/v1beta1"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

const (
	// SocketName is our socket file name inside the kubelet's device plugin
	// directory.
	SocketName = "ergon-shared-cpus.sock"
	// ResocketPeriod is how often we check that our socket still exists: the
	// kubelet deletes every device plugin socket when it restarts.
	ResocketPeriod = time.Second
	// MaxDeviceMessage is the kubelet's default gRPC receive limit (4 MiB).
	// One ListAndWatch reply must stay under it.
	MaxDeviceMessage = 4 << 20
)

// Plugin serves the device plugin API.
type Plugin struct {
	pluginapi.UnimplementedDevicePluginServer
	capacity int
	log      *slog.Logger

	mu           sync.Mutex
	healthy      bool
	listeners    []chan *pluginapi.ListAndWatchResponse
	onRegistered func()
}

// New returns a device plugin advertising n device IDs (millicpus).
func New(capacity int, log *slog.Logger) *Plugin {
	return &Plugin{capacity: capacity, log: log, healthy: true}
}

// Capacity is the number of advertised device IDs.
func (p *Plugin) Capacity() int { return p.capacity }

// OnRegistered sets a function called after each successful registration
// with the kubelet (first serve and every re-serve after a kubelet restart).
func (p *Plugin) OnRegistered(f func()) {
	p.mu.Lock()
	p.onRegistered = f
	p.mu.Unlock()
}

// IDs returns the stable device IDs: m000000, m000001, ... A restart keeps
// them valid for the kubelet's checkpointed allocations.
func (p *Plugin) IDs() []string {
	ids := make([]string, p.capacity)
	for i := range ids {
		ids[i] = fmt.Sprintf("m%06d", i)
	}
	return ids
}

// Devices returns the device list with the current health.
func (p *Plugin) Devices() []*pluginapi.Device {
	p.mu.Lock()
	health := pluginapi.Healthy
	if !p.healthy {
		health = pluginapi.Unhealthy
	}
	p.mu.Unlock()
	ids := p.IDs()
	devs := make([]*pluginapi.Device, len(ids))
	for i, id := range ids {
		devs[i] = &pluginapi.Device{ID: id, Health: health}
	}
	return devs
}

// SetHealthy flips every device's health. While the NRI connection is down
// the devices are Unhealthy, node allocatable drops to 0 and new pods go
// elsewhere.
func (p *Plugin) SetHealthy(v bool) {
	p.mu.Lock()
	changed := p.healthy != v
	p.healthy = v
	var listeners []chan *pluginapi.ListAndWatchResponse
	if changed {
		listeners = append(listeners, p.listeners...)
	}
	p.mu.Unlock()
	if !changed {
		return
	}
	resp := &pluginapi.ListAndWatchResponse{Devices: p.Devices()}
	for _, ch := range listeners {
		select {
		case ch <- resp:
		default:
		}
	}
}

// GetDevicePluginOptions: no PreStartContainer, no preferred allocation.
func (p *Plugin) GetDevicePluginOptions(context.Context, *pluginapi.Empty) (*pluginapi.DevicePluginOptions, error) {
	return &pluginapi.DevicePluginOptions{}, nil
}

// ListAndWatch streams the device list. The first reply goes out at once;
// later ones follow a health change.
func (p *Plugin) ListAndWatch(_ *pluginapi.Empty, stream pluginapi.DevicePlugin_ListAndWatchServer) error {
	ch := make(chan *pluginapi.ListAndWatchResponse, 1)
	p.mu.Lock()
	p.listeners = append(p.listeners, ch)
	p.mu.Unlock()
	defer func() {
		p.mu.Lock()
		for i, c := range p.listeners {
			if c == ch {
				p.listeners = append(p.listeners[:i], p.listeners[i+1:]...)
				break
			}
		}
		p.mu.Unlock()
	}()
	if err := stream.Send(&pluginapi.ListAndWatchResponse{Devices: p.Devices()}); err != nil {
		return err
	}
	for {
		select {
		case resp, ok := <-ch:
			if !ok {
				return nil
			}
			if err := stream.Send(resp); err != nil {
				return err
			}
		case <-stream.Context().Done():
			return stream.Context().Err()
		}
	}
}

// Allocate tags each container with the number of millicpus it asked for. It
// never fails: the admission rules live in the NRI plugin.
func (p *Plugin) Allocate(_ context.Context, req *pluginapi.AllocateRequest) (*pluginapi.AllocateResponse, error) {
	out := &pluginapi.AllocateResponse{}
	for _, cr := range req.ContainerRequests {
		milli := fmt.Sprintf("%d", len(cr.DevicesIDs))
		out.ContainerResponses = append(out.ContainerResponses, &pluginapi.ContainerAllocateResponse{
			Envs:        map[string]string{names.EnvMilli: milli},
			Annotations: map[string]string{names.MilliAnnotation: milli},
		})
	}
	return out, nil
}

// SocketPath is the plugin's socket inside the kubelet's device plugin dir.
func SocketPath() string { return filepath.Join(pluginapi.DevicePluginPath, SocketName) }

// RegisterWithKubelet dials the kubelet's registration socket and registers
// the plugin for the given resource name.
func RegisterWithKubelet(ctx context.Context, resource string) error {
	conn, err := grpc.NewClient("unix://"+pluginapi.KubeletSocket,
		grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return fmt.Errorf("dial kubelet: %w", err)
	}
	defer conn.Close()
	client := pluginapi.NewRegistrationClient(conn)
	_, err = client.Register(ctx, &pluginapi.RegisterRequest{
		Version:      pluginapi.Version,
		Endpoint:     SocketName,
		ResourceName: resource,
		Options:      &pluginapi.DevicePluginOptions{},
	})
	if err != nil {
		return fmt.Errorf("register with kubelet: %w", err)
	}
	return nil
}

// Serve starts the gRPC server on SocketPath and registers with the kubelet.
// The returned stop closes the server and removes the socket.
func (p *Plugin) Serve(ctx context.Context, resource string) (func(), error) {
	if err := os.MkdirAll(pluginapi.DevicePluginPath, 0o755); err != nil {
		return nil, fmt.Errorf("device plugin dir: %w", err)
	}
	socket := SocketPath()
	if err := os.Remove(socket); err != nil && !errors.Is(err, os.ErrNotExist) {
		return nil, fmt.Errorf("remove old socket: %w", err)
	}
	lis, err := net.Listen("unix", socket)
	if err != nil {
		return nil, fmt.Errorf("listen %s: %w", socket, err)
	}
	srv := grpc.NewServer()
	pluginapi.RegisterDevicePluginServer(srv, p)
	healthpb.RegisterHealthServer(srv, health.NewServer())
	go func() {
		if err := srv.Serve(lis); err != nil && !errors.Is(err, grpc.ErrServerStopped) {
			p.log.Error("device plugin serve", "error", err.Error())
		}
	}()
	if err := RegisterWithKubelet(ctx, resource); err != nil {
		srv.Stop()
		return nil, err
	}
	p.log.Info("device plugin registered", "resource", resource, "capacity", p.capacity, "socket", socket)
	p.mu.Lock()
	f := p.onRegistered
	p.mu.Unlock()
	if f != nil {
		f()
	}
	return func() {
		srv.Stop()
		_ = os.Remove(socket)
	}, nil
}

// WatchSocket re-serves and re-registers when the kubelet deleted our socket
// (it does that on every kubelet restart). It returns when ctx is done.
func (p *Plugin) WatchSocket(ctx context.Context, resource string) {
	var stop func()
	defer func() {
		if stop != nil {
			stop()
		}
	}()
	for {
		select {
		case <-ctx.Done():
			return
		case <-time.After(ResocketPeriod):
		}
		if _, err := os.Stat(SocketPath()); err == nil {
			continue
		}
		p.log.Warn("device plugin socket is gone (kubelet restart?); serving again")
		// Drop the old server first: its socket file is gone, and leaving it
		// running leaks a goroutine and a listener per kubelet restart.
		if stop != nil {
			stop()
			stop = nil
		}
		s, err := p.Serve(ctx, resource)
		if err != nil {
			p.log.Error("device plugin re-serve", "error", err.Error())
			continue
		}
		stop = s
	}
}
