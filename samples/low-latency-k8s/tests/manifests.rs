//! The Kubernetes manifests, rendered and checked without a cluster:
//!
//! - every overlay through `kubectl kustomize`, the NRI chart through
//!   `helm template` with each of the sample's values files (the chart in
//!   git, and the published 0.1.1 for the kubeadm values);
//! - each render through kubeconform (`ghcr.io/yannh/kubeconform:v0.7.0`, by
//!   digest, in Docker) against Kubernetes 1.33.0 and 1.36.0, strict;
//! - the sample's own rules ([`check_pods`]): Guaranteed whole CPUs, the
//!   shared-millicpus request, the plugin annotation, the priority class,
//!   `resizePolicy`, no exec probes, the Aeron directory mounted, the driver
//!   under `ergon-cpus` with hugepages (vdev-local aside), no conductor or
//!   native resource agent affinity, and `privileged` Pod Security on every
//!   namespace that mounts a hostPath.
//!
//! - per overlay: the image names (`ll:baseline` on vdev, the `REGISTRY/`
//!   placeholder and a pinned tag on EKS, with the tag's `kubernetes.io/arch`
//!   in every workload's node affinity), the report's `LL_NIC`, and a ping
//!   that outlasts `scripts/verify.sh`'s window;
//! - `node/eks-userdata.sh`'s output: housekeeping CPUs 0-1, IRQs on 0, the
//!   sample's `NodeConfig`, under EC2's limit.
//!
//! A missing tool fails the test. The old `md-dpdk/deploy/low-latency`
//! pod (kept as a fixture) must fail the rules.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_yaml::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const KUBECONFORM: &str = "ghcr.io/yannh/kubeconform@sha256:85dbef6b4b312b99133decc9c6fc9495e9fc5f92293d4ff3b7e1b30f5611823c";
const K8S_VERSIONS: [&str; 2] = ["1.33.0", "1.36.0"];
const OVERLAYS: [&str; 4] = [
    "vdev-local",
    "vdev-isolated",
    "eks-ena-arm64",
    "eks-ena-x86",
];

