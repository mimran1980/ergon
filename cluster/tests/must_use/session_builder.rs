#![deny(unused_must_use)]

use std::time::Duration;
use ergo_aeron_cluster::SessionBuilder;

fn main() {
    SessionBuilder::default().message_timeout(Duration::from_secs(5)); // discarded Result — must_use violation
}
