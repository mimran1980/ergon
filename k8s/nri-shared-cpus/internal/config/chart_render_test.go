// SPDX-License-Identifier: Apache-2.0

package config

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"sigs.k8s.io/yaml"
)

func chartRoot(t *testing.T) string {
	t.Helper()
	root := filepath.Join("..", "..", "charts", "ergon-shared-cpus")
	if _, err := os.Stat(filepath.Join(root, "Chart.yaml")); err != nil {
		t.Fatalf("chart: %v", err)
	}
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatal(err)
	}
	return abs
}

func helmBin(t *testing.T) string {
	t.Helper()
	bin, err := exec.LookPath("helm")
	if err != nil {
		t.Fatal("helm is required to render the chart")
	}
	return bin
}

func helmOut(t *testing.T, args ...string) string {
	t.Helper()
	cmd := exec.Command(helmBin(t), args...)
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		t.Fatalf("helm %s\n%s\n%s", strings.Join(args, " "), stderr.String(), stdout.String())
	}
	if stderr.Len() > 0 {
		t.Log(stderr.String())
	}
	return stdout.String()
}

func splitDocs(rendered string) [][]byte {
	parts := strings.Split(rendered, "\n---\n")
	var docs [][]byte
	for _, p := range parts {
		p = strings.TrimSpace(p)
		if p == "" || strings.HasPrefix(p, "# Source:") && !strings.Contains(p, "\nkind:") && !strings.Contains(p, "\napiVersion:") {
			// Keep documents that helm prefixes with a source comment.
		}
		if p == "" {
			continue
		}
		docs = append(docs, []byte(p))
	}
	return docs
}

type chartObj struct {
	APIVersion string `json:"apiVersion"`
	Kind       string `json:"kind"`
	Metadata   struct {
		Name        string            `json:"name"`
		Annotations map[string]string `json:"annotations"`
	} `json:"metadata"`
	Data map[string]string `json:"data"`
}

func objects(t *testing.T, rendered string) []chartObj {
	t.Helper()
	var out []chartObj
	for _, doc := range splitDocs(rendered) {
		var o chartObj
		if err := yaml.Unmarshal(doc, &o); err != nil {
			t.Fatalf("unmarshal rendered doc: %v\n%s", err, doc)
		}
		if o.Kind == "" {
			continue
		}
		out = append(out, o)
	}
	return out
}

func configYAML(t *testing.T, rendered string) string {
	t.Helper()
	for _, o := range objects(t, rendered) {
		if o.Kind == "ConfigMap" && o.Data["config.yaml"] != "" {
			return o.Data["config.yaml"]
		}
	}
	t.Fatal("rendered ConfigMap has no config.yaml")
	return ""
}

func configFrom(t *testing.T, rendered string) (*Config, string) {
	t.Helper()
	raw := configYAML(t, rendered)
	cfg, err := Parse([]byte(raw))
	if err != nil {
		t.Fatalf("parsed by config.Parse: %v\n%s", err, raw)
	}
	return cfg, raw
}

func kinds(objs []chartObj) map[string]int {
	m := map[string]int{}
	for _, o := range objs {
		m[o.Kind]++
	}
	return m
}

func TestChartAdmissionFilesMatchSources(t *testing.T) {
	chart := chartRoot(t)
	for _, name := range []string{"validating-admission-policy.yaml", "priority-class.yaml"} {
		got, err := os.ReadFile(filepath.Join(chart, "files", name))
		if err != nil {
			t.Fatal(err)
		}
		src := filepath.Join("/admission", name)
		if _, err := os.Stat(src); err != nil {
			src = filepath.Join("..", "..", "..", "nri-latency-pods", "admission", name)
		}
		want, err := os.ReadFile(src)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(got, want) {
			t.Fatalf("chart files/%s differs from the named admission source", name)
		}
	}
	t.Log("chart admission files match the two named sources")
}

