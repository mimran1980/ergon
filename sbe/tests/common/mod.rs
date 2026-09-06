//! Shared test helpers for `ergon` integration tests.
//!
//! # Never set `CARGO_NET_OFFLINE` on the scratch-crate builds
//!
//! The `compile_and_run*` helpers write a throwaway crate and `cargo build`
//! it. That crate resolves its own dependency graph (`chrono`, `rust_decimal`,
//! `proptest`, …), whose transitive closure is **not** a subset of this
//! workspace's — so a CI runner's cargo cache does not contain it. Forcing
//! offline therefore fails on whichever transitive crate happens to be
//! missing, while passing on any developer machine with a warm `~/.cargo`.
//!
//! That exact bug turned `CI` red on every release commit from 0.1.19 to
//! 0.1.22 (`bit-set v0.8.0`, then `ahash v0.7.8`) and on Dependabot PRs
//! (`coverage` under `cargo llvm-cov` inherits `CARGO_NET_OFFLINE`). Always
//! `env_remove("CARGO_NET_OFFLINE")` on scratch cargo. Do not reintroduce
//! `--offline` on these helpers.
//!
//! # Codegen bug workaround
//!
//! The current codegen emits several known-compile errors. `patch_source()`
//! applies surgical string replacements so generated code compiles and runs
//! in tests.  This is a stopgap — each patch is tracked against a fixup todo
//! and should be removed once the codegen is fixed.

#![allow(missing_docs)]
#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Cargo for a throwaway crate. Clears `CARGO_NET_OFFLINE` inherited from
/// `cargo llvm-cov` (and similar harnesses) so extra scratch deps can download.
pub fn scratch_cargo() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.env_remove("CARGO_NET_OFFLINE");
    cmd
}

use ergo_sbe::{DomainVarData, GenerationConfig, Generator, Schema, parse_file};

pub struct Paths;

impl Paths {
    pub fn workspace_root() -> PathBuf {
        let cwd = std::env::current_dir().unwrap();
        for ancestor in cwd.ancestors() {
            if ancestor.join("Cargo.toml").exists() && (ancestor.join("sbe").exists()) {
                return ancestor.to_path_buf();
            }
        }
        let fallback = PathBuf::from("../..");
        if fallback.join("Cargo.toml").exists() {
            return fallback;
        }
        panic!("Cannot find workspace root from {cwd:?}");
    }

    pub fn sbe_dir() -> PathBuf {
        Self::workspace_root().join("sbe")
    }

    fn fixtures_dir() -> PathBuf {
        Self::sbe_dir()
            .join("tests")
            .join("fixtures")
            .join("schemas")
    }

    fn sbe_tool_test() -> PathBuf {
        Self::fixtures_dir()
    }

    pub fn example_schema() -> PathBuf {
        Self::fixtures_dir().join("example-schema.xml")
    }

    pub fn extension_schema() -> PathBuf {
        Self::fixtures_dir().join("extension-schema.xml")
    }

    pub fn bigendian_schema() -> PathBuf {
        Self::fixtures_dir().join("example-bigendian-test-schema.xml")
    }

    pub fn basic_variable_length_schema() -> PathBuf {
        Self::fixtures_dir().join("basic-variable-length-schema.xml")
    }

    pub fn fixed_array_schema() -> PathBuf {
        Self::fixtures_dir().join("fixed-sized-primitive-array-types.xml")
    }

    pub fn optional_enum_nullify_schema() -> PathBuf {
        Self::fixtures_dir().join("optional_enum_nullify.xml")
    }

    pub fn float_composite_schema() -> PathBuf {
        Self::fixtures_dir().join("float-composite-schema.xml")
    }

    pub fn all_types_le_schema() -> PathBuf {
        Self::fixtures_dir().join("all-types-le-schema.xml")
    }

    pub fn all_types_be_schema() -> PathBuf {
        Self::fixtures_dir().join("all-types-be-schema.xml")
    }

    pub fn custom_header_layout_schema() -> PathBuf {
        Self::fixtures_dir().join("custom-header-layout-schema.xml")
    }

    pub fn custom_header_layout_be_schema() -> PathBuf {
        Self::fixtures_dir().join("custom-header-layout-be-schema.xml")
    }

