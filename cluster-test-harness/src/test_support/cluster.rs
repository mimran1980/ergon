use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};

use ergo_aeron_cluster::ClusterError;

use super::{jar, jvm};

static NEXT_BASE_PORT: AtomicU16 = AtomicU16::new(9000);

/// A Java Aeron Cluster of `ClusterLauncher` nodes, killed on drop.
pub struct TestCluster {
    /// The nodes' JVM processes, by member id.
    pub processes: Vec<Child>,
    /// The ingress channel clients connect to.
    pub ingress_channel: String,
    /// The egress channel the cluster replies on.
    pub egress_channel: String,
    /// The Aeron directory of node 0's media driver (the leader's).
    pub aeron_dir: PathBuf,
    /// The first port of the nodes' range (each cluster takes its own).
    pub base_port: u16,
}

fn classpath() -> Result<OsString, ClusterError> {
    let aeron_all = jar::try_find_jar("aeron-all-")?;
    let aeron_cluster = jar::try_find_jar("aeron-cluster-")?;
    let aeron_samples = jar::try_find_jar("aeron-samples-")?;
    let project_classes = PathBuf::from(env!("OUT_DIR")).join("test-harness-java");
    std::env::join_paths([project_classes, aeron_all, aeron_cluster, aeron_samples]).map_err(|e| {
        ClusterError::connect(format!("Java classpath entries are not valid paths: {e}"))
    })
}

fn launch_node(base_port: u16, member_id: u16, node_count: u16) -> Result<Child, ClusterError> {
    launch_node_impl(base_port, member_id, node_count, false, "echo", None)
}

fn launch_node_keep_dirs(
    base_port: u16,
    member_id: u16,
    node_count: u16,
) -> Result<Child, ClusterError> {
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
) -> Result<Child, ClusterError> {
    let mut cmd = match jvm_args {
        None => jvm::try_command()?,
        Some(args) => {
            let mut cmd = jvm::try_java()?;
            cmd.args(args);
            cmd
        }
    };
    cmd.arg("-cp").arg(classpath()?).args([
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
        .map_err(|e| ClusterError::connect(format!("failed to spawn ClusterLauncher: {e}")))
}

/// A node's ingress, egress and Aeron directory, as it prints them once ready.
fn read_ready(child: &mut Child) -> Result<(String, String, String), ClusterError> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ClusterError::connect("ClusterLauncher's stdout is not piped"))?;
    let reader = BufReader::new(stdout);
    let mut ingress = String::new();
    let mut egress = String::new();
    let mut aeron_dir = String::new();
    let mut ready = false;
    for line in reader.lines() {
        let line = line.map_err(|e| {
            ClusterError::connect(format!("cannot read ClusterLauncher's stdout: {e}"))
        })?;
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
        return Err(ClusterError::connect(format!(
            "ClusterLauncher did not emit CLUSTER_READY (status: {status:?})\nstderr:\n{stderr}"
        )));
    }
    Ok((ingress, egress, aeron_dir))
}

fn single_node_with(
    service: &str,
    jvm_args: Option<&[OsString]>,
) -> Result<TestCluster, ClusterError> {
    let base_port = NEXT_BASE_PORT.fetch_add(100, Ordering::SeqCst);
    let mut child = launch_node_impl(base_port, 0, 1, false, service, jvm_args)?;
    let (ingress, egress, aeron_dir) = read_ready(&mut child)?;
    Ok(TestCluster {
        processes: vec![child],
        ingress_channel: ingress,
        egress_channel: egress,
        aeron_dir: PathBuf::from(aeron_dir),
        base_port,
    })
}

