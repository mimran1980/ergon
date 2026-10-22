// SPDX-License-Identifier: Apache-2.0

package config

import (
	"context"
	"errors"
	"fmt"
	"strings"

	"sigs.k8s.io/yaml"
)

// checkMode accepts exactly one configuration mode. Legacy is the flat
// sharedCpus file. Profiles is a map plus nodeProfileLabel and none of the
// flat operator fields.
func checkMode(f File) (bool, error) {
	hasProfiles := len(f.Profiles) > 0
	hasLabel := f.NodeProfileLabel != ""
	if hasProfiles && legacyFieldsSet(f) {
		return false, errors.New("config: sharedCpus and profiles are both set; use one mode")
	}
	if hasProfiles || hasLabel {
		if strings.ContainsAny(f.NodeProfileLabel, " \t\n=") || (hasLabel && strings.Contains(f.NodeProfileLabel, "..")) {
			return false, fmt.Errorf("config: nodeProfileLabel %q is not a label key", f.NodeProfileLabel)
		}
	}
	if hasProfiles && !hasLabel {
		return false, errors.New("config: nodeProfileLabel is required when profiles are set")
	}
	if hasLabel && !hasProfiles {
		return false, errors.New("config: profiles are required when nodeProfileLabel is set")
	}
	return hasProfiles, nil
}

func legacyFieldsSet(f File) bool {
	return f.SharedCPUs != nil || f.Oversubscription != nil || len(f.Namespaces) > 0 ||
		f.DefaultAffinity != "" || f.HookPath != "" || f.AnnotationOptIn ||
		f.MetricsAddress != "" || f.Rlimits != nil || f.Seccomp != nil ||
		f.SeccompProfilePath != "" || f.DevRoot != "" || f.SchedFifo != "" ||
		f.Ptp != "" || f.HugepagesAeronDir != ""
}

// UsesProfiles reports whether data is the named-profile mode.
// An ambiguous file is an error.
func UsesProfiles(data []byte) (bool, error) {
	var f File
	if err := yaml.UnmarshalStrict(data, &f); err != nil {
		return false, fmt.Errorf("config: %w", err)
	}
	return checkMode(f)
}

// ProfileLabelKey is the node label a profiles file selects on.
func ProfileLabelKey(data []byte) (string, error) {
	var f File
	if err := yaml.UnmarshalStrict(data, &f); err != nil {
		return "", fmt.Errorf("config: %w", err)
	}
	if f.NodeProfileLabel == "" {
		return "", errors.New("config: nodeProfileLabel is required when profiles are set")
	}
	return f.NodeProfileLabel, nil
}

// Resolve checks every named profile and returns the one named profileName.
// An unknown name fails. No profile is inferred from an instance size.
func Resolve(data []byte, profileName string) (*Config, error) {
	var f File
	if err := yaml.UnmarshalStrict(data, &f); err != nil {
		return nil, fmt.Errorf("config: %w", err)
	}
	profiles, err := checkMode(f)
	if err != nil {
		return nil, err
	}
	if !profiles {
		return nil, errors.New("config: Resolve needs profiles")
	}
	if profileName == "" {
		return nil, errors.New("config: profile name is empty")
	}
	var selected *Config
	for name, profile := range f.Profiles {
		if !dnsLabel.MatchString(name) {
			return nil, fmt.Errorf("config: profile name %q is not a DNS label", name)
		}
		raw, err := yaml.Marshal(profile.asFile())
		if err != nil {
			return nil, fmt.Errorf("config: profile %s: %w", name, err)
		}
		cfg, err := Parse(raw)
		if err != nil {
			return nil, fmt.Errorf("config: profile %s: %w", name, err)
		}
		if name == profileName {
			selected = cfg
		}
	}
	if selected == nil {
		return nil, fmt.Errorf("config: unknown profile %q", profileName)
	}
	return selected, nil
}

// Select applies legacy parsing, or exactly one profile from labels.
// labels come from one Node GET. The caller does not watch or relabel.
func Select(data []byte, nodeName string, labels map[string]string) (*Config, error) {
	profiles, err := UsesProfiles(data)
	if err != nil {
		return nil, err
	}
	if !profiles {
		return Parse(data)
	}
	if nodeName == "" {
		return nil, errors.New("config: NODE_NAME is empty; a profile cannot be selected")
	}
	key, err := ProfileLabelKey(data)
	if err != nil {
		return nil, err
	}
	name := labels[key]
	if name == "" {
		return nil, fmt.Errorf("config: missing node profile label %s", key)
	}
	return Resolve(data, name)
}

// SelectFromGetter is the startup path: one read-only Node GET, then Select.
func SelectFromGetter(ctx context.Context, data []byte, nodeName string, get func(context.Context, string) (map[string]string, error)) (*Config, error) {
	profiles, err := UsesProfiles(data)
	if err != nil {
		return nil, err
	}
	if !profiles {
		return Parse(data)
	}
	if nodeName == "" {
		return nil, errors.New("config: NODE_NAME is empty; a profile cannot be selected")
	}
	if get == nil {
		return nil, errors.New("config: node API unavailable")
	}
	labels, err := get(ctx, nodeName)
	if err != nil {
		if strings.Contains(err.Error(), "node API unavailable") {
			return nil, err
		}
		return nil, fmt.Errorf("config: node API unavailable: %w", err)
	}
	return Select(data, nodeName, labels)
}
