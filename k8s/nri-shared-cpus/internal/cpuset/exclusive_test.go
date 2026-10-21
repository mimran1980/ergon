// SPDX-License-Identifier: Apache-2.0

package cpuset

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestMissingExclusiveNamesTheFeature: a kernel without cpuset.cpus.exclusive
// answers ENOENT, which says nothing about why the partition cannot be made.
// The rewrite has to name the file and what provides it.
func TestMissingExclusiveNamesTheFeature(t *testing.T) {
	raw := &fs.PathError{Op: "open", Path: "/c/ergon-isolated/cpuset.cpus.exclusive", Err: fs.ErrNotExist}
	err := MissingExclusive("/c/ergon-isolated/cpuset.cpus.exclusive", fmt.Errorf("hook: write %s: %w", raw.Path, raw))
	if err == nil {
		t.Fatal("MissingExclusive returned nil for a missing exclusive file")
	}
	for _, want := range []string{"cpuset.cpus.exclusive", "6.7", "remote partition"} {
		if !strings.Contains(err.Error(), want) {
			t.Errorf("error %q does not mention %q", err, want)
		}
	}
	if !errors.Is(err, fs.ErrNotExist) {
		t.Error("the rewritten error must still wrap fs.ErrNotExist, so callers can tell a missing file from a refused one")
	}
}

// TestMissingExclusiveLeavesOtherErrorsAlone: only a missing exclusive file is
// the missing feature. A refused write, or a missing cpuset.cpus, is its own
// problem and must not be reported as "this kernel has no cpuset.cpus.exclusive".
func TestMissingExclusiveLeavesOtherErrorsAlone(t *testing.T) {
	perm := &fs.PathError{Op: "open", Path: "/c/cpuset.cpus.exclusive", Err: fs.ErrPermission}
	if got := MissingExclusive(perm.Path, perm); got != perm {
		t.Errorf("a refused write became %v, want it unchanged", got)
	}
	other := &fs.PathError{Op: "open", Path: "/c/cpuset.cpus", Err: fs.ErrNotExist}
	if got := MissingExclusive(other.Path, other); got != other {
		t.Errorf("a missing cpuset.cpus became %v, want it unchanged", got)
	}
	if got := MissingExclusive(perm.Path, nil); got != nil {
		t.Errorf("a nil error became %v, want nil", got)
	}
}

// touch builds the two files that mark a cgroup as cpuset-enabled, and the
// exclusive file only when the kernel is meant to have it.
func touch(t *testing.T, dir string, exclusive bool) {
	t.Helper()
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"cpuset.cpus", "cgroup.subtree_control"} {
		if err := os.WriteFile(filepath.Join(dir, name), []byte("\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	if exclusive {
		if err := os.WriteFile(filepath.Join(dir, ExclusiveFile), []byte("\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
}

// TestHasExclusiveReadsTheFilesystemNotTheVersion: a vendor kernel that
// backported cpuset.cpus.exclusive provides the same file a 6.7 one does, so
// that is all this may look at.
func TestHasExclusiveReadsTheFilesystemNotTheVersion(t *testing.T) {
	root := t.TempDir()
	touch(t, filepath.Join(root, "pod"), true)

	present, probed := HasExclusive(root, filepath.Join(root, "pod", "ctr"))
	if !present || !probed {
		t.Errorf("HasExclusive = (%v, %v), want (true, true) when an ancestor has the file", present, probed)
	}
}

// TestHasExclusiveReportsAnOlderKernel: a cpuset-enabled cgroup with no
// exclusive file is a kernel that cannot make a remote partition. That is a
// definite answer, so probed must be true and present false.
func TestHasExclusiveReportsAnOlderKernel(t *testing.T) {
	root := t.TempDir()
	touch(t, filepath.Join(root, "pod"), false)

	present, probed := HasExclusive(root, filepath.Join(root, "pod", "ctr"))
	if present || !probed {
		t.Errorf("HasExclusive = (%v, %v), want (false, true) when the file is absent", present, probed)
	}
}

// TestHasExclusiveLooksAtRootsChildren: the host check has no container to
// start from. Its probe is whatever under the cgroup root already has cpuset.
func TestHasExclusiveLooksAtRootsChildren(t *testing.T) {
	root := t.TempDir()
	touch(t, filepath.Join(root, "kubepods.slice"), true)

	present, probed := HasExclusive(root, root)
	if !present || !probed {
		t.Errorf("HasExclusive = (%v, %v), want (true, true) from a child of the root", present, probed)
	}
}

// TestHasExclusiveCannotTellWithoutACpusetCgroup: nothing has the cpuset
// controller enabled yet, so there is no file to look at. The caller must not
// treat that as "the feature is missing": the write itself reports that.
func TestHasExclusiveCannotTellWithoutACpusetCgroup(t *testing.T) {
	root := t.TempDir()
	if err := os.MkdirAll(filepath.Join(root, "pod"), 0o755); err != nil {
		t.Fatal(err)
	}

	present, probed := HasExclusive(root, filepath.Join(root, "pod", "ctr"))
	if present || probed {
		t.Errorf("HasExclusive = (%v, %v), want (false, false) with no cpuset-enabled cgroup", present, probed)
	}
}