/// Kills every process launched so far when a later launch fails.
fn kill_all(processes: &mut [Child]) {
    for child in processes {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Three nodes on `base_port`, each read up to `CLUSTER_READY`: node 0's
/// Aeron directory and the processes. A failed launch kills those started.
fn launch_three(base_port: u16, keep: bool) -> Result<(Vec<Child>, PathBuf), ClusterError> {
    let mut processes = Vec::new();
    let mut leader_aeron_dir = PathBuf::new();
    for member_id in 0..3u16 {
        let launched = if keep {
            launch_node_keep_dirs(base_port, member_id, 3)
        } else {
            launch_node(base_port, member_id, 3)
        };
        let mut child = match launched {
            Ok(child) => child,
            Err(e) => {
                kill_all(&mut processes);
                return Err(e);
            }
        };
        let ready = read_ready(&mut child);
        processes.push(child);
        match ready {
            Ok((_, _, aeron_dir)) if member_id == 0 => leader_aeron_dir = PathBuf::from(aeron_dir),
            Ok(_) => {}
            Err(e) => {
                kill_all(&mut processes);
                return Err(e);
            }
        }
    }
    Ok((processes, leader_aeron_dir))
}

impl TestCluster {
    /// Launch one node running the echo service.
    ///
    /// # Errors
    ///
    /// [`ClusterError::ConnectFailed`] when a jar is missing, the `java`
    /// fails its check, or the node does not start and report ready.
    pub fn single_node() -> Result<Self, ClusterError> {
        single_node_with("echo", None)
    }

    /// One echo node whose JVM gets `jvm_args` in place of
    /// [`jvm::test_options`]: system properties (`-D`) configure Aeron.
    ///
    /// # Errors
    ///
    /// As [`Self::single_node`].
    pub fn single_node_with_jvm_args(jvm_args: &[OsString]) -> Result<Self, ClusterError> {
        single_node_with("echo", Some(jvm_args))
    }

    /// One node running the official `BasicAuctionClusteredService`.
    ///
    /// # Errors
    ///
    /// As [`Self::single_node`].
    pub fn single_node_auction() -> Result<Self, ClusterError> {
        single_node_with("auction", None)
    }

    /// Launch a 3-node static cluster. Node 0 is the leader.
    ///
    /// # Errors
    ///
    /// As [`Self::single_node`], for any of the three nodes; those already
    /// started are killed.
    pub fn three_node() -> Result<Self, ClusterError> {
        let base_port = NEXT_BASE_PORT.fetch_add(300, Ordering::SeqCst);
        let (processes, leader_aeron_dir) = launch_three(base_port, false)?;
        let ingress = format!("aeron:udp?endpoint=localhost:{}", base_port + 2);
        let egress = format!("aeron:udp?endpoint=localhost:{}", base_port + 2);
        Ok(Self {
            processes,
            ingress_channel: ingress,
            egress_channel: egress,
            aeron_dir: leader_aeron_dir,
            base_port,
        })
    }

    /// The Aeron directory of node 0's media driver.
    #[must_use]
    pub fn aeron_dir(&self) -> &std::path::Path {
        &self.aeron_dir
    }

    /// Launch 3 nodes that preserve existing dirs (log-recovery restart).
    /// Uses the same `base_port` as a previous cluster.
    ///
    /// # Errors
    ///
    /// As [`Self::three_node`].
    pub fn restart_keep_dirs(&self) -> Result<Self, ClusterError> {
        let (processes, _) = launch_three(self.base_port, true)?;
        let ingress = format!("aeron:udp?endpoint=localhost:{}", self.base_port + 2);
        let egress = format!("aeron:udp?endpoint=localhost:{}", self.base_port + 2);
        Ok(Self {
            processes,
            ingress_channel: ingress,
            egress_channel: egress,
            aeron_dir: self.aeron_dir.clone(),
            base_port: self.base_port,
        })
    }

    /// The first port of the cluster's range.
    #[must_use]
    pub const fn base_port(&self) -> u16 {
        self.base_port
    }

    /// Kill node `index` and wait for it (no-op past the last node).
    pub fn kill_node(&mut self, index: usize) {
        if let Some(child) = self.processes.get_mut(index) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// The number of nodes launched.
    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.processes.len()
    }
}

impl Drop for TestCluster {
    fn drop(&mut self) {
        kill_all(&mut self.processes);
    }
}
