⚠️ **DEFERRED — post-v1.** ergosbe-build driver is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# build.rs driver (ergosbe-build)

**Blocked by:** `01-scalar-wire-parity` (can start early, progressively integrate)

Developer-facing `build.rs` integration. Single call generates a module into
`OUT_DIR`. No CLI — the generator library is the single source of truth.

## Acceptance criteria

- [x] `ergosbe-build` crate: `ergosbe_build::generate("schema.xml", "mod_name")`
- [x] Writes generated `.rs` into `OUT_DIR`, user `include!()`s it
- [x] Schema path resolution relative to `CARGO_MANIFEST_DIR`
- [x] Rerun-if-changed on the schema file
- [x] Example crate demonstrating end-to-end usage
- [x] Shared runtime opt-in (`shared_runtime` config flag)

Ref: `design/DECISIONS.md` §11 slice 12, §10.
