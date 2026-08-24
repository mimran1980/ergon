#![deny(unused_must_use)]

use ergo_aeron_cluster::SessionBuilder;

fn main() {
    let session = SessionBuilder::default()
        .ingress_channel("aeron:ipc")
        .unwrap()
        .egress_channel("aeron:ipc")
        .unwrap();
    session.connect("/dev/shm/aeron-driver").unwrap().state(); // discarded — must_use violation
}
