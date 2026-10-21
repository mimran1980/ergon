// SPDX-License-Identifier: Apache-2.0

package deviceplugin

import (
	"context"
	"encoding/binary"
	"log/slog"
	"testing"

	pluginapi "k8s.io/kubelet/pkg/apis/deviceplugin/v1beta1"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/names"
)

func newTestPlugin(t *testing.T, capacity int) *Plugin {
	t.Helper()
	return New(capacity, slog.Default())
}

func TestIDsAreStableAndCountRight(t *testing.T) {
	p := newTestPlugin(t, 2500)
	ids := p.IDs()
	if len(ids) != 2500 {
		t.Fatalf("got %d IDs, want 2500", len(ids))
	}
	if ids[0] != "m000000" || ids[1] != "m000001" || ids[2499] != "m002499" {
		t.Fatalf("IDs are not stable: %q %q %q", ids[0], ids[1], ids[2499])
	}
	again := p.IDs()
	for i := range ids {
		if ids[i] != again[i] {
			t.Fatalf("ID %d changed between calls: %q vs %q", i, ids[i], again[i])
		}
	}
}

func TestDevicesHealth(t *testing.T) {
	p := newTestPlugin(t, 3)
	devs := p.Devices()
	if len(devs) != 3 {
		t.Fatalf("got %d devices, want 3", len(devs))
	}
	for _, d := range devs {
		if d.Health != pluginapi.Healthy {
			t.Fatalf("device %s is %q, want Healthy", d.ID, d.Health)
		}
	}
	p.SetHealthy(false)
	for _, d := range p.Devices() {
		if d.Health != pluginapi.Unhealthy {
			t.Fatalf("device %s is %q after SetHealthy(false), want Unhealthy", d.ID, d.Health)
		}
	}
	p.SetHealthy(true)
	for _, d := range p.Devices() {
		if d.Health != pluginapi.Healthy {
			t.Fatalf("device %s is %q after SetHealthy(true), want Healthy", d.ID, d.Health)
		}
	}
}

func TestAllocateSetsEnvAndAnnotation(t *testing.T) {
	p := newTestPlugin(t, 4000)
	resp, err := p.Allocate(context.Background(), &pluginapi.AllocateRequest{
		ContainerRequests: []*pluginapi.ContainerAllocateRequest{
			{DevicesIDs: p.IDs()[:250]},
			{DevicesIDs: p.IDs()[250:1500]},
		},
	})
	if err != nil {
		t.Fatal(err)
	}
	if len(resp.ContainerResponses) != 2 {
		t.Fatalf("got %d container responses, want 2", len(resp.ContainerResponses))
	}
	checks := []string{"250", "1250"}
	for i, want := range checks {
		got := resp.ContainerResponses[i]
		if got.Envs[names.EnvMilli] != want {
			t.Errorf("container %d: env %s=%q, want %q", i, names.EnvMilli, got.Envs[names.EnvMilli], want)
		}
		if got.Annotations[names.MilliAnnotation] != want {
			t.Errorf("container %d: annotation %s=%q, want %q", i, names.MilliAnnotation, got.Annotations[names.MilliAnnotation], want)
		}
	}
}

func TestAllocateEmptyDevicesIsZero(t *testing.T) {
	p := newTestPlugin(t, 10)
	resp, err := p.Allocate(context.Background(), &pluginapi.AllocateRequest{
		ContainerRequests: []*pluginapi.ContainerAllocateRequest{{DevicesIDs: nil}},
	})
	if err != nil {
		t.Fatal(err)
	}
	if got := resp.ContainerResponses[0].Envs[names.EnvMilli]; got != "0" {
		t.Fatalf("empty allocation env is %q, want \"0\"", got)
	}
}

func TestGetDevicePluginOptions(t *testing.T) {
	p := newTestPlugin(t, 1)
	opts, err := p.GetDevicePluginOptions(context.Background(), &pluginapi.Empty{})
	if err != nil {
		t.Fatal(err)
	}
	if opts.PreStartRequired {
		t.Error("PreStartRequired is true; we do not implement it")
	}
	if opts.GetPreferredAllocationAvailable {
		t.Error("GetPreferredAllocationAvailable is true; we do not implement it")
	}
}

func TestListAndWatchMessageSizeAt100k(t *testing.T) {
	p := newTestPlugin(t, 100000)
	resp := &pluginapi.ListAndWatchResponse{Devices: p.Devices()}
	// The generated gogo types expose XXX_Size without a proto.Message
	// interface in this module's build tags.
	payload := resp.XXX_Size()
	// The kubelet dials us with grpc-go defaults, so one reply must stay under
	// 4 MiB. Frame it the way gRPC would: 5-byte prefix plus the payload.
	frame := 5 + payload
	if frame > MaxDeviceMessage {
		t.Fatalf("ListAndWatch reply is %d bytes framed, the kubelet's limit is %d", frame, MaxDeviceMessage)
	}
	var size [4]byte
	binary.BigEndian.PutUint32(size[:], uint32(payload))
	t.Logf("100000 devices: %d bytes of payload, %d framed (limit %d)", payload, frame, MaxDeviceMessage)
}

func TestSetHealthyNotifiesWatchers(t *testing.T) {
	p := newTestPlugin(t, 2)
	ch := make(chan *pluginapi.ListAndWatchResponse, 4)
	p.mu.Lock()
	p.listeners = append(p.listeners, ch)
	p.mu.Unlock()
	p.SetHealthy(false)
	select {
	case resp := <-ch:
		if resp.Devices[0].Health != pluginapi.Unhealthy {
			t.Fatalf("watcher got health %q, want Unhealthy", resp.Devices[0].Health)
		}
	default:
		t.Fatal("SetHealthy(false) did not notify the watcher")
	}
	// No change: no notification.
	p.SetHealthy(false)
	select {
	case <-ch:
		t.Fatal("SetHealthy with no change notified the watcher")
	default:
	}
}
