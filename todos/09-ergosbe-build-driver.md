# build.rs driver (ergosbe-build)

**Blocked by:** `01-scalar-wire-parity` (can start early, progressively integrate)

Developer-facing `build.rs` integration. Single call generates a module into
`OUT_DIR`. No CLI — the generator library is the single source of truth.

## Acceptance criteria

- [ ] `ergosbe-build` crate: `ergosbe_build::generate("schema.xml", "mod_name")`
- [ ] Writes generated `.rs` into `OUT_DIR`, user `include!()`s it
- [ ] Schema path resolution relative to `CARGO_MANIFEST_DIR`
- [ ] Rerun-if-changed on the schema file
- [ ] Example crate demonstrating end-to-end usage
- [ ] Shared runtime opt-in (`shared_runtime` config flag)

Ref: `design/DECISIONS.md` §11 slice 12, §10.