    pub fn uint64_vardata_be_schema() -> PathBuf {
        Self::fixtures_dir().join("uint64-vardata-be-schema.xml")
    }

    pub fn issue_schema(num: &str) -> PathBuf {
        Self::fixtures_dir().join(format!("issue{num}.xml"))
    }

    /// L3 orderbook: two sequential top-level groups (`bids` then `asks`), each
    /// with a nested `orders` group + `orderId` var-data. Canonical dual-group
    /// fixture for consuming-stage proofs.
    pub fn l3_orderbook_schema() -> PathBuf {
        Self::fixtures_dir().join("l3-orderbook-schema.xml")
    }

    pub fn versioned_l3_schema(version: u8) -> PathBuf {
        Self::fixtures_dir().join(format!("versioned-l3-v{version}.xml"))
    }

    /// Versioned groups and var-data used by the ordered one-pass decoder tests.
    pub fn ordered_decoder_version_tails_schema() -> PathBuf {
        Self::fixtures_dir().join("ordered-decoder-version-tails.xml")
    }

    pub fn bool_semantic_schema() -> PathBuf {
        Self::fixtures_dir().join("bool-semantic-schema.xml")
    }

    pub fn versioned_domain_schema() -> PathBuf {
        Self::fixtures_dir().join("versioned-domain-schema.xml")
    }

    /// A group with 2+ nested groups and 2+ var-data fields — exercises the
    /// ng_idx and nvd_idx counters in `generate_group_decoder`.
    pub fn multi_nested_group_schema() -> PathBuf {
        Self::fixtures_dir().join("multi-nested-group-schema.xml")
    }

    pub fn baseline_binary() -> PathBuf {
        Self::sbe_dir()
            .join("tests")
            .join("fixtures")
            .join("car_example_baseline_data.sbe")
    }

    pub fn extension_binary() -> PathBuf {
        Self::sbe_dir()
            .join("tests")
            .join("fixtures")
            .join("car_example_extension_data.sbe")
    }

    /// Generic path to a resource in `sbe-tool/src/test/resources/`.
    pub fn sbe_tool_test_resource(name: &str) -> PathBuf {
        Self::sbe_tool_test().join(name)
    }

    /// Checked-in sbe-tool Rust reference crate for dual-encode wire parity.
    /// Layout: `sbe/tests/sbe_tool_reference/<key>/` (Cargo package `parity_<key>`).
    pub fn sbe_tool_reference(key: &str) -> PathBuf {
        Self::sbe_dir()
            .join("tests")
            .join("sbe_tool_reference")
            .join(key)
    }
}

pub fn generate(xml_path: &Path, module_name: &str) -> (Schema, String) {
    let ir = parse_file(xml_path).unwrap_or_else(|e| panic!("parse {xml_path:?}: {e}"));
    let schema = Schema::from_ir(ir);
    let mut g = Generator::new(GenerationConfig::new(module_name));
    let (modules, _warnings) = g.generate(&schema).unwrap().into_parts();
    let module = modules.into_iter().next().unwrap();
    (schema, module.source)
}

/// Verify generated source parses as valid Rust syntax and contains expected items.
pub fn assert_source_ok(src: &str, expected: &[&str]) {
    syn::parse_file(src).expect("generated code is not valid Rust");
    for item in expected {
        assert!(src.contains(item), "missing expected item: {item}");
    }
}

/// Apply surgical patches for known codegen bugs.
pub fn patch_source(src: &str) -> String {
    // no patches needed currently; if a new codegen bug requires patching, add the patch here and record the bug; delete this function if it stays empty two releases
    // Entry encoders use the unsafe borrow split directly.
    // Message encoders take `mut self` by value so no borrow conflict.
    src.to_string()
}

/// Write generated source + a `main()` test body into a temp crate, compile,
/// and run.  `code` is placed directly inside `main()`.
pub fn compile_and_run(module_name: &str, source: &str, code: &str) {
    let _ = _compile_and_run(module_name, source, code, &[], "");
}

/// Like [`compile_and_run`], but returns combined stdout (for keep-gate samples).
pub fn compile_and_run_capture(module_name: &str, source: &str, code: &str) -> String {
    _compile_and_run(module_name, source, code, &[], "")
}