fn here() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(cmd: &mut Command, stdin: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let shown = format!("{cmd:?}");
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{shown}: {e} (is the tool installed?)"))?;
    if let Some(text) = stdin {
        child
            .stdin
            .take()
            .ok_or("no stdin")?
            .write_all(text.as_bytes())?;
    }
    drop(child.stdin.take());
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Err(format!(
            "{shown}: {}\n{}{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(out.stdout)?)
}

fn render(overlay: &str) -> Result<String, Box<dyn std::error::Error>> {
    run(
        Command::new("kubectl")
            .arg("kustomize")
            .arg(here().join("k8s/overlays").join(overlay)),
        None,
    )
}

fn docs(yaml: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    for doc in format!("\n{yaml}").split("\n---") {
        let v: Value = serde_yaml::from_str(doc)?;
        if !v.is_null() {
            out.push(v);
        }
    }
    Ok(out)
}

fn kubeconform(yaml: &str, what: &str) -> TestResult {
    for version in K8S_VERSIONS {
        let summary = run(
            Command::new("docker").args([
                "run",
                "--rm",
                "-i",
                KUBECONFORM,
                "-kubernetes-version",
                version,
                "-strict",
                "-summary",
                "-",
            ]),
            Some(yaml),
        )
        .map_err(|e| format!("kubeconform {what} @ {version}: {e}"))?;
        assert!(
            // without -ignore-missing-schemas a kind with no schema is an error
            summary.contains("Invalid: 0, Errors: 0,"),
            "{what} @ {version}: {summary}"
        );
    }
    Ok(())
}

fn s<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(v, |v, k| v.get(*k))
}

fn text(v: &Value, path: &[&str]) -> String {
    s(v, path)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The sample's rules over one render: the violations, one line each.
#[allow(clippy::too_many_lines)] // the rules in the order the module docs list them
fn check_pods(objects: &[Value], hot: bool, hugepages: bool, ena: bool) -> Vec<String> {
    let mut bad = Vec::new();
    let mut privileged = BTreeMap::new();
    let mut config: BTreeMap<String, String> = BTreeMap::new();
    for o in objects {
        let kind = text(o, &["kind"]);
        if kind == "Namespace" {
            let level = text(
                o,
                &["metadata", "labels", "pod-security.kubernetes.io/enforce"],
            );
            privileged.insert(text(o, &["metadata", "name"]), level == "privileged");
        }
        if kind == "ConfigMap"
            && let Some(data) = s(o, &["data"]).and_then(Value::as_mapping)
        {
            for (k, v) in data {
                config.insert(
                    k.as_str().unwrap_or_default().to_owned(),
                    v.as_str().unwrap_or_default().to_owned(),
                );
            }
        }
    }
    let mut workloads = 0;
    for o in objects {
        let kind = text(o, &["kind"]);
        let name = text(o, &["metadata", "name"]);
        let pod = match kind.as_str() {
            "Pod" => o,
            "DaemonSet" | "Deployment" | "Job" if name.starts_with("ll-") => {
                match s(o, &["spec", "template"]) {
                    Some(t) => t,
                    None => continue,
                }
            }
            _ => continue,
        };
        workloads += 1;
        let at = format!("{kind}/{name}");
        let mut fail = |m: String| bad.push(format!("{at}: {m}"));
        let ns = text(o, &["metadata", "namespace"]);
        if text(
            pod,
            &[
                "metadata",
                "annotations",
                "required-plugins.noderesource.dev",
            ],
        ) != r#"["ergon-shared-cpus"]"# {
            fail("no exact required-plugins annotation".into());
        }
        if text(pod, &["spec", "priorityClassName"]) != "ergon-hot" {
            fail("priorityClassName is not ergon-hot".into());
        }
        let is_hot = text(pod, &["metadata", "labels", "ergon.io/latency"]) == "hot";
        if is_hot != hot {
            fail(format!("ergon.io/latency: hot is {is_hot}, wanted {hot}"));
        }
        let uses_host_path = s(pod, &["spec", "volumes"])
            .and_then(Value::as_sequence)
            .is_some_and(|vs| vs.iter().any(|v| v.get("hostPath").is_some()));
        if uses_host_path && privileged.get(&ns) != Some(&true) {
            fail(format!(
                "mounts a hostPath but namespace {ns:?} is not Pod Security privileged in this render"
            ));
        }
        let containers = s(pod, &["spec", "containers"])
            .and_then(Value::as_sequence)
            .cloned()
            .unwrap_or_default();
        if containers.is_empty() {
            fail("no containers".into());
        }
        for c in &containers {
            let req = s(c, &["resources", "requests"])
                .and_then(Value::as_mapping)
                .cloned()
                .unwrap_or_default();
            let lim = s(c, &["resources", "limits"])
                .and_then(Value::as_mapping)
                .cloned()
                .unwrap_or_default();
            if req != lim || req.is_empty() {
                fail("requests differ from limits: not Guaranteed".into());
            }
            let cpu = req.get("cpu").and_then(Value::as_str).unwrap_or_default();
            if cpu.is_empty() || !cpu.bytes().all(|b| b.is_ascii_digit()) {
                fail(format!("cpu {cpu:?} is not a whole number"));
            }
            if req.get("ergon.io/shared-millicpus").is_none() {
                fail("no ergon.io/shared-millicpus".into());
            }
            let resize_ok = s(c, &["resizePolicy"])
                .and_then(Value::as_sequence)
                .is_some_and(|ps| {
                    ps.iter().any(|p| {
                        text(p, &["resourceName"]) == "cpu"
                            && text(p, &["restartPolicy"]) == "RestartContainer"
                    })
                });
            if !resize_ok {
                fail("no resizePolicy RestartContainer for cpu".into());
            }
            for probe in ["livenessProbe", "readinessProbe", "startupProbe"] {
                if s(c, &[probe, "exec"]).is_some() {
                    fail(format!("{probe} is exec"));
                }
            }
            if s(c, &["lifecycle"]).is_some() {
                fail("a lifecycle handler".into());
            }
            let mounts_aeron = s(c, &["volumeMounts"])
                .and_then(Value::as_sequence)
                .is_some_and(|ms| {
                    ms.iter()
                        .any(|m| text(m, &["mountPath"]) == "/mnt/huge-aeron")
                });
            if !mounts_aeron
                || !config
                    .get("AERON_DIR")
                    .is_some_and(|d| d.starts_with("/mnt/huge-aeron/"))
            {
                fail("the Aeron directory is not on the /mnt/huge-aeron mount".into());
            }
            let env: Vec<String> = s(c, &["env"])
                .and_then(Value::as_sequence)
                .map(|es| es.iter().map(|e| text(e, &["name"])).collect())
                .unwrap_or_default();
            for forbidden in [
                "AERON_CONDUCTOR_CPU_AFFINITY",
                "AERON_DRIVER_NATIVE_RESOURCE_AGENT_CPU_AFFINITY",
            ] {
                if env.iter().any(|e| e == forbidden) || config.contains_key(forbidden) {
                    fail(format!(
                        "{forbidden} is set: the conductor belongs on the shared CPUs"
                    ));
                }
            }
            let has_huge = req.get("hugepages-2Mi").is_some();
            let command: Vec<String> = s(c, &["command"])
                .and_then(Value::as_sequence)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            if kind == "DaemonSet" || kind == "Pod" {
                if command.first().map(String::as_str) != Some("ergon-cpus")
                    || !command
                        .iter()
                        .any(|a| a == "AERON_SENDER_CPU_AFFINITY=isolated:0")
                    || !command
                        .iter()
                        .any(|a| a == "AERON_RECEIVER_CPU_AFFINITY=isolated:1")
                {
                    fail(format!(
                        "the driver does not run under ergon-cpus isolated:0/isolated:1: {command:?}"
                    ));
                }
                if has_huge != hugepages {
                    fail(format!(
                        "driver hugepages-2Mi is {has_huge}, wanted {hugepages}"
                    ));
                }
                if req.get("rusteron.io/dpdk-ena-pair").is_some() != ena {
                    fail(format!("rusteron.io/dpdk-ena-pair wanted {ena}"));
                }
            } else if has_huge {
                fail("an app requests hugepages".into());
            }
        }
    }
    if workloads == 0 {
        bad.push("no workload in the render".into());
    }
    bad
}

#[test]
fn the_old_low_latency_pod_fails_the_rules() -> TestResult {
    let objects = docs(&std::fs::read_to_string(
        here().join("tests/fixtures/md-dpdk-low-latency-pod.yaml"),
    )?)?;
    let bad = check_pods(&objects, true, true, false);
    println!("old pod.yaml: {bad:#?}");
    assert!(bad.iter().any(|b| b.contains("ergon-cpus")), "{bad:?}");
    assert!(bad.iter().any(|b| b.contains("hugepages")), "{bad:?}");
    assert!(bad.iter().any(|b| b.contains("Aeron directory")), "{bad:?}");
    Ok(())
}

#[test]
fn every_overlay_renders_validates_and_follows_the_rules() -> TestResult {
    for overlay in OVERLAYS {
        let yaml = render(overlay)?;
        kubeconform(&yaml, overlay)?;
        let objects = docs(&yaml)?;
        let hot = overlay != "vdev-local";
        let ena = overlay.starts_with("eks");
        let bad = check_pods(&objects, hot, overlay != "vdev-local", ena);
        assert!(bad.is_empty(), "{overlay}: {bad:#?}");
        let has_plugin = objects
            .iter()
            .any(|o| text(o, &["metadata", "name"]) == "rusteron-dpdk-device-plugin");
        assert_eq!(has_plugin, ena, "{overlay}: md-dpdk's device plugin");
        let names: Vec<String> = objects
            .iter()
            .map(|o| text(o, &["metadata", "name"]))
            .collect();
        assert!(
            !names.iter().any(|n| n == "rusteron-media-driver"),
            "{overlay}: md-dpdk's own driver was not removed"
        );
        println!(
            "{overlay}: {} objects, kubeconform {K8S_VERSIONS:?}, rules PASS",
            objects.len()
        );
    }
    Ok(())
}

/// The rendered `ConfigMap` `name`'s data.
fn configmap(objects: &[Value], name: &str) -> BTreeMap<String, String> {
    objects
        .iter()
        .filter(|o| text(o, &["kind"]) == "ConfigMap" && text(o, &["metadata", "name"]) == name)
        .filter_map(|o| s(o, &["data"]).and_then(Value::as_mapping))
        .flat_map(|m| {
            m.iter().map(|(k, v)| {
                (
                    k.as_str().unwrap_or_default().to_owned(),
                    v.as_str().unwrap_or_default().to_owned(),
                )
            })
        })
        .collect()
}

/// verify.sh's default window, s, and the most it waits before a probe
/// starts (self-checks, pod start), s.
const VERIFY_WINDOW_S: u64 = 30;
const VERIFY_START_S: u64 = 60;

#[test]
fn every_overlay_names_its_image_nic_and_a_long_enough_run() -> TestResult {
    for overlay in OVERLAYS {
        let objects = docs(&render(overlay)?)?;
        let eks = overlay.starts_with("eks");
        let app = configmap(&objects, "ll-app");
        // the ping is still measuring for the whole verify window
        let num = |k: &str| -> Result<u64, String> {
            app.get(k)
                .ok_or_else(|| format!("{overlay}: no {k}"))?
                .parse()
                .map_err(|e| format!("{overlay}: {k}: {e}"))
        };
        let (duration, samples) = (num("LL_DURATION_S")?, num("LL_SAMPLES")?);
        assert!(
            duration >= VERIFY_WINDOW_S + VERIFY_START_S,
            "{overlay}: LL_DURATION_S={duration} ends before verify.sh's window"
        );
        assert!(
            samples >= duration * 1_000_000,
            "{overlay}: LL_SAMPLES={samples} can run out before LL_DURATION_S={duration} at 1 us a round trip"
        );
        let nic = app.get("LL_NIC").map_or("", String::as_str);
        assert_eq!(
            nic,
            if eks { "ena" } else { "vdev-tap" },
            "{overlay}: LL_NIC"
        );
        let mut images = 0;
        for o in &objects {
            let kind = text(o, &["kind"]);
            if !matches!(kind.as_str(), "DaemonSet" | "Deployment" | "Job") {
                continue;
            }
            let at = format!("{overlay} {kind}/{}", text(o, &["metadata", "name"]));
            let pod =
                s(o, &["spec", "template", "spec"]).ok_or_else(|| format!("{at}: no pod spec"))?;
            let arches: Vec<String> = s(
                pod,
                &[
                    "affinity",
                    "nodeAffinity",
                    "requiredDuringSchedulingIgnoredDuringExecution",
                    "nodeSelectorTerms",
                ],
            )
            .and_then(Value::as_sequence)
            .into_iter()
            .flatten()
            .filter_map(|t| t.get("matchExpressions").and_then(Value::as_sequence))
            .flatten()
            .filter(|e| text(e, &["key"]) == "kubernetes.io/arch")
            .filter_map(|e| e.get("values").and_then(Value::as_sequence))
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect();
            for c in s(pod, &["containers"])
                .and_then(Value::as_sequence)
                .into_iter()
                .flatten()
            {
                let image = text(c, &["image"]);
                images += 1;
                assert!(
                    !image.ends_with(":latest") && image.contains(':'),
                    "{at}: {image} is not a pinned tag"
                );
                if eks {
                    assert!(
                        image.starts_with("REGISTRY/"),
                        "{at}: {image} is not the REGISTRY placeholder"
                    );
                    let tag = image.rsplit(':').next().unwrap_or_default();
                    let arch = if overlay.ends_with("arm64") {
                        "arm64"
                    } else {
                        "amd64"
                    };
                    assert!(
                        tag.starts_with(arch) || tag.ends_with(arch),
                        "{at}: {image} is not an {arch} tag"
                    );
                    assert_eq!(arches, [arch], "{at}: kubernetes.io/arch affinity");
                } else if image.starts_with("ll") {
                    // the multi-arch tag: no arch selector needed
                    assert_eq!(image, "ll:baseline", "{at}");
                    assert!(arches.is_empty(), "{at}: {arches:?}");
                }
            }
        }
        assert!(images >= 3, "{overlay}: {images} images");
        println!(
            "{overlay}: LL_DURATION_S={duration} LL_SAMPLES={samples} LL_NIC={nic}, {images} images PASS"
        );
    }
    Ok(())
}

#[test]
fn the_eks_user_data_keeps_the_shared_cpu_out_of_isolation() -> TestResult {
    let script = here().join("node/eks-userdata.sh");
    let user_data = |role: &str| {
        run(
            Command::new("bash").arg("-c").arg(format!(
                "set -o pipefail; bash {} demo https://ABC.gr7.eu-west-1.eks.amazonaws.com Zm9vYmFy 10.100.0.0/16 {role} | gzip -dc",
                script.display()
            )),
            None,
        )
    };
    let text = user_data("pong")?;
    for want in [
        "RUSTERON_HOUSEKEEPING_CPUS=0-1\n",
        "RUSTERON_IRQ_CPUS=0\n",
        "reservedSystemCPUs: \"0-1\"",
        "ergon.io/shared-cpus=true",
        "ergon.io/ll-role=pong",
        "io.containerd.nri.v1.nri",
        "name: demo",
        "cidr: 10.100.0.0/16",
    ] {
        assert!(text.contains(want), "no {want:?} in the user data");
    }
    assert_eq!(text.matches("kind: NodeConfig").count(), 1);
    for placeholder in [
        "CLUSTER_NAME",
        "API_SERVER_ENDPOINT",
        "CA_BASE64",
        "SERVICE_CIDR",
        "LL_ROLE",
    ] {
        assert!(
            !text.contains(placeholder),
            "{placeholder} left in the user data"
        );
    }
    // helm/values-eks.yaml's shared CPU is a housekeeping CPU, not an IRQ one
    let values = std::fs::read_to_string(here().join("helm/values-eks.yaml"))?;
    assert!(values.contains(r#"sharedCpus: "1""#));
    assert!(user_data("pang").is_err(), "a bad role was accepted");
    println!("eks-userdata: housekeeping 0-1, IRQs 0, NodeConfig swapped PASS");
    Ok(())
}

#[test]
fn the_probe_pod_template_validates() -> TestResult {
    let template = std::fs::read_to_string(here().join("k8s/verify/probe-pod.yaml"))?;
    let pod = template
        .replace("__NAME__", "ll-probe-n1")
        .replace("__NODE__", "n1")
        .replace("__IMAGE__", "ll:baseline")
        .replace(
            "__COMMAND__",
            r#"["ll-probe","watch","--pod","trading/ll-ping-x=uid"]"#,
        );
    kubeconform(&pod, "probe-pod")?;
    let objects = docs(&pod)?;
    assert_eq!(objects.len(), 1);
    assert_eq!(
        s(&objects[0], &["spec", "hostPID"]).and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(text(&objects[0], &["metadata", "namespace"]), "ll-verify");
    Ok(())
}

#[test]
fn the_nri_chart_renders_with_the_samples_values() -> TestResult {
    let chart = here().join("../../k8s/nri-shared-cpus/charts/ergon-shared-cpus");
    let mut renders = Vec::new();
    for values in ["values-kind", "values-eks"] {
        let file = here().join("helm").join(format!("{values}.yaml"));
        renders.push((
            format!("{values} (chart in git)"),
            template(&chart, &file, &[])?,
        ));
    }
    // the published chart, as the quick start installs it (kubeadm values)
    let oci = Path::new("oci://ghcr.io/mimran1980/charts/ergon-shared-cpus");
    renders.push((
        "values-kind (GHCR 0.1.1)".to_owned(),
        template(
            oci,
            &here().join("helm/values-kind.yaml"),
            &["--version", "0.1.1"],
        )?,
    ));
    for (what, yaml) in &renders {
        kubeconform(yaml, what)?;
        assert!(yaml.contains(r#"sharedCpus: "1""#), "{what}: sharedCpus");
        assert!(
            yaml.contains(r#"namespaces: ["trading", "trading-driver"]"#),
            "{what}: namespaces"
        );
        for flag in ["--kubelet-config", "--host-sys", "--host-proc"] {
            assert!(yaml.contains(flag), "{what}: {flag}");
        }
    }
    let eks = &renders[1].1;
    assert!(
        eks.contains("/etc/kubernetes/kubelet/config.json"),
        "values-eks: kubelet path"
    );
    assert!(
        eks.contains("--kubelet-config-dir"),
        "values-eks: kubeletConfigDir"
    );
    Ok(())
}

fn template(
    chart: &Path,
    values: &Path,
    extra: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    run(
        Command::new("helm")
            .args(["template", "ergon-shared-cpus"])
            .arg(chart)
            .args(["--namespace", "ergon-system", "-f"])
            .arg(values)
            .args(extra),
        None,
    )
}
