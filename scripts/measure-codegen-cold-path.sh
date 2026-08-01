#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf "${scratch}"' EXIT
mkdir -p "${scratch}/src"

cat >"${scratch}/Cargo.toml" <<EOF
[package]
name = "ergo-sbe-cold-probe"
version = "0.0.0"
edition = "2024"

[build-dependencies]
ergo-sbe = { path = "${repo_root}/sbe" }

[workspace]
EOF

cat >"${scratch}/build.rs" <<EOF
fn main() -> Result<(), Box<dyn std::error::Error>> {
    ergo_sbe::generate_to_out_dir(
        "${repo_root}/sbe/tests/fixtures/schemas/example-schema.xml",
        ergo_sbe::GenerationConfig::new("messages"),
    )?;
    Ok(())
}
EOF

cat >"${scratch}/src/main.rs" <<'EOF'
#![allow(clippy::all, dead_code, unused)]
mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}

fn main() {
    let mut buf = [0u8; messages::CarEncoder::ENCODED_LENGTH];
    std::hint::black_box(messages::CarEncoder::wrap_and_apply_header(&mut buf, 0));
}
EOF

(
  cd "${scratch}"
  /usr/bin/time -p -o compile-time.txt cargo build --release
)

generated="$(find "${scratch}/target/release/build" -path '*/out/messages.rs' -print -quit)"
binary="${scratch}/target/release/ergo-sbe-cold-probe"
if [[ ! -f "${binary}" ]]; then
  binary="${binary}.exe"
fi

echo "generated source bytes: $(wc -c <"${generated}" | tr -d ' ')"
echo "fresh generated-crate compile:"
cat "${scratch}/compile-time.txt"
echo "final binary bytes: $(wc -c <"${binary}" | tr -d ' ')"
if command -v size >/dev/null 2>&1; then
  echo "binary sections:"
  size "${binary}"
fi
