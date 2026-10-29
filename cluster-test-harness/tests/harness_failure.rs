#![allow(missing_docs)]

use ergo_aeron_cluster_test_harness::jar;

#[test]
fn test_find_jar_returns_error_for_invalid_prefix() {
    assert!(
        jar::try_find_jar("nonexistent-jar-").is_err(),
        "try_find_jar should return Err on unknown jar prefix"
    );
}
