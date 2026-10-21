#![allow(missing_docs)]

use ergo_aeron_cluster_test_harness::jar;

#[test]
fn test_find_jar_returns_error_for_invalid_prefix() -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        jar::try_find_jar("nonexistent-jar-").is_err(),
        "try_find_jar should return Err on unknown jar prefix"
    );
    // Legacy panic wrapper still panics for callers that prefer it.
    let result = std::panic::catch_unwind(|| {
        jar::find_jar("nonexistent-jar-");
    });
    assert!(
        result.is_err(),
        "find_jar should panic on unknown jar prefix"
    );

    Ok(())
}