func TestHelmChartRender(t *testing.T) {
	chart := chartRoot(t)
	lint := helmOut(t, "lint", chart)
	t.Log("helm lint")
	t.Log(lint)

	def := helmOut(t, "template", "rel", chart)
	cfg, raw := configFrom(t, def)
	t.Log("parsed by config.Parse")
	if cfg.SchedFifo != SchedFifoOff || cfg.Ptp != "off" || cfg.SeccompIOUring {
		t.Fatalf("seccomp/ptp/schedFifo must stay off, got fifo=%s ptp=%s seccomp=%v\n%s", cfg.SchedFifo, cfg.Ptp, cfg.SeccompIOUring, raw)
	}
	if !strings.Contains(raw, "ioUring: \"off\"") || !strings.Contains(raw, "schedFifo: \"off\"") || !strings.Contains(raw, "ptp: \"off\"") {
		t.Fatalf("off must be quoted\n%s", raw)
	}
	t.Log("seccomp.ioUring off")
	t.Log("ptp off")
	t.Log("schedFifo off")
	if strings.Contains(raw, "cgroupRoot") {
		t.Fatalf("cgroupRoot is not a config field\n%s", raw)
	}
	t.Log("cgroupRoot is not a config field")
	if cfg.Shared.List() != "4" || cfg.DevRoot != "/host/dev" || cfg.SeccompProfilePath != DefaultSeccompProfilePath {
		t.Fatalf("defaults did not reach the parser: %+v", cfg)
	}
	if !strings.Contains(def, "ghcr.io/mimran1980/ergon-shared-cpus:0.1.3") {
		t.Fatal("default image must be the chart appVersion tag 0.1.3 (the dpdk.ergon.io and Aeron hugepage env are new in that image)")
	}
	if strings.Contains(def, "@sha256:") {
		t.Fatal("default render must not pin a digest")
	}
	t.Log("default image is the appVersion tag 0.1.3")
	objs := objects(t, def)
	if kinds(objs)["ValidatingAdmissionPolicy"] != 0 || kinds(objs)["PriorityClass"] != 0 {
		t.Fatal("default admission.create=false must not render the policy or PriorityClasses")
	}
	var owner bool
	for _, o := range objs {
		if o.Kind == "ConfigMap" && o.Data["owner"] == "external" {
			owner = true
		}
		if strings.Contains(strings.Join([]string{o.Metadata.Annotations["checksum/config"], o.Metadata.Annotations["ergon.io/layout-revision"]}, " "), "checksum") {
			t.Fatal("a ConfigMap checksum must not be rendered")
		}
	}
	if !owner || !strings.Contains(def, "owner: external") {
		t.Fatal("external admission mode must render data.owner: external")
	}
	t.Log("admission owner external")
	if strings.Contains(def, "FileOrCreate") {
		t.Fatal("default render must not FileOrCreate a host path")
	}
	if strings.Contains(def, "ergon.io/layout-revision") {
		t.Fatal("empty layoutRevision must not annotate the pod")
	}
	assertDefaultHostContract(t, def)
	if !strings.Contains(def, "helm.sh/chart: ergon-shared-cpus-0.1.3") {
		t.Fatal("chart version must be 0.1.3 (default image 0.1.3)")
	}
	t.Log("chart 0.1.3")

	// A ConfigMap-only change must not alter the DaemonSet pod template.
	// A values file, not --set: Helm splits a comma in 0,1 into two keys.
	movedFile := filepath.Join(t.TempDir(), "moved.yaml")
	if err := os.WriteFile(movedFile, []byte("config:\n  sharedCpus: \"0,1\"\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	moved := helmOut(t, "template", "rel", chart, "-f", movedFile)
	if daemonPod(t, def) != daemonPod(t, moved) {
		t.Fatal("ConfigMap edit rolled the DaemonSet")
	}
	t.Log("ConfigMap edit does not roll the DaemonSet")

	digest := "sha256:d28fea2f95ebce3e53d33f8770f3ba69be75c76ec7437ea2b92c1329759e07e9"
	fullFile := filepath.Join(t.TempDir(), "full.yaml")
	fullYAML := fmt.Sprintf(`
image:
  digest: %s
  tag: ignored
admission:
  create: true
layoutRevision: layout-2
config:
  sharedCpus: "0,1"
  oversubscription: 1
  rlimits:
    memlock: "inf"
    nofile: "1048576"
    rtPrio: "99"
    nice: "1"
  hugepagesAeronDir: /dev/hugepages/aeron
  seccompProfilePath: /etc/ergon-shared-cpus/ergon-io-uring.json
hostContract:
  kubeletConfig:
    enabled: true
  sys:
    enabled: true
  proc:
    enabled: true
  kubeletOverrides:
    enabled: true
    type: Directory
`, digest)
	if err := os.WriteFile(fullFile, []byte(fullYAML), 0o644); err != nil {
		t.Fatal(err)
	}
	full := helmOut(t, "template", "rel", chart, "-f", fullFile)
	if !strings.Contains(full, "ghcr.io/mimran1980/ergon-shared-cpus@"+digest) {
		t.Fatal("digest render must be repository@digest")
	}
	if strings.Contains(full, "ergon-shared-cpus:ignored") || strings.Contains(full, "ergon-shared-cpus:0.1.3") {
		t.Fatal("digest render must not keep a tag")
	}
	t.Log("repository@digest")
	cfg, raw = configFrom(t, full)
	t.Log("parsed by config.Parse")
	if cfg.Shared.List() != "0,1" || cfg.Rlimits.Memlock != "inf" || cfg.Rlimits.Nofile != "1048576" ||
		cfg.Rlimits.RtPrio != "99" || cfg.Rlimits.Nice != "1" || cfg.HugepagesAeronDir != "/dev/hugepages/aeron" ||
		cfg.SeccompIOUring || cfg.SchedFifo != SchedFifoOff || cfg.Ptp != "off" {
		t.Fatalf("supported values did not reach the parser: %+v\n%s", cfg, raw)
	}
	for _, key := range []string{"sharedCpus", "oversubscription", "namespaces", "defaultAffinity", "hookPath", "annotationOptIn", "metricsAddress", "devRoot", "seccomp", "ioUring", "seccompProfilePath", "schedFifo", "ptp", "hugepagesAeronDir", "rlimits", "memlock", "nofile", "rtPrio", "nice"} {
		if !strings.Contains(raw, key) {
			t.Errorf("rendered config missing %s\n%s", key, raw)
		}
	}
	fobjs := objects(t, full)
	if kinds(fobjs)["ValidatingAdmissionPolicy"] != 1 || kinds(fobjs)["ValidatingAdmissionPolicyBinding"] != 1 || kinds(fobjs)["PriorityClass"] != 2 {
		t.Fatalf("admission create mode kinds = %#v", kinds(fobjs))
	}
	if strings.Contains(full, "owner: external") {
		t.Fatal("admission create mode must not render the external owner marker")
	}
	if !strings.Contains(full, "name: ergon-hot") || !strings.Contains(full, "name: ergon-helper") {
		t.Fatal("admission create mode must render both PriorityClasses")
	}
	if !strings.Contains(full, "startupProbe must not be exec") || !strings.Contains(full, "lifecycle postStart and preStop must not be exec") {
		t.Fatal("rendered policy is not the shipped startup and lifecycle rules")
	}
	t.Log("admission create renders policy and priority classes")
	if !strings.Contains(full, "ergon.io/layout-revision: \"layout-2\"") && !strings.Contains(full, "ergon.io/layout-revision: layout-2") {
		t.Fatal("layoutRevision must be the only pod-template replacement marker")
	}
	if strings.Contains(full, "checksum/config") {
		t.Fatal("layout replacement must not be a ConfigMap checksum")
	}
	if strings.Contains(full, "FileOrCreate") {
		t.Fatal("hostContract mounts must not use FileOrCreate")
	}
	if strings.Contains(full, "type: DirectoryOrCreate") {
		// The hook directory is the pre-existing DirectoryOrCreate. Host
		// contract mounts must not add another one.
		if strings.Count(full, "type: DirectoryOrCreate") != 1 || !strings.Contains(full, "path: /opt/ergon/bin") {
			t.Fatal("hostContract mounts must not use DirectoryOrCreate")
		}
	}
	for _, flag := range []string{"--kubelet-config", "--host-sys", "--host-proc", "--kubelet-overrides"} {
		if !strings.Contains(full, flag) {
			t.Errorf("enabled hostContract did not pass %s", flag)
		}
	}
	if !strings.Contains(full, "type: File") || !strings.Contains(full, "type: Directory") {
		t.Fatal("kubelet config must mount as File and sys as Directory")
	}

	bad := exec.Command(helmBin(t), "template", "rel", chart, "--set", "image.digest=not-a-digest")
	if err := bad.Run(); err == nil {
		t.Fatal("a non-sha256 digest must fail the render")
	}
	schema := exec.Command(helmBin(t), "lint", chart, "--set", "config.cgroupRoot=/sys/fs/cgroup")
	if err := schema.Run(); err == nil {
		t.Fatal("cgroupRoot must fail schema validation")
	}
	t.Log("cgroupRoot is not a config field")
	create := exec.Command(helmBin(t), "template", "rel", chart,
		"--set", "hostContract.kubeletOverrides.enabled=true",
		"--set", "hostContract.kubeletOverrides.type=FileOrCreate")
	if err := create.Run(); err == nil {
		t.Fatal("FileOrCreate must fail the render")
	}
}

func TestHelmProfiles(t *testing.T) {
	chart := chartRoot(t)
	if strings.Contains(helmOut(t, "template", "rel", chart), "kind: ClusterRole") {
		t.Fatal("legacy render must not grant node read")
	}
	if strings.Contains(helmOut(t, "template", "rel", chart), "NODE_NAME") {
		t.Fatal("legacy render must not set NODE_NAME")
	}
	file := filepath.Join(t.TempDir(), "profiles.yaml")
	if err := os.WriteFile(file, []byte(`
config:
  nodeProfileLabel: ergon.io/node-profile
  profiles:
    small:
      sharedCpus: "0"
      namespaces: ["trading"]
    wide:
      sharedCpus: "0,1"
      namespaces: ["trading"]
      rlimits:
        memlock: "inf"
      seccomp:
        ioUring: "off"
      schedFifo: "off"
      ptp: "off"
`), 0o644); err != nil {
		t.Fatal(err)
	}
	rendered := helmOut(t, "template", "rel", chart, "-f", file)
	raw := configYAML(t, rendered)
	if strings.Contains(raw, "\nsharedCpus:") || strings.HasPrefix(strings.TrimSpace(raw), "sharedCpus:") {
		t.Fatalf("profiles render wrote a legacy sharedCpus\n%s", raw)
	}
	wide, err := Resolve([]byte(raw), "wide")
	if err != nil {
		t.Fatalf("parsed by config.Parse via Resolve: %v\n%s", err, raw)
	}
	if wide.Shared.List() != "0,1" || wide.Rlimits.Memlock != "inf" || wide.SeccompIOUring || wide.SchedFifo != SchedFifoOff || wide.Ptp != "off" {
		t.Fatalf("profile wide: %+v", wide)
	}
	small, err := Resolve([]byte(raw), "small")
	if err != nil || small.Shared.List() != "0" {
		t.Fatalf("profile small: %v %+v", err, small)
	}
	if _, err := Parse([]byte(raw)); err == nil {
		t.Fatal("profiles file must not Parse as a legacy config")
	}
	if !strings.Contains(rendered, "kind: ClusterRole") || !strings.Contains(rendered, `resources: ["nodes"]`) || !strings.Contains(rendered, `verbs: ["get"]`) {
		t.Fatal("profiles render must grant nodes get")
	}
	if !strings.Contains(rendered, "name: NODE_NAME") || !strings.Contains(rendered, "automountServiceAccountToken: true") {
		t.Fatal("profiles render must set NODE_NAME and mount the token")
	}
	t.Log("one DaemonSet selects a named profile from a node label")
	missing := exec.Command(helmBin(t), "template", "rel", chart, "--set", "config.nodeProfileLabel=ergon.io/node-profile")
	if err := missing.Run(); err == nil {
		t.Fatal("a label without profiles must fail the render")
	}
}

// assertDefaultHostContract checks that the default render passes the hot-pod
// contract its facts: without --kubelet-config every pod labelled
// ergon.io/latency: hot is refused at create. The kubelet file, /sys and
// /proc are on by default; configz and the overrides directory are not
// (neither exists on a stock kubeadm node, and a File/Directory mount of a
// missing path keeps the pod from starting).
func assertDefaultHostContract(t *testing.T, rendered string) {
	t.Helper()
	ds := daemonPod(t, rendered)
	for _, want := range []string{
		"- --kubelet-config\n            - /host/kubelet-config",
		"- --host-sys\n            - /host/sys",
		"- --host-proc\n            - /host/proc",
		"path: /var/lib/kubelet/config.yaml\n            type: File",
		"path: /sys\n            type: Directory",
		"path: /proc\n            type: Directory",
		"mountPath: /host/kubelet-config\n              readOnly: true",
		"mountPath: /host/sys\n              readOnly: true",
		"mountPath: /host/proc\n              readOnly: true",
	} {
		if !strings.Contains(ds, want) {
			t.Errorf("default render is missing %q\n%s", want, ds)
		}
	}
	for _, flag := range []string{"--kubelet-configz", "--kubelet-overrides", "--kubelet-config-dir", "/host/kubelet-config-dir"} {
		if strings.Contains(ds, flag) {
			t.Errorf("default render must not pass %s", flag)
		}
	}
	if t.Failed() {
		t.FailNow()
	}
	t.Log("default render mounts the kubelet config, /sys and /proc for the hot-pod contract")

	// The other common kubelet path is one --set away, and the switch still
	// turns the mount off.
	chart := chartRoot(t)
	eks := daemonPod(t, helmOut(t, "template", "rel", chart,
		"--set", "hostContract.kubeletConfig.path=/etc/kubernetes/kubelet/config.json"))
	if !strings.Contains(eks, "path: /etc/kubernetes/kubelet/config.json\n            type: File") {
		t.Fatalf("kubeletConfig.path override did not reach the hostPath\n%s", eks)
	}
	// EKS AL2023 (nodeadm): the base file plus its --config-dir, the drop-in
	// directory mounted read-only as a Directory (never DirectoryOrCreate: an
	// empty created directory would hide the node's real drop-ins).
	eksDir := daemonPod(t, helmOut(t, "template", "rel", chart,
		"--set", "hostContract.kubeletConfig.path=/etc/kubernetes/kubelet/config.json",
		"--set", "hostContract.kubeletConfigDir.enabled=true"))
	for _, want := range []string{
		"- --kubelet-config\n            - /host/kubelet-config\n",
		"- --kubelet-config-dir\n            - /host/kubelet-config-dir\n",
		"path: /etc/kubernetes/kubelet/config.json\n            type: File",
		"path: /etc/kubernetes/kubelet/config.json.d\n            type: Directory\n",
		"mountPath: /host/kubelet-config-dir\n              readOnly: true",
	} {
		if !strings.Contains(eksDir, want) {
			t.Errorf("EKS drop-in render is missing %q\n%s", want, eksDir)
		}
	}
	if t.Failed() {
		t.FailNow()
	}
	t.Log("kubeletConfigDir mounts the drop-in directory read-only and passes --kubelet-config-dir")
	for _, bad := range [][]string{
		{"--set", "hostContract.kubeletConfigDir.type=DirectoryOrCreate"},
		{"--set", "hostContract.kubeletConfigDir.enabled=true", "--set", "hostContract.kubeletConfigDir.path="},
		// A drop-in directory without its base file would refuse every hot
		// create; the render fails instead.
		{"--set", "hostContract.kubeletConfigDir.enabled=true", "--set", "hostContract.kubeletConfig.enabled=false"},
	} {
		args := append([]string{"template", "rel", chart}, bad...)
		if err := exec.Command(helmBin(t), args...).Run(); err == nil {
			t.Errorf("schema must refuse %v", bad)
		}
	}
	off := daemonPod(t, helmOut(t, "template", "rel", chart,
		"--set", "hostContract.kubeletConfig.enabled=false"))
	if strings.Contains(off, "--kubelet-config") || strings.Contains(off, "/host/kubelet-config") {
		t.Fatal("kubeletConfig.enabled=false must drop the flag and the mount")
	}
}

// daemonPod returns the DaemonSet document so two renders can be compared
// after a ConfigMap-only edit.
func daemonPod(t *testing.T, rendered string) string {
	t.Helper()
	for _, doc := range splitDocs(rendered) {
		if bytes.Contains(doc, []byte("kind: DaemonSet")) {
			return string(doc)
		}
	}
	t.Fatal("no DaemonSet in render")
	return ""
}
