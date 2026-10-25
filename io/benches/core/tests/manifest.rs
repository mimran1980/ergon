//! The run manifest: what fairness checks compare across arms.

use ergon_latency_core::manifest::{CORE_BUILD_HASH, RunManifest, sha256_hex};
use ergon_latency_core::message::template;

type R = Result<(), Box<dyn std::error::Error>>;

#[test]
fn sha256_is_lowercase_hex_of_the_bytes() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn the_core_build_hash_is_a_sha256() {
    assert_eq!(CORE_BUILD_HASH.len(), 64);
    assert!(
        CORE_BUILD_HASH
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
}

#[test]
fn the_manifest_names_payload_build_arm_and_settings() -> R {
    let payload = template();
    let mut m = RunManifest::new("mio-udp", &payload, 106);
    m.setting("streams", "8")
        .setting("busy_poll", "50")
        .setting("a\"quote", "back\\slash");
    let json = m.to_json();
    let want = format!(
        "{{\n  \"arm\": \"mio-udp\",\n  \"payload_sha256\": \"{}\",\n  \"payload_size\": 64,\n  \"wire_bytes\": 106,\n  \
         \"core_build_hash\": \"{CORE_BUILD_HASH}\",\n  \"settings\": {{\n    \"a\\\"quote\": \"back\\\\slash\",\n    \
         \"busy_poll\": \"50\",\n    \"streams\": \"8\"\n  }}\n}}\n",
        sha256_hex(&payload)
    );
    assert_eq!(json, want);
    let dir = std::env::temp_dir().join(format!(
        "ergon-latency-core-manifest-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("run_manifest.json");
    m.write(&path)?;
    assert_eq!(std::fs::read_to_string(&path)?, want);
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}