/// Like [`compile_and_run`] but appends `deps` to `[dependencies]` in the
/// temp crate's `Cargo.toml` (e.g. `"chrono = \"0.4\"\n"`).
pub fn compile_and_run_with_deps(module_name: &str, source: &str, code: &str, deps: &str) {
    let _ = _compile_and_run(module_name, source, code, &[], deps);
}

/// Diagnostic-checked negative proof: write generated source + a `main()` body
/// into a temp crate and require both a compile failure and every supplied
/// diagnostic fragment. This prevents an unrelated syntax/import error from
/// making a UI contract test pass accidentally.
pub fn compile_fails_with_diagnostics(
    module_name: &str,
    source: &str,
    code: &str,
    expected_diagnostics: &[&str],
) {
    assert!(
        !expected_diagnostics.is_empty(),
        "compile-fail test {module_name} must name its intended diagnostic"
    );
    let dir = std::env::temp_dir().join(format!("ergo_test_cf_{module_name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();

    let patched = patch_source(source);
    fs::write(src.join(format!("{module_name}.rs")), &patched).unwrap();

    let main = format!(
        "#![allow(dead_code,unused_imports,unused_variables)]\n\
         mod {module_name};\nuse {module_name}::*;\nfn main() {{\n{code}\n}}\n"
    );
    fs::write(src.join("main.rs"), &main).unwrap();

    let sbe_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("sbe");
    let cargo = format!(
        "[package]\nname=\"{module_name}_cf\"\nversion=\"0.1.0\"\nedition=\"2024\"\n\
         [dependencies]\n\
         ergo-sbe = {{ path = \"{}\", features = [\"compact_str\", \"smol_str\", \"bytes\", \"chrono\"] }}\n",
        sbe_path.display(),
    );
    fs::write(dir.join("Cargo.toml"), &cargo).unwrap();

    let target_dir = dir.join("target_ci");
    let out = scratch_cargo()
        .args(["build"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        // A compile-fail fixture asserts that a lint or type error FIRES. Ambient
        // RUSTFLAGS can silence exactly that: `cargo mutants` runs with
        // `cap_lints = true` (.cargo/mutants.toml), which caps every lint to
        // `allow`, so a `#[deny(unused_must_use)]` fixture compiled cleanly and
        // the test reported "expected a compile error, but the crate built
        // successfully". Clear both spellings so the fixture's own attributes
        // decide the outcome, not the environment that invoked the suite.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .expect("cargo build failed");

    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = fs::remove_dir_all(&dir);

    if out.status.success() {
        panic!(
            "compile_fails_with_diagnostics {module_name}: expected a compile error, but the crate built successfully.\nstderr:\n{stderr}"
        );
    }
    for expected in expected_diagnostics {
        assert!(
            stderr.contains(expected),
            "compile-fail {module_name} failed for the wrong reason: missing diagnostic fragment {expected:?}\nstderr:\n{stderr}"
        );
    }
}

/// Like `compile_and_run` but adds the given feature to `[features]` in the
/// temp crate's `Cargo.toml` and passes `--features <feature>` at build time.
pub fn compile_and_run_with_feature(module_name: &str, source: &str, code: &str, feature: &str) {
    let _ = _compile_and_run(module_name, source, code, &[feature], "");
}

fn _compile_and_run(
    module_name: &str,
    source: &str,
    code: &str,
    features: &[&str],
    deps: &str,
) -> String {
    let dir = std::env::temp_dir().join(format!("ergo_test_{module_name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();

    let patched = patch_source(source);
    fs::write(src.join(format!("{module_name}.rs")), &patched).unwrap();

    let main = format!(
        "#![allow(dead_code,unused_imports,unused_variables)]\n\
         mod {module_name};\nuse {module_name}::*;\n\
         fn main() -> Result<(), Box<dyn std::error::Error>> {{\n{code}\nOk(())\n}}\n"
    );
    fs::write(src.join("main.rs"), &main).unwrap();

    let sbe_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("sbe");
    let mut cargo = format!(
        "[package]\nname=\"{module_name}_test\"\nversion=\"0.1.0\"\nedition=\"2024\"\n\
         [dependencies]\n",
    );
    // Only add ergo-sbe if the caller hasn't already provided a custom dep string.
    if !deps.contains("ergo-sbe") {
        cargo.push_str(&format!(
            "ergo-sbe = {{ path = \"{}\", features = [\"compact_str\", \"smol_str\", \"bytes\", \"chrono\"] }}\n",
            sbe_path.display(),
        ));
    }
    if !features.is_empty() {
        cargo.push_str("[features]\n");
        for f in features {
            cargo.push_str(&format!("{f} = []\n"));
        }
    }
    if !deps.is_empty() {
        cargo.push_str(deps);
    }
    fs::write(dir.join("Cargo.toml"), &cargo).unwrap();

    let mut args: Vec<&str> = vec!["run"];
    for f in features {
        args.push("--features");
        args.push(f);
    }
    let target_dir = dir.join("target_ci");
    let out = scratch_cargo()
        .args(&args)
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .expect("cargo run failed");

    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let _ = fs::remove_dir_all(&dir);

    if !out.status.success() {
        let e = String::from_utf8_lossy(&out.stderr);
        panic!("test {module_name} FAILED\nstdout:\n{stdout}\nstderr:\n{e}");
    }
    stdout
}

/// Generate, compile, and run a test against a binary fixture.
/// `code` is placed in `main()` and can refer to the fixture bytes via `FIXTURE`.
pub fn run_fixture_test(name: &str, schema: &Path, fixture: &Path, code: &str) {
    let bytes = fs::read(fixture).unwrap_or_else(|e| panic!("fixture {fixture:?}: {e}"));
    let hex = bytes
        .iter()
        .map(|b| format!("0x{b:02x}u8"))
        .collect::<Vec<_>>()
        .join(", ");
    let (_, src) = generate(schema, name);
    let body = format!("let FIXTE: &[u8] = &[{hex}];\n{code}");
    compile_and_run(name, &src, &body);
}

/// Dual-encode wire parity: generate ergo-sbe for `schema`, path-depend on the
/// checked-in sbe-tool reference crate `tool_key`, then compile+run `code`.
///
/// Inside `code`:
/// - `use ergo::*;` is already applied
/// - sbe-tool crate is available as `tool::...` (`package = "parity_<tool_key>"`)
/// - helper `assert_frames_eq(label, ergo, tool)` is in scope
///
/// The test body should encode the same logical payload with both generators
/// and call `assert_frames_eq`.
pub fn dual_encode_run(test_name: &str, schema: &Path, tool_key: &str, code: &str) {
    let (_, ergo_src) = generate(schema, "ergo");
    dual_encode_run_modules(test_name, &[("ergo", &ergo_src)], tool_key, code);
}

/// Like [`dual_encode_run`] but with several ergo-sbe modules in one crate —
/// e.g. the same schema generated at different `with_encode_version` settings,
/// so one sbe-tool oracle can be given wire from every acting version.
///
/// The first module is also glob-imported (`use <first>::*;`) so single-module
/// bodies read the same as [`dual_encode_run`].
pub fn dual_encode_run_modules(
    test_name: &str,
    modules: &[(&str, &str)],
    tool_key: &str,
    code: &str,
) {
    assert!(
        !modules.is_empty(),
        "dual_encode_run_modules({test_name}) requires at least one module"
    );
    let tool_path = Paths::sbe_tool_reference(tool_key);
    assert!(
        tool_path.join("Cargo.toml").is_file(),
        "missing sbe-tool reference crate at {tool_path:?}; regenerate with scripts/regenerate-sbe-tool-reference.sh"
    );
    let tool_path_str = tool_path
        .canonicalize()
        .unwrap_or_else(|e| panic!("canonicalize {tool_path:?}: {e}"))
        .display()
        .to_string();
    // Escape backslashes for Windows paths in TOML strings.
    let tool_path_toml = tool_path_str.replace('\\', "/");
    let package = format!("parity_{tool_key}");

    let dir = std::env::temp_dir().join(format!("ergo_dual_{test_name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();

    let mut mod_decls = String::new();
    for (name, source) in modules {
        fs::write(src.join(format!("{name}.rs")), patch_source(source)).unwrap();
        mod_decls.push_str(&format!("mod {name};\n"));
    }
    mod_decls.push_str(&format!("use {}::*;\n", modules[0].0));

    let main = format!(
        r#"#![allow(dead_code, unused_imports, unused_variables, unused_mut, unused_assignments, clippy::all)]
{mod_decls}

fn assert_frames_eq(label: &str, ergo: &[u8], tool: &[u8]) {{
    assert_eq!(ergo.len(), tool.len(), "{{label}}: encoded length mismatch — ergon={{}}, sbe_tool={{}}", ergo.len(), tool.len());
    if ergo != tool {{
        let n = ergo.len().min(tool.len());
        let mut first = None;
        for i in 0..n {{
            if ergo[i] != tool[i] {{
                first = Some(i);
                break;
            }}
        }}
        panic!(
            "{{}}: frames differ ergo_len={{}} tool_len={{}} first_mismatch={{:?}}\\n  ergo[:64]={{:02x?}}\\n  tool[:64]={{:02x?}}",
            label,
            ergo.len(),
            tool.len(),
            first,
            &ergo[..ergo.len().min(64)],
            &tool[..tool.len().min(64)],
        );
    }}
}}

fn main() -> Result<(), Box<dyn std::error::Error>> {{
{code}
Ok(())
}}
"#
    );
    fs::write(src.join("main.rs"), &main).unwrap();

    // The generated ergo module references `ergo_sbe::…` for the optional
    // string/byte types, so the dependency must be present regardless of which
    // features the outer test run enables.
    let sbe_path_toml = Paths::sbe_dir().display().to_string().replace('\\', "\\\\");
    let cargo = format!(
        r#"[package]
name = "{test_name}_dual"
version = "0.1.0"
edition = "2024"

[workspace]

[dependencies]
tool = {{ path = "{tool_path_toml}", package = "{package}" }}
ergo-sbe = {{ path = "{sbe_path_toml}", features = ["compact_str", "smol_str", "bytes", "chrono"] }}
"#
    );
    fs::write(dir.join("Cargo.toml"), &cargo).unwrap();

    let target_dir = dir.join("target_ci");
    let out = scratch_cargo()
        .args(["run", "--quiet"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .expect("cargo run failed to start");

    if !out.status.success() {
        let e = String::from_utf8_lossy(&out.stderr);
        let o = String::from_utf8_lossy(&out.stdout);
        // Keep the temp dir on failure for debugging.
        panic!(
            "dual_encode {test_name} FAILED (dir={})\nstdout:\n{o}\nstderr:\n{e}",
            dir.display()
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Cross-check the generated message-header decoder against the matching
/// sbe-tool codec and independently constructed wire bytes.
///
/// This deliberately exercises decoding separately from full-frame encoder
/// parity: an encoder and decoder can disagree while self-roundtrip tests still
/// pass. `big_endian` is supplied by the test matrix rather than read from
/// ergo-sbe's IR, so a byte-order regression cannot bless itself.
pub fn dual_header_decode_run(test_name: &str, schema: &Path, tool_key: &str, big_endian: bool) {
    let code = format!(
        r###"
        use tool::message_header_codec::MessageHeaderDecoder as ToolHeaderDecoder;

        let cases = [
            (0u16, 1u16, 2u16, 3u16),
            (0x1234u16, 0x5678u16, 0x2345u16, 0x6789u16),
            (u16::MAX - 1, u16::MAX - 2, u16::MAX - 3, u16::MAX - 4),
        ];

        for (block_length, template_id, schema_id, version) in cases {{
            let to_wire = |value: u16| -> [u8; 2] {{
                if {big_endian} {{
                    value.to_be_bytes()
                }} else {{
                    value.to_le_bytes()
                }}
            }};

            let mut wire = [0u8; MESSAGE_HEADER_ENCODED_LENGTH];
            wire[0..2].copy_from_slice(&to_wire(block_length));
            wire[2..4].copy_from_slice(&to_wire(template_id));
            wire[4..6].copy_from_slice(&to_wire(schema_id));
            wire[6..8].copy_from_slice(&to_wire(version));

            let ergo_header = MessageHeader(wire);
            assert_eq!(ergo_header.block_length(), block_length, "ergo blockLength");
            assert_eq!(ergo_header.template_id(), template_id, "ergo templateId");
            assert_eq!(ergo_header.schema_id(), schema_id, "ergo schemaId");
            assert_eq!(ergo_header.version(), version, "ergo version");
            assert_eq!(
                MessageHeader::peek_header(&wire),
                Some(PeekedHeader {{ template_id, schema_id }}),
                "ergo header peek",
            );

            let tool_header =
                ToolHeaderDecoder::default().wrap(tool::ReadBuf::new(&wire), 0);
            assert_eq!(tool_header.block_length(), block_length, "tool blockLength");
            assert_eq!(tool_header.template_id(), template_id, "tool templateId");
            assert_eq!(tool_header.schema_id(), schema_id, "tool schemaId");
            assert_eq!(tool_header.version(), version, "tool version");

        }}
        println!("PASS: {test_name}");
        "###,
    );
    dual_encode_run(test_name, schema, tool_key, &code);
}

/// Generate two modules, write them into the same temp crate, compile, and run.
/// `module_a` / `source_a` and `module_b` / `source_b` are written as separate
/// Rust source files; `code` goes inside `main()` and can `use` both modules.
pub fn compile_and_run_two_modules(
    test_name: &str,
    module_a: &str,
    source_a: &str,
    module_b: &str,
    source_b: &str,
    code: &str,
) {
    compile_and_run_modules(
        test_name,
        &[(module_a, source_a), (module_b, source_b)],
        code,
    );
}

/// Generate `modules` (name, source) into one temp crate, compile, and run.
/// `code` goes inside `main()` and can `use` every named module.
pub fn compile_and_run_modules(test_name: &str, modules: &[(&str, &str)], code: &str) {
    assert!(
        !modules.is_empty(),
        "compile_and_run_modules({test_name}) requires at least one module"
    );
    let dir = std::env::temp_dir().join(format!("ergo_test_{test_name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();

    let mut mod_decls = String::new();
    for (name, source) in modules {
        fs::write(src.join(format!("{name}.rs")), patch_source(source)).unwrap();
        mod_decls.push_str(&format!("mod {name};\n"));
    }

    let main = format!(
        "#![allow(dead_code,unused_imports,unused_variables)]\n\
         {mod_decls}\
         fn main() -> Result<(), Box<dyn std::error::Error>> {{\n{code}\nOk(())\n}}\n"
    );
    fs::write(src.join("main.rs"), &main).unwrap();

    let sbe_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("sbe");
    let cargo = format!(
        "[package]\nname=\"{test_name}_test\"\nversion=\"0.1.0\"\nedition=\"2024\"\n\
         [dependencies]\n\
         ergo-sbe = {{ path = \"{}\", features = [\"compact_str\", \"smol_str\", \"bytes\", \"chrono\"] }}\n",
        sbe_path.display(),
    );
    fs::write(dir.join("Cargo.toml"), &cargo).unwrap();

    let target_dir = dir.join("target_ci");
    let out = scratch_cargo()
        .args(["run"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .expect("cargo run failed");

    let _ = fs::remove_dir_all(&dir);

    if !out.status.success() {
        let e = String::from_utf8_lossy(&out.stderr);
        let o = String::from_utf8_lossy(&out.stdout);
        panic!("test {test_name} FAILED\nstdout:\n{o}\nstderr:\n{e}");
    }
}

pub fn generate_domain(xml_path: &Path, module_name: &str) -> (Schema, String) {
    generate_domain_with(xml_path, module_name, |c| {
        c.with_domain_objects(DomainVarData::Bytes)
    })
}

pub fn generate_domain_with(
    xml_path: &Path,
    module_name: &str,
    configure: impl FnOnce(GenerationConfig) -> GenerationConfig,
) -> (Schema, String) {
    let ir = parse_file(xml_path).unwrap_or_else(|e| panic!("parse {xml_path:?}: {e}"));
    let schema = Schema::from_ir(ir);
    let config = configure(GenerationConfig::new(module_name));
    let mut g = Generator::new(config);
    let ms = g.generate(&schema).unwrap();
    let module = ms.modules().next().unwrap();
    (schema, module.source.clone())
}

pub mod encoded_length_matrix;
