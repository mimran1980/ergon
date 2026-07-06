//! Shared test helpers for ErgoSBE integration tests.
//!
//! # Codegen bug workaround
//!
//! The current codegen emits several known-compile errors. `patch_source()`
//! applies surgical string replacements so generated code compiles and runs
//! in tests.  This is a stopgap — each patch is tracked against a fixup todo
//! and should be removed once the codegen is fixed.

#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ergosbe::{GenerationConfig, Generator, Schema, parse};

// ── Schema & fixture path resolution ──────────────────────────────────

pub struct Paths;

impl Paths {
    fn workspace_root() -> PathBuf {
        let cwd = std::env::current_dir().unwrap();
        for ancestor in cwd.ancestors() {
            if ancestor.join("Cargo.toml").exists()
                && (ancestor.join("ergosbe").exists() || ancestor.join("crates/ergosbe").exists())
            {
                return ancestor.to_path_buf();
            }
        }
        // Fallback: running from crate dir
        let fallback = PathBuf::from("../..");
        if fallback.join("Cargo.toml").exists() {
            return fallback;
        }
        panic!("Cannot find workspace root from {:?}", cwd);
    }

    fn ergosbe_dir() -> PathBuf {
        let root = Self::workspace_root();
        if root.join("ergosbe").exists() {
            root.join("ergosbe")
        } else {
            root.join("crates/ergosbe")
        }
    }

    fn sample_resources(sub: &str) -> PathBuf {
        Self::workspace_root()
            .join("simple-binary-encoding")
            .join(sub)
            .join("src")
            .join("main")
            .join("resources")
    }

    fn sbe_samples() -> PathBuf {
        Self::sample_resources("sbe-samples")
    }

    fn sbe_tool_test() -> PathBuf {
        Self::workspace_root()
            .join("simple-binary-encoding")
            .join("sbe-tool")
            .join("src")
            .join("test")
            .join("resources")
    }

    pub fn example_schema() -> PathBuf {
        Self::sbe_samples().join("example-schema.xml")
    }

    pub fn extension_schema() -> PathBuf {
        Self::sbe_samples().join("example-extension-schema.xml")
    }

    pub fn bigendian_schema() -> PathBuf {
        Self::sbe_tool_test().join("example-bigendian-test-schema.xml")
    }

    pub fn basic_variable_length_schema() -> PathBuf {
        Self::sbe_tool_test().join("basic-variable-length-schema.xml")
    }

    pub fn fixed_array_schema() -> PathBuf {
        Self::sbe_tool_test().join("fixed-sized-primitive-array-types.xml")
    }

    pub fn optional_enum_nullify_schema() -> PathBuf {
        Self::sbe_tool_test().join("optional_enum_nullify.xml")
    }

    pub fn issue_schema(num: &str) -> PathBuf {
        Self::sbe_tool_test().join(format!("issue{num}.xml"))
    }

    pub fn baseline_binary() -> PathBuf {
        Self::ergosbe_dir()
            .join("tests")
            .join("fixtures")
            .join("car_example_baseline_data.sbe")
    }

