// SPDX-License-Identifier: Apache-2.0

package names

import "testing"

func TestRequiredPluginListed(t *testing.T) {
	yes := []string{
		`["ergon-shared-cpus"]`,
		`["ergon-shared-cpus","other"]`,
		`["other", "ergon-shared-cpus"]`,
		`[ "ergon-shared-cpus" ]`,
	}
	no := []string{
		``,
		`ergon-shared-cpus`,
		`["not-ergon-shared-cpus"]`,
		`["ergon-shared-cpus-extra"]`,
		`["ergon-shared-cpus-extra","other"]`,
		`not-ergon-shared-cpus`,
	}
	for _, s := range yes {
		if !RequiredPluginListed(s) {
			t.Errorf("RequiredPluginListed(%q) = false, want true", s)
		}
	}
	for _, s := range no {
		if RequiredPluginListed(s) {
			t.Errorf("RequiredPluginListed(%q) = true, want false", s)
		}
	}
}
