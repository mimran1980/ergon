use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};

use crate::jar;

static NEXT_BASE_PORT: AtomicU16 = AtomicU16::new(9000);

pub struct TestCluster {
    processes: Vec<Child>,
    pub ingress_channel: String,
    pub egress_channel: String,
    aeron_dir: PathBuf,
}

fn classpath() -> String {
    let aeron_all = jar::find_jar("aeron-all-");
    let aeron_cluster = jar::find_jar("aeron-cluster-");
    let aeron_samples = jar::find_jar("aeron-samples-");
    let samples_classes = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("aeron")
        .join("aeron-samples")
        .join("build")
        .join("classes")
        .join("java")
        .join("main");
    format!(
        "{}:{}:{}:{}",
        aeron_all.display(),
        aeron_cluster.display(),
        aeron_samples.display(),
        samples_classes.display()
    )
}

fn launch_node(base_port: u16, member_id: u16, node_count: u16) -> Child {
    Command::new("java")
        .args([
            "--add-opens",
            "java.base/jdk.internal.misc=ALL-UNNAMED",
            "-cp",
            &classpath(),
            "ClusterLauncher",
            &base_port.to_string(),
            &member_id.to_string(),
            &node_count.to_string(),
        ])
        .stdout(Stdio::piped())
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
        panic!("ClusterLauncher did not emit CLUSTER_READY");
    }
    (ingress, egress, aeron_dir)
}

impl TestCluster {
    pub fn single_node() -> Self {
        let base_port = NEXT_BASE_PORT.fetch_add(100, Ordering::SeqCst);
        let mut child = launch_node(base_port, 0, 1);
        let (ingress, egress, aeron_dir) = read_ready(&mut child);
        Self {
            processes: vec![child],
            ingress_channel: ingress,
            egress_channel: egress,
            aeron_dir: PathBuf::from(aeron_dir),
        }
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
        }
    }

    pub fn aeron_dir(&self) -> &std::path::Path {
        &self.aeron_dir
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
