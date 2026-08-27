#![deny(unused_must_use)]

use ergo_aeron_cluster::StaticCredentials;

fn main() {
    StaticCredentials::new(vec![1u8, 2, 3]);
    StaticCredentials::from_utf8("user:pass");
}
