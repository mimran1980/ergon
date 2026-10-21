// SPDX-License-Identifier: Apache-2.0

package config

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	"github.com/mimran1980/ergon/k8s/nri-shared-cpus/internal/latency"
)

// fakeNuma builds a sysfs node directory with the given node<N> directories
// and the files the real one carries beside them.
func fakeNuma(t *testing.T, nodes ...int) string {
	t.Helper()
	root := t.TempDir()
	dir := filepath.Join(root, "devices/system/node")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"online", "possible", "has_cpu", "has_memory"} {
		if err := os.WriteFile(filepath.Join(dir, name), []byte("0-1\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	for _, n := range nodes {
		if err := os.Mkdir(filepath.Join(dir, "node"+strconv.Itoa(n)), 0o755); err != nil {
			t.Fatal(err)
		}
	}
	return root
}

func TestDiscoverHostNuma(t *testing.T) {
	c := &Config{}
	if err := DiscoverHostNuma(c, fakeNuma(t, 0, 1)); err != nil {
		t.Fatal(err)
	}
	if len(c.NodeNumaNodes) != 2 || c.NodeNumaNodes[0] != 0 || c.NodeNumaNodes[1] != 1 {
		t.Errorf("NodeNumaNodes = %v, want [0 1]", c.NodeNumaNodes)
	}
}

func TestDiscoverHostNumaSingle(t *testing.T) {
	c := &Config{}
	if err := DiscoverHostNuma(c, fakeNuma(t, 0)); err != nil {
		t.Fatal(err)
	}
	if len(c.NodeNumaNodes) != 1 || c.NodeNumaNodes[0] != 0 {
		t.Errorf("NodeNumaNodes = %v, want [0]", c.NodeNumaNodes)
	}
}

// A kernel built without CONFIG_NUMA has no node directory at all. The
// machine still has one node, and cpuset.mems names it 0, so that is what
// the list says. This is the only case where the list is not read from
// node<N> directories, and it is still a real answer to bind to.
func TestDiscoverHostNumaWithoutANodeDirectory(t *testing.T) {
	c := &Config{}
	if err := DiscoverHostNuma(c, t.TempDir()); err != nil {
		t.Fatal(err)
	}
	if len(c.NodeNumaNodes) != 1 || c.NodeNumaNodes[0] != 0 {
		t.Errorf("NodeNumaNodes = %v, want [0]", c.NodeNumaNodes)
	}
}

// A node directory with no node<N> entry cannot be answered from: the
// plugin must not fall back to "no NUMA nodes" and drop a binding request.
func TestDiscoverHostNumaRefusesAnEmptyDirectory(t *testing.T) {
	c := &Config{}
	err := DiscoverHostNuma(c, fakeNuma(t))
	if err == nil {
		t.Fatal("want a failure when the node directory names no node")
	}
	if len(c.NodeNumaNodes) != 0 {
		t.Errorf("NodeNumaNodes = %v, want nothing recorded", c.NodeNumaNodes)
	}
}

func TestDiscoverHostNumaIgnoresOtherNames(t *testing.T) {
	root := fakeNuma(t, 0, 1)
	dir := filepath.Join(root, "devices/system/node")
	for _, name := range []string{"node", "nodes", "node-1", "cpu0"} {
		if err := os.Mkdir(filepath.Join(dir, name), 0o755); err != nil {
			t.Fatal(err)
		}
	}
	c := &Config{}
	if err := DiscoverHostNuma(c, root); err != nil {
		t.Fatal(err)
	}
	if len(c.NodeNumaNodes) != 2 || c.NodeNumaNodes[0] != 0 || c.NodeNumaNodes[1] != 1 {
		t.Errorf("NodeNumaNodes = %v, want [0 1]", c.NodeNumaNodes)
	}
}

// Nothing in this tree discovers the data NIC's NUMA node, so Parse must
// leave NicNumaNode at -1 ("unknown"). Zero would name node 0, which is a
// real NUMA node on a multi-node machine.
func TestParseNicNumaNodeUnknown(t *testing.T) {
	c, err := Parse([]byte("sharedCpus: \"0,1\"\n"))
	if err != nil {
		t.Fatal(err)
	}
	if c.NicNumaNode != -1 {
		t.Errorf("NicNumaNode = %d, want -1 (unknown until something records it)", c.NicNumaNode)
	}
	// That default is what PickMems sees: on a multi-node machine the
	// NIC's-node ask must be refused, never answered as node 0.
	mems, nodes, err := latency.PickMems("", []int{0, 1}, c.NicNumaNode)
	if err == nil {
		t.Fatalf("PickMems(%q, [0 1], %d) = %q, %q: want a refusal, not node 0", "", c.NicNumaNode, mems, nodes)
	}
	if !strings.Contains(err.Error(), "none recorded") {
		t.Errorf("error %q must name that no NIC node is recorded", err)
	}
}

// With no NIC node recorded, "" ("the NIC's node") is refused on a
// multi-node machine: node 0 would be a guess.
func TestNicNumaNodeUnknownRefusesOnMultiNode(t *testing.T) {
	mems, nodes, err := latency.PickMems("", []int{0, 1}, -1)
	if err == nil {
		t.Fatalf("PickMems(%q, [0 1], -1) = %q, %q: want a refusal, not node 0", "", mems, nodes)
	}
	if !strings.Contains(err.Error(), "none recorded") {
		t.Errorf("error %q must name that no NIC node is recorded", err)
	}
}

// A one-node machine has one answer, so an unknown NIC node still resolves.
func TestNicNumaNodeUnknownStillAnswersOnOneNode(t *testing.T) {
	mems, nodes, err := latency.PickMems("", []int{0}, -1)
	if err != nil {
		t.Fatal(err)
	}
	if mems != "0" || nodes != "0" {
		t.Errorf("got mems=%q nodes=%q, want 0", mems, nodes)
	}
}

// A recorded NIC node answers its own node.
func TestNicNumaNodeKnownAnswersItsNode(t *testing.T) {
	mems, nodes, err := latency.PickMems("", []int{0, 1}, 1)
	if err != nil {
		t.Fatal(err)
	}
	if mems != "1" || nodes != "1" {
		t.Errorf("got mems=%q nodes=%q, want the NIC node 1", mems, nodes)
	}
}