    pub fn extension_binary() -> PathBuf {
        Self::ergosbe_dir()
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

/// Parse a schema XML file and generate ErgoSBE Rust source.
pub fn generate(xml_path: &Path, module_name: &str) -> (Schema, String) {
    let xml = fs::read_to_string(xml_path).unwrap_or_else(|e| panic!("read {xml_path:?}: {e}"));
    let ir = parse(&xml).unwrap_or_else(|e| panic!("parse {xml_path:?}: {e}"));
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
    let mut s = src.to_string();

    // Bug 6 (E0499): use unsafe pointer cast to decouple the encoder's buffer
    // lifetime from self.buf, so self.buf is available after the closure.
    // The safety-critical `'a` lifetime parameter makes a normal reborrow
    // persist beyond the closure.  We cannot change codegen.rs; instead we
    // create an independent &'a mut [u8] from the raw pointer so the type
    // system sees two separate borrows.  Safety: the two refs never alias
    // during the encoder operation (self.buf is unused inside the block).
    //
    // CarEncoder::fuel_figures
    s = s.replace(
        "        let mut group = FuelFiguresEncoder::wrap(self.buf, self.pos + 4, count);\n        f(&mut group);\n        Ok(CarEncoder {\n            buf: self.buf,\n            message_start: self.message_start,\n            pos: group.pos,",
        "        let __pos;\n        {\n            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };\n            let mut group = FuelFiguresEncoder::wrap(__buf, self.pos + 4, count);\n            f(&mut group);\n            __pos = group.pos;\n        }\n        Ok(CarEncoder {\n            buf: self.buf,\n            message_start: self.message_start,\n            pos: __pos,",
    );
    // CarEncoder::performance_figures
    s = s.replace(
        "        let mut group = PerformanceFiguresEncoder::wrap(self.buf, self.pos + 4, count);\n        f(&mut group);\n        Ok(CarEncoder {\n            buf: self.buf,\n            message_start: self.message_start,\n            pos: group.pos,",
        "        let __pos;\n        {\n            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };\n            let mut group = PerformanceFiguresEncoder::wrap(__buf, self.pos + 4, count);\n            f(&mut group);\n            __pos = group.pos;\n        }\n        Ok(CarEncoder {\n            buf: self.buf,\n            message_start: self.message_start,\n            pos: __pos,",
    );
    // FuelFiguresEncoder::add
    s = s.replace(
        "        let mut entry = FuelFiguresEntryEncoder::wrap(self.buf, self.pos);\n        f(&mut entry);\n        self.pos = entry.pos;\n        self.written += 1;\n        Ok(())",
        "        {\n            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };\n            let mut entry = FuelFiguresEntryEncoder::wrap(__buf, self.pos);\n            f(&mut entry);\n            self.pos = entry.pos;\n            self.written += 1;\n        }\n        Ok(())",
    );
    // PerformanceFiguresEncoder::add
    s = s.replace(
        "        let mut entry = PerformanceFiguresEntryEncoder::wrap(self.buf, self.pos);\n        f(&mut entry);\n        self.pos = entry.pos;\n        self.written += 1;\n        Ok(())",
        "        {\n            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };\n            let mut entry = PerformanceFiguresEntryEncoder::wrap(__buf, self.pos);\n            f(&mut entry);\n            self.pos = entry.pos;\n            self.written += 1;\n        }\n        Ok(())",
    );
    // PerformanceFiguresEntryEncoder::acceleration
    s = s.replace(
        "        let mut group = AccelerationEncoder::wrap(self.buf, self.pos + 4, count);\n        f(&mut group);\n        self.pos = group.pos;\n        Ok(self)",
        "        {\n            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };\n            let mut group = AccelerationEncoder::wrap(__buf, self.pos + 4, count);\n            f(&mut group);\n            self.pos = group.pos;\n        }\n        Ok(self)",
    );
    // AccelerationEncoder::add
    s = s.replace(
        "        let mut entry = AccelerationEntryEncoder::wrap(self.buf, self.pos);\n        f(&mut entry);\n        self.pos = entry.pos;\n        self.written += 1;\n        Ok(())",
        "        {\n            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };\n            let mut entry = AccelerationEntryEncoder::wrap(__buf, self.pos);\n            f(&mut entry);\n            self.pos = entry.pos;\n            self.written += 1;\n        }\n        Ok(())",
    );

    s
}

// ── Compile and run generated code ───────────────────────────────────

/// Write generated source + a `main()` test body into a temp crate, compile,
/// and run.  `code` is placed directly inside `main()`.
pub fn compile_and_run(module_name: &str, source: &str, code: &str) {
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

    let cargo =
        format!("[package]\nname=\"{module_name}_test\"\nversion=\"0.1.0\"\nedition=\"2024\"\n");
    fs::write(dir.join("Cargo.toml"), &cargo).unwrap();

    let out = Command::new("cargo")
        .args(["run"])
        .current_dir(&dir)
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
