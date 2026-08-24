#![deny(unused_must_use)]

use ergo_aeron_cluster::SessionBuilder;

fn main() {
    let session = SessionBuilder::default()
        .ingress_channel("aeron:ipc")
        .unwrap()
        .egress_channel("aeron:ipc")
        .unwrap();
    let conn = session.connect_async("/dev/shm/aeron-driver");
    conn.step(); // discarded — must_use violation
}
