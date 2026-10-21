// SPDX-License-Identifier: Apache-2.0

package config

import (
	"os"
	"path/filepath"
	"testing"
)

func TestParseDevRoot(t *testing.T) {
	c, err := Parse([]byte("sharedCpus: \"0\"\n"))
	if err != nil {
		t.Fatal(err)
	}
	if c.DevRoot != "/dev" {
		t.Errorf("default DevRoot = %q, want /dev", c.DevRoot)
	}
	c, err = Parse([]byte("sharedCpus: \"0\"\ndevRoot: /host/dev\n"))
	if err != nil {
		t.Fatal(err)
	}
	if c.DevRoot != "/host/dev" {
		t.Errorf("DevRoot = %q, want /host/dev", c.DevRoot)
	}
	for _, bad := range []string{"devRoot: dev\n", "devRoot: /host/dev/../etc\n", "devRoot: /host/dev/\n"} {
		if _, err := Parse([]byte("sharedCpus: \"0\"\n" + bad)); err == nil {
			t.Errorf("Parse(%q) accepted a dirty or relative devRoot", bad)
		}
	}
}

func TestHostDevicePath(t *testing.T) {
	for _, tc := range []struct{ root, in, want string }{
		{"/dev", "/dev/ptp0", "/dev/ptp0"},
		{"", "/dev/ptp0", "/dev/ptp0"},
		{"/host/dev", "/dev/ptp0", "/host/dev/ptp0"},
		{"/host/dev", "/dev/cpu_dma_latency", "/host/dev/cpu_dma_latency"},
	} {
		if got := HostDevicePath(tc.root, tc.in); got != tc.want {
			t.Errorf("HostDevicePath(%q, %q) = %q, want %q", tc.root, tc.in, got, tc.want)
		}
	}
}

func TestDiscoverHostDevices(t *testing.T) {
	dir := t.TempDir()
	for _, name := range []string{"ptp0", "ptp1", "ptp_ena", "cpu_dma_latency", "null", "tty"} {
		if err := os.WriteFile(filepath.Join(dir, name), nil, 0o600); err != nil {
			t.Fatal(err)
		}
	}
	c := &Config{DevRoot: dir}
	if err := DiscoverHostDevices(c, "amd64"); err != nil {
		t.Fatal(err)
	}
	if c.GOARCH != "amd64" {
		t.Errorf("GOARCH = %q", c.GOARCH)
	}
	want := "ptp0,ptp1,ptp_ena"
	got := ""
	for i, p := range c.HostPtp {
		if i > 0 {
			got += ","
		}
		got += filepath.Base(p)
	}
	if got != want {
		t.Errorf("HostPtp = %q, want %q (sorted, /dev/ names)", got, want)
	}
	for _, p := range c.HostPtp {
		if filepath.Dir(p) != "/dev" {
			t.Errorf("HostPtp entry %q must be a /dev/ path (the granted name), not under %s", p, dir)
		}
	}
	if c.HostCpuDmaLatency != "/dev/cpu_dma_latency" {
		t.Errorf("HostCpuDmaLatency = %q, want /dev/cpu_dma_latency", c.HostCpuDmaLatency)
	}
}

func TestDiscoverHostDevicesNoPtpNoQos(t *testing.T) {
	dir := t.TempDir()
	c := &Config{DevRoot: dir}
	if err := DiscoverHostDevices(c, "arm64"); err != nil {
		t.Fatal(err)
	}
	if len(c.HostPtp) != 0 || c.HostCpuDmaLatency != "" || c.GOARCH != "arm64" {
		t.Errorf("empty dev root: %+v", c)
	}
}

func TestDiscoverHostDevicesMissingRoot(t *testing.T) {
	c := &Config{DevRoot: filepath.Join(t.TempDir(), "absent")}
	if err := DiscoverHostDevices(c, "amd64"); err == nil {
		t.Error("a missing dev root must fail closed: the plugin cannot grant devices it cannot see")
	}
}
