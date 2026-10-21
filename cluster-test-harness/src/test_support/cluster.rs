use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};

use super::{jar, jvm};

static NEXT_BASE_PORT: AtomicU16 = AtomicU16::new(9000);

pub struct TestCluster {
    pub processes: Vec<Child>,
    pub ingress_channel: String,
    pub egress_channel: String,
    pub aeron_dir: PathBuf,
    pub base_port: u16,
}

fn classpath() -> std::ffi::OsString {
    let aeron_all = jar::find_jar("aeron-all-");
    let aeron_cluster = jar::find_jar("aeron-cluster-");
    let aeron_samples = jar::find_jar("aeron-samples-");
    let project_classes = PathBuf::from(env!("OUT_DIR")).join("test-harness-java");
    std::env::join_paths([project_classes, aeron_all, aeron_cluster, aeron_samples])
        .expect("Java classpath entries must be valid paths")
}

fn launch_node(base_port: u16, member_id: u16, node_count: u16) -> Child {
    launch_node_impl(base_port, member_id, node_count, false, "echo", None)
}

fn launch_node_keep_dirs(base_port: u16, member_id: u16, node_count: u16) -> Child {
    launch_node_impl(base_port, member_id, node_count, true, "echo", None)
}

/// `jvm_args` replaces [`jvm::test_options`] when given.
fn launch_node_impl(
    base_port: u16,
    member_id: u16,
    node_count: u16,
    keep: bool,
    service: &str,
    jvm_args: Option<&[OsString]>,
) -> Child {
    let mut cmd = match jvm_args {
        None => jvm::command(),
        Some(args) => {
            let mut cmd = jvm::try_java().unwrap_or_else(|e| panic!("{e}"));
            cmd.args(args);
            cmd
        }
    };
    cmd.arg("-cp").arg(classpath()).args([
        "ClusterLauncher",
        &base_port.to_string(),
        &member_id.to_string(),
        &node_count.to_string(),
    ]);
    if keep {
        cmd.arg("keep");
    }
    if service != "echo" {
        cmd.arg(service);
    }
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ClusterLauncher")
}

fn read_ready(child: &mut Child) -> (String, String, String) {
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let mut ingress = String::new();
    let mut egress = String::new();
    let mut aeron_dir = String::new();
    let mut ready = false;
    for line in reader.lines() {
        let line = line.unwrap();
        if line.strip_prefix("CLUSTER_READY").is_some() {
            ready = true;
        } else if let Some(v) = line.strip_prefix("INGRESS=") {
            ingress = v.to_string();
        } else if let Some(v) = line.strip_prefix("EGRESS=") {
            egress = v.to_string();
        } else if let Some(v) = line.strip_prefix("AERON_DIR=") {
            aeron_dir = v.to_string();
        }
        if ready && !ingress.is_empty() && !egress.is_empty() && !aeron_dir.is_empty() {
            break;
        }
    }
    if !ready {
        let mut stderr = String::new();
        if let Some(mut child_stderr) = child.stderr.take() {
            let _ = child_stderr.read_to_string(&mut stderr);
        }
        let status = child.try_wait().ok().flatten();
        panic!(
            "ClusterLauncher did not emit CLUSTER_READY (status: {status:?})\nstderr:\n{stderr}"
        );
    }
    (ingress, egress, aeron_dir)
}

fn single_node_with(service: &str, jvm_args: Option<&[OsString]>) -> TestCluster {
    let base_port = NEXT_BASE_PORT.fetch_add(100, Ordering::SeqCst);
    let mut child = launch_node_impl(base_port, 0, 1, false, service, jvm_args);
    let (ingress, egress, aeron_dir) = read_ready(&mut child);
    TestCluster {
        processes: vec![child],
        ingress_channel: ingress,
        egress_channel: egress,
        aeron_dir: PathBuf::from(aeron_dir),
        base_port,
    }
}

impl TestCluster {
    pub fn single_node() -> Self {
        single_node_with("echo", None)
    }

    /// One echo node whose JVM gets `jvm_args` in place of
    /// [`jvm::test_options`]: system properties (`-D`) configure Aeron.
    pub fn single_node_with_jvm_args(jvm_args: &[OsString]) -> Self {
        single_node_with("echo", Some(jvm_args))
    }

    /// One node running the official `BasicAuctionClusteredService`.
    pub fn single_node_auction() -> Self {
        single_node_with("auction", None)
    }

    /// Launch a 3-node static cluster. Node 0 is the leader.
    pub fn three_node() -> Self {
        let base_port = NEXT_BASE_PORT.fetch_add(300, Ordering::SeqCst);
        let mut processes = Vec::new();
        let mut leader_aeron_dir = PathBuf::new();
        for member_id in 0..3u16 {
            let mut child = launch_node(base_port, member_id, 3);
            let (_ing, _egr, ad) = read_ready(&mut child);
            if member_id == 0 {
                leader_aeron_dir = PathBuf::from(ad);
            }
            processes.push(child);
        }
        let ingress = format!("aeron:udp?endpoint=localhost:{}", base_port + 2);
        let egress = format!("aeron:udp?endpoint=localhost:{}", base_port + 2);
        Self {
            processes,
            ingress_channel: ingress,
            egress_channel: egress,
            aeron_dir: leader_aeron_dir,
            base_port,
        }
    }

    pub fn aeron_dir(&self) -> &std::path::Path {
        &self.aeron_dir
    }
    /// Launch 3 nodes that preserve existing dirs (log-recovery restart).
    /// Uses the same `base_port` as a previous cluster.
    pub fn restart_keep_dirs(&self) -> Self {
        let mut processes = Vec::new();
        for member_id in 0..3u16 {
            let mut child = launch_node_keep_dirs(self.base_port, member_id, 3);
            let _ = read_ready(&mut child);
            processes.push(child);
        }
        let ingress = format!("aeron:udp?endpoint=localhost:{}", self.base_port + 2);
        let egress = format!("aeron:udp?endpoint=localhost:{}", self.base_port + 2);
        Self {
            processes,
            ingress_channel: ingress,
            egress_channel: egress,
            aeron_dir: self.aeron_dir.clone(),
            base_port: self.base_port,
        }
    }

    pub fn base_port(&self) -> u16 {
        self.base_port
    }

    pub fn kill_node(&mut self, index: usize) {
        if index < self.processes.len() {
            let _ = self.processes[index].kill();
            let _ = self.processes[index].wait();
        }
    }
    pub fn node_count(&self) -> usize {
        self.processes.len()
    }
}

impl Drop for TestCluster {
    fn drop(&mut self) {
        for child in &mut self.processes {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
