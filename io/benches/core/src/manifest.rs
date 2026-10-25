//! A run's manifest: what the fairness checks compare across arms. The
//! payload's hash and size, the bytes on the wire per message, the build of
//! this core, the arm and its settings, as JSON (no serde).

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

/// SHA-256 of this crate's sources (`Cargo.toml`, `build.rs`, `src/`), from
/// build.rs: two arms built from different cores differ here.
pub const CORE_BUILD_HASH: &str = env!("CORE_BUILD_HASH");

/// Lowercase hex SHA-256 of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// One run's manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunManifest {
    /// The arm's name.
    pub arm: String,
    /// SHA-256 of the payload's layout ([`template`](crate::message::template)).
    pub payload_sha256: String,
    /// Payload bytes.
    pub payload_size: usize,
    /// Bytes on the wire per message.
    pub wire_bytes: u64,
    /// [`CORE_BUILD_HASH`].
    pub core_build_hash: &'static str,
    /// The arm's settings, by name.
    pub settings: BTreeMap<String, String>,
}

fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl RunManifest {
    /// `arm`'s manifest for `payload`, `wire_bytes` on the wire per message.
    #[must_use]
    pub fn new(arm: &str, payload: &[u8], wire_bytes: u64) -> Self {
        Self {
            arm: arm.to_owned(),
            payload_sha256: sha256_hex(payload),
            payload_size: payload.len(),
            wire_bytes,
            core_build_hash: CORE_BUILD_HASH,
            settings: BTreeMap::new(),
        }
    }

    /// Records a setting (a later one of the same name replaces it).
    pub fn setting(&mut self, name: &str, value: &str) -> &mut Self {
        self.settings.insert(name.to_owned(), value.to_owned());
        self
    }

    /// The manifest as JSON, settings sorted by name.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\n  \"arm\": ");
        json_str(&mut s, &self.arm);
        s.push_str(",\n  \"payload_sha256\": ");
        json_str(&mut s, &self.payload_sha256);
        let _ = write!(
            s,
            ",\n  \"payload_size\": {},\n  \"wire_bytes\": {},\n  \"core_build_hash\": ",
            self.payload_size, self.wire_bytes
        );
        json_str(&mut s, self.core_build_hash);
        s.push_str(",\n  \"settings\": {");
        for (i, (k, v)) in self.settings.iter().enumerate() {
            s.push_str(if i == 0 { "\n    " } else { ",\n    " });
            json_str(&mut s, k);
            s.push_str(": ");
            json_str(&mut s, v);
        }
        s.push_str(if self.settings.is_empty() {
            "}\n}\n"
        } else {
            "\n  }\n}\n"
        });
        s
    }

    /// Writes [`to_json`](Self::to_json) to `path`.
    ///
    /// # Errors
    /// The write's.
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, self.to_json())
    }
}
