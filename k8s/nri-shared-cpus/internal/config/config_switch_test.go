// SPDX-License-Identifier: Apache-2.0

package config

import (
	"strings"
	"testing"
)

// seccomp.ioUring and schedFifo share one parse: the field's own mode
// words, or a YAML 1.1 boolean spelling, quoted or not, any case.
func TestSwitchSpellings(t *testing.T) {
	offWords := []string{"false", "False", "FALSE", "no", "No", "NO", "off", "Off", "OFF", "0",
		`"false"`, `"no"`, `"off"`, `"0"`, `"OFF"`, `""`}
	onWords := []string{"true", "True", "TRUE", "yes", "Yes", "YES", "on", "On", "ON", "1",
		`"true"`, `"yes"`, `"on"`, `"1"`, `"YES"`}
	for _, f := range []struct {
		name  string
		yaml  func(string) string
		on    string
		other string
		isOn  func(*Config) bool
	}{
		{"seccomp.ioUring", func(v string) string { return "seccomp:\n  ioUring: " + v + "\n" }, "allow", "thread",
			func(c *Config) bool { return c.SeccompIOUring }},
		{"schedFifo", func(v string) string { return "schedFifo: " + v + "\n" }, "thread", "allow",
			func(c *Config) bool { return c.SchedFifo == SchedFifoThread }},
	} {
		check := func(v string, want bool) {
			t.Helper()
			c, err := Parse([]byte("sharedCpus: \"0\"\n" + f.yaml(v)))
			if err != nil {
				t.Errorf("%s: %s rejected: %v", f.name, v, err)
				return
			}
			if f.isOn(c) != want {
				t.Errorf("%s: %s gave on=%v, want %v", f.name, v, f.isOn(c), want)
			}
			if c.SchedFifo != SchedFifoOff && c.SchedFifo != SchedFifoThread {
				t.Errorf("%s: %s left schedFifo %q, want off or thread", f.name, v, c.SchedFifo)
			}
		}
		for _, v := range offWords {
			check(v, false)
		}
		for _, v := range append(onWords, f.on, strings.ToUpper(f.on), `"`+f.on+`"`) {
			check(v, true)
		}
		for _, v := range []string{f.other, "2", `"y"`, "enabled", `" off"`} {
			_, err := Parse([]byte("sharedCpus: \"0\"\n" + f.yaml(v)))
			if err == nil {
				t.Errorf("%s: %s accepted", f.name, v)
				continue
			}
			if !strings.Contains(err.Error(), f.name) || !strings.Contains(err.Error(), f.on) || !strings.Contains(err.Error(), "true") {
				t.Errorf("%s: %s: error %q must name the field, its mode and the boolean spellings", f.name, v, err)
			}
		}
	}
}
