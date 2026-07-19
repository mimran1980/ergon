#![cfg(feature = "test-harness")]

use ergo_aeron_cluster::test_support::jar;

#[test]
fn test_find_jar_returns_error_for_invalid_prefix() -> Result<(), Box<dyn std::error::Error>> {
    // find_jar panics on unknown prefix — we test this by checking
    // that it panics (not segfaults or hangs)
    let result = std::panic::catch_unwind(|| {
        jar::find_jar("nonexistent-jar-");
    });
    assert!(
        result.is_err(),
        "find_jar should panic on unknown jar prefix"
    );

    Ok(())
}

#[test]
fn test_sha256_is_stable() -> Result<(), Box<dyn std::error::Error>> {
    let path = jar::find_jar("aeron-all-");
    let h1 = jar::sha256(&path);
    let h2 = jar::sha256(&path);
    assert_eq!(h1, h2, "SHA-256 should be deterministic");
    assert_eq!(h1.len(), 64, "SHA-256 hex output should be 64 chars");
    assert!(
        h1.chars().all(|c| c.is_ascii_hexdigit()),
        "SHA-256 should only contain hex digits"
    );

    Ok(())
}
