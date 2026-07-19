//! Shared test helpers for `ErgoSBE` integration tests.
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

use ergo_sbe::{GenerationConfig, Generator, Schema, parse_file};

// ── Schema & fixture path resolution ──────────────────────────────────

pub struct Paths;

impl Paths {
    fn workspace_root() -> PathBuf {
        let cwd = std::env::current_dir().unwrap();
        for ancestor in cwd.ancestors() {
            if ancestor.join("Cargo.toml").exists() && (ancestor.join("sbe").exists()) {
                return ancestor.to_path_buf();
            }
        }
        // Fallback: running from crate dir
        let fallback = PathBuf::from("../..");
        if fallback.join("Cargo.toml").exists() {
            return fallback;
        }
        panic!("Cannot find workspace root from {cwd:?}");
    }

    fn sbe_dir() -> PathBuf {
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

    pub fn issue_schema(num: &str) -> PathBuf {
        Self::fixtures_dir().join(format!("issue{num}.xml"))
    }

    /// L3 orderbook: two sequential top-level groups (`bids` then `asks`), each
    /// with a nested `orders` group + `orderId` var-data. The canonical
    /// dual-group fixture for DECISIONS.md §3 consuming-stage proofs.
    pub fn l3_orderbook_schema() -> PathBuf {
        Self::fixtures_dir().join("l3-orderbook-schema.xml")
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
}

// ── Code generation helpers ──────────────────────────────────────────

/// Parse a schema XML file and generate `ErgoSBE` Rust source.
pub fn generate(xml_path: &Path, module_name: &str) -> (Schema, String) {
    let ir = parse_file(xml_path).unwrap_or_else(|e| panic!("parse {xml_path:?}: {e}"));
    let schema = Schema::from_ir(ir);
    let g = Generator::new(GenerationConfig::new(module_name));
    let ms = g.generate(&schema);
    let module = ms.modules().next().unwrap();
    (schema, module.source.clone())
}

/// Verify generated source parses as valid Rust syntax and contains expected items.
pub fn assert_source_ok(src: &str, expected: &[&str]) {
    syn::parse_file(src).expect("generated code is not valid Rust");
    for item in expected {
        assert!(src.contains(item), "missing expected item: {item}");
    }
}

// ── Known codegen bug patches ───────────────────────────────────────
//
// Each patch corresponds to a codegen bug that will be fixed in a
// separate PR.  Remove patches as their upstream fixes land.

/// Apply surgical patches for known codegen bugs.
pub fn patch_source(src: &str) -> String {
    // ponytail: all patches are dead — codegen now produces correct code.
    // Entry encoders use the unsafe borrow split directly.
    // Message encoders take `mut self` by value so no borrow conflict.
    src.to_string()
}

// ── Compile and run generated code ───────────────────────────────────

/// Write generated source + a `main()` test body into a temp crate, compile,
/// and run.  `code` is placed directly inside `main()`.
pub fn compile_and_run(module_name: &str, source: &str, code: &str) {
    _compile_and_run(module_name, source, code, &[], "");
}

/// Negative-proof helper: write generated source + a `main()` body into a temp
/// crate and assert that it FAILS to compile. Used for compile-fail API proofs
/// (DECISIONS.md §11 / todo 137): out-of-order tail access, reused consumed
/// stages, etc. `code` is placed directly inside `main()`.
pub fn compile_fails(module_name: &str, source: &str, code: &str) {
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

    let cargo =
        format!("[package]\nname=\"{module_name}_cf\"\nversion=\"0.1.0\"\nedition=\"2024\"\n");
    fs::write(dir.join("Cargo.toml"), &cargo).unwrap();

    let target_dir = dir.join("target_ci");
    let out = Command::new("cargo")
        .args(["build"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .expect("cargo build failed");

    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = fs::remove_dir_all(&dir);

    if out.status.success() {
        panic!(
            "compile_fails {module_name}: expected a compile error, but the crate built successfully.\nstderr:\n{stderr}"
        );
    }
    // Keep stderr reachable for diagnostics via the returned-into-owned value above;
    // callers may extend this to assert specific error text.
}

/// Like `compile_and_run` but adds the given feature to `[features]` in the
/// temp crate's `Cargo.toml` and passes `--features <feature>` at build time.
pub fn compile_and_run_with_feature(module_name: &str, source: &str, code: &str, feature: &str) {
    _compile_and_run(module_name, source, code, &[feature], "");
}

/// Like `compile_and_run` but enables the generated module's `serde` feature
/// and adds serde + serde_json as dependencies, for Serialize/Deserialize
/// round-trip tests. Requires the crates in the cargo registry cache.
pub fn compile_and_run_serde(module_name: &str, source: &str, code: &str) {
    const DEPS: &str = "serde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"\n";
    _compile_and_run(module_name, source, code, &["serde"], DEPS);
}

fn _compile_and_run(module_name: &str, source: &str, code: &str, features: &[&str], deps: &str) {
    let dir = std::env::temp_dir().join(format!("ergo_test_{module_name}"));
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

    let mut cargo =
        format!("[package]\nname=\"{module_name}_test\"\nversion=\"0.1.0\"\nedition=\"2024\"\n");
    if !features.is_empty() {
        cargo.push_str("[features]\n");
        for f in features {
            cargo.push_str(&format!("{f} = []\n"));
        }
    }
    if !deps.is_empty() {
        cargo.push_str("[dependencies]\n");
        cargo.push_str(deps);
    }
    fs::write(dir.join("Cargo.toml"), &cargo).unwrap();

    let mut args: Vec<&str> = vec!["run"];
    for f in features {
        args.push("--features");
        args.push(f);
    }
    let target_dir = dir.join("target_ci");
    let out = Command::new("cargo")
        .args(&args)
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .expect("cargo run failed");

    let _ = fs::remove_dir_all(&dir);

    if !out.status.success() {
        let e = String::from_utf8_lossy(&out.stderr);
        let o = String::from_utf8_lossy(&out.stdout);
        panic!("test {module_name} FAILED\nstdout:\n{o}\nstderr:\n{e}");
    }
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
    let dir = std::env::temp_dir().join(format!("ergo_test_{test_name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(src.join(format!("{module_a}.rs")), source_a).unwrap();
    fs::write(src.join(format!("{module_b}.rs")), source_b).unwrap();

    let main = format!(
        "#![allow(dead_code,unused_imports,unused_variables)]\n\
         mod {module_a};\nmod {module_b};\nfn main() {{\n{code}\n}}\n"
    );
    fs::write(src.join("main.rs"), &main).unwrap();

    let cargo =
        format!("[package]\nname=\"{test_name}_test\"\nversion=\"0.1.0\"\nedition=\"2024\"\n");
    fs::write(dir.join("Cargo.toml"), &cargo).unwrap();

    let target_dir = dir.join("target_ci");
    let out = Command::new("cargo")
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

/// Generate with domain objects enabled.
pub fn generate_domain(xml_path: &Path, module_name: &str) -> (Schema, String) {
    let ir = parse_file(xml_path).unwrap_or_else(|e| panic!("parse {xml_path:?}: {e}"));
    let schema = Schema::from_ir(ir);
    let mut config = GenerationConfig::new(module_name);
    config.domain_objects = true;
    let g = Generator::new(config);
    let ms = g.generate(&schema);
    let module = ms.modules().next().unwrap();
    (schema, module.source.clone())
}
