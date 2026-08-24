#![deny(unused_must_use)]

use ergo_aeron_cluster::PublicationFailure;

fn main() {
    PublicationFailure::NotConnected.is_retryable(); // discarded — must_use violation
}
