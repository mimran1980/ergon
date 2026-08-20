#![deny(unused_must_use)]

use ergo_aeron_cluster::SessionBuilder;

fn main() {
    SessionBuilder::default()
        .ingress_channel("aeron:ipc")
        .unwrap()
        .egress_channel("aeron:ipc")
        .unwrap()
        .connect_async("/dev/shm/aeron-driver"); // discarded — must_use violation
}
