# Review tickets

## Quick wins — 0.1.xx

## T-1: Propagate invalid required booleans as `DecodeError`

- Type: CORRECTNESS
- Stage: 0.1.xx
- Priority: P0 · Effort: S
- Symptom: Domain-object generation turns a required boolean enum into `dec.try_*_bool().expect("null or invalid bool value")` (`sbe/src/codegen/domain_cluster.rs:449-464`). The generated `try_from_decoder` contract says it propagates malformed input instead of panicking, but the car golden contains that `expect` (`sbe/tests/golden/car_example.rs:6267-6333`), even though the underlying accessor already returns `DecodeError::InvalidBoolean` (`sbe/tests/golden/car_example.rs:2135-2145`). Existing domain tests cover valid booleans and invalid UTF-8, not a null/unknown required boolean (`sbe/tests/domain_objects_test.rs:252-323`, `sbe/tests/domain_objects_test.rs:739-789`). A temporary review probe against the current tree encoded `ToggleDomain`, replaced the required boolean discriminant with `0xff`, and confirmed that `try_from_slice_with_header` panics.
- Change: In `sbe/src/codegen/domain_cluster.rs`, emit `field: dec.try_<field>_bool()?` for required boolean fields, including fields nested in generated group DTOs. Preserve the existing `DecodeError::InvalidBoolean { field, discriminant }` without wrapping or stringifying it. SBE-pattern rationale: a boolean enum is still an untrusted wire discriminant (including `NullVal` and forward/unknown values); the fallible DTO materialisation boundary must fail closed with a typed error, never cross into a panic path.
- What breaks (API only), what it buys: No public signature breaks. The observable failure changes from an unwind to `Err(DecodeError::InvalidBoolean)`, so hostile or corrupt frames cannot crash a process that deliberately chose the `try_*` API.
- Acceptance criteria: Add root-message and repeating-group fixtures for both `NullVal` and an unknown boolean discriminant; assert the exact field and raw discriminant in `InvalidBoolean`; update `sbe/tests/golden/car_example.rs`; retain the valid boolean and invalid UTF-8 cases; add a regression assertion that no domain `try_from_decoder` body contains `expect(` or `unwrap(`.
- Verification plan: Run `cargo test -p ergo-sbe --test domain_objects_test`, the boolean/comprehensive tests, hostile-frame/property tests, `just check-golden`, and `just test-sbe`. Because generated decode code changes, run `just bench` and compare the maintained LTO-on and LTO-off ratios (all must remain `<= 1.00`); also compare the flyweight generated bodies to prove only the owned-domain conversion path changed.

## T-2: Escape schema prose before emitting rustdoc

- Type: CORRECTNESS
- Stage: 0.1.xx
- Priority: P1 · Effort: S
- Symptom: `sanitize_description_for_doc` returns every single-line description unchanged and `doc_attr_tokens` feeds it directly to rustdoc (`sbe/src/codegen/runtime.rs:2689-2707`). The valid benchmark schema contains prose such as `Option<EnumType>` and `Option<u32>` (`sbe/benchmarks/schemas/bench-null-option.xml:40-45`). On this tree, `RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe-benchmarks --no-deps` fails with 13 `rustdoc::invalid_html_tags` errors generated from those descriptions.
- Change: Escape schema-sourced `&`, `<`, and `>` in the single-line branch of `sanitize_description_for_doc` before constructing `#[doc]`; keep the existing multiline `text` fence, which already makes its content literal. Apply this one helper to every schema-description emission site. SBE-pattern rationale: schema descriptions are external schema data, not trusted rustdoc HTML; the schema-to-Rust boundary should preserve them as deterministic literal prose rather than letting them alter or invalidate generated documentation.
- What breaks (API only), what it buys: No Rust API breaks. Intentional raw HTML in an XML description will render literally; in return, an otherwise valid schema can no longer break a downstream `-D warnings` documentation build.
- Acceptance criteria: Extend `sbe/tests/schema_docs_provenance_test.rs:184-219` with single-line `Option<u32>`, `A & B`, and literal angle-bracket cases plus multiline indented prose; assert readable literal output and parseable generated Rust; update only doc-visible goldens; the benchmark crate's denied-warning rustdoc command passes.
- Verification plan: Run the focused provenance/generated-doc tests, `RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe-benchmarks --no-deps`, workspace doctests, `just check-golden`, and the generated API snapshot. Run `just bench` in both LTO modes because generator output changes; require all maintained ratios `<= 1.00` and confirm the generated executable-token diff is empty after stripping doc attributes.

## T-3: Preserve output paths in `BuildError`

- Type: API
- Stage: 0.1.xx
- Priority: P1 · Effort: S
- Symptom: Public `BuildError::Io(std::io::Error)` displays only `I/O error: ...` (`sbe/src/build.rs:39-56`). `generate_to_dir` and `write_module_set` know the failing directory or destination but discard it through bare `?` conversions (`sbe/src/build.rs:147-156`, `sbe/src/build.rs:361-384`). Schema-read failures already preserve their path and source (`sbe/src/xml/error.rs:103-112`), so output failures are materially harder to diagnose than input failures.
- Change: Make `BuildError` `#[non_exhaustive]` and replace the tuple variant with `Io { action: &'static str, path: PathBuf, #[source] source: std::io::Error }`. Map every output-directory creation and module-write site in `sbe/src/build.rs` with the exact attempted path and an action such as `"create output directory"` or `"write generated module"`. SBE-pattern rationale: code generation is a staged schema-to-artifact pipeline; an artifact failure must identify its stage and destination just as parsing identifies its source document.
- What breaks (API only), what it buys: Exhaustive matches and `BuildError::Io(_)` patterns must adopt a wildcard and the named fields. Users get a build-script diagnostic that identifies the exact failing artifact while preserving `Error::source` and `io::ErrorKind`; `#[non_exhaustive]` prevents the next diagnostic variant from forcing another exhaustive-match break.
- Acceptance criteria: Unit tests force output-directory creation failure and a specific generated-module write failure, assert action plus full path in `Display`, and assert that `source()` is the original I/O error; update rustdoc and every pattern match; successful generated bytes remain unchanged.
- Verification plan: Run focused `sbe::build` tests, workspace tests, denied-warning rustdoc, `just check-public-api`, and `just check-golden`. This touches only build/error paths, but repository policy still requires `just bench`; both LTO reports must remain `<= 1.00`, with hot generated source byte-identical.

## T-4: Validate ingress endpoint maps in the builder setter

- Type: API
- Stage: 0.1.xx
- Priority: P1 · Effort: S
- Symptom: Channel and timeout setters reject invalid values immediately (`cluster/src/config.rs:144-200`), while `SessionBuilder::ingress_endpoints` stores arbitrary text infallibly (`cluster/src/config.rs:216-220`). Missing `=`, bad IDs, empty endpoints, and duplicate IDs are already detected by `parse_ingress_endpoints` (`cluster/src/endpoints.rs:17-53`) but are deferred to `validate` (`cluster/src/config.rs:251-266`); async connect does not call that until the first `poll` (`cluster/src/client.rs:1182-1202`).
- Change: Change the target signature to `pub fn ingress_endpoints(mut self, endpoints: impl Into<String>) -> Result<Self, ClusterError>`, parse the owned string before storing it, and update callers with `?`. Document the exact `member_id=host:port` grammar and the absence of an `aeron:` prefix in `cluster/src/config.rs`, `cluster/README.md`, and `book/src/cluster/session-builder.md`. Do not add URI-level validation until compatibility evidence supports it. SBE/Aeron-pattern rationale: validate external connection metadata once at the builder trust boundary so malformed state never enters the consuming async-connect state machine.
- What breaks (API only), what it buys: Builder chains using `.ingress_endpoints(...)` must handle `Result`. A typo or duplicate member ID now fails on the line that supplied it instead of after transport setup/polling, and an invalid endpoint-map state is no longer representable in `SessionBuilder`.
- Acceptance criteria: Empty, missing-`=`, nonnumeric-ID, empty-endpoint, and duplicate-ID maps fail at the setter; valid unsorted input succeeds with current member-ID resolution semantics; all sync/async callers compile; the rustdoc, README, and named book chapter contain the exact grammar and one runnable multi-member example.
- Verification plan: Run endpoint/config tests, sync and async connect state tests, cluster examples, denied-warning rustdoc, workspace tests, and the public API check. Run `just bench-cluster`; maintained codec scenarios must not regress, and a source diff must show no generated codec or steady-state offer/poll body changes.

## T-5: Make poller helper outcomes truthful

- Type: API
- Stage: 0.1.xx
- Priority: P1 · Effort: S
- Symptom: `parse_event` returns `Result<Option<EgressEvent>, ClusterError>`, yet the unknown-template branch is `Some(Other)` and every decoded branch is also `Some` (`cluster/src/poller.rs:61-116`); the public example carries an unreachable nesting layer (`cluster/examples/failover_demo.rs:114-135`). Conversely, `parse_leader_endpoint` turns every detailed endpoint-map parse error into `None` via `.ok()?` (`cluster/src/poller.rs:119-135`), so callers cannot distinguish malformed network data from a valid map that lacks the requested leader (`cluster/src/client.rs:357-368`, `cluster/src/client.rs:841-852`).
- Change: Change `parse_event` to `Result<EgressEvent, ClusterError>`. Change `parse_leader_endpoint` to `Result<Option<String>, ClusterError>`: return `Err` for malformed maps, `Ok(None)` only for a well-formed map missing `leader_member_id`, and `Ok(Some(endpoint))` on success. Update sync connect, async connect, failover handling, tests, and the example. SBE/Aeron-pattern rationale: one successfully decoded fragment has a total template classification, while endpoint-map syntax failure and member absence are distinct trust-boundary states that should stay distinct in the type system.
- What breaks (API only), what it buys: Callers remove the impossible `Some`/`None` layer around events and add `?` around leader-map parsing. They gain exhaustive event handling and actionable malformed-map errors instead of a generic “leader missing” message.
- Acceptance criteria: Projected, unknown, and known-unprojected templates return direct `EgressEvent` variants with the original template ID; short/malformed frames stay typed errors; malformed endpoint maps preserve the parser's specific reason; a clean missing-member map is exactly `Ok(None)`; sync/async clients and `cluster/examples/failover_demo.rs` use the new shapes.
- Verification plan: Run poller, fragmentation, failover, endpoint, sync-connect, and async-connect tests plus examples, rustdoc, workspace tests, and the public API gate. Run `just bench-cluster`; all maintained ratios must pass and encoded bytes must be unchanged.

## T-6: Reject invalid keyword suffixes before code emission

- Type: CORRECTNESS
- Stage: 0.1.xx
- Priority: P1 · Effort: S
- Symptom: `GenerationConfig::with_keyword_append_token` accepts any string (`sbe/src/config.rs:780-793`), the generator appends it directly to Rust keywords (`sbe/src/codegen/runtime.rs:504-513`), and pre-emission validation checks module/type paths but not this token (`sbe/src/codegen/mod.rs:477-508`). The empty-token regression consequently expects a generic `InvalidGeneratedSource` after emission (`sbe/tests/reserved_name_clash_test.rs:464-503`) even though the fault is user configuration.
- Change: In `Generator::validate_paths`, prove that a representative Rust keyword plus the configured suffix is a valid, non-keyword `syn::Ident`; return `GenerateError::InvalidConfiguration { option: "keyword_append_token", value, reason }` for empty or invalid suffixes. Keep the builder setter infallible and validate at `generate`, matching the other stringly configuration options. SBE-pattern rationale: identifier policy is schema-to-language mapping configuration and must be rejected before it can produce an invalid codec artifact.
- What breaks (API only), what it buys: No signature break; callers matching the old generic error observe the more precise variant. Users see the bad option and value immediately instead of debugging invalid generated Rust.
- Acceptance criteria: Empty, whitespace, punctuation, and suffixes that still form a keyword return `InvalidConfiguration` without panicking; `_` and an alphanumeric suffix generate compilable keyword-named fields; update the reserved-name regression; default generated output is byte-for-byte unchanged.
- Verification plan: Run reserved-name and configuration tests, compile the positive fixture, `just check-golden`, generated API checks, and workspace tests. Run `just bench` in both LTO modes and require `<= 1.00`; valid-default hot-path source must be identical.

## T-7: Model `TooManyParts` without inventing a raw sentinel

- Type: API
- Stage: 0.1.xx
- Priority: P1 · Effort: S
- Symptom: `PublicationFailure::from_offer_error` maps the typed rusteron `AeronOfferError::TooManyParts` to `Other(-100)` (`cluster/src/error.rs:73-84`), although `-100` is not an Aeron offer sentinel. `raw()` then presents the fabricated value as genuine (`cluster/src/error.rs:93-104`), and the exhaustive public enum has no forward-extension protection (`cluster/src/error.rs:41-57`).
- Change: Make `PublicationFailure` `#[non_exhaustive]`, add `TooManyParts`, map the rusteron variant directly, and change `raw` to `raw_code(self) -> Option<i64>` (`None` for `TooManyParts`, `Some` for actual raw sentinels/`Other`). Give the new variant a precise `Display` message. SBE/Aeron-pattern rationale: preserve typed transport outcomes at the FFI boundary; a non-wire condition must not be laundered into a made-up raw code.
- What breaks (API only), what it buys: Exhaustive matches need a wildcard, and `raw()` callers must migrate to optional `raw_code()`. Users can distinguish an oversized vectored offer from an unknown Aeron sentinel and never log or branch on a fictitious `-100`.
- Acceptance criteria: Unit tests cover every `AeronOfferError` mapping, exact retryability, `Display`, and raw-code availability; no test or documentation mentions `-100`; update the changelog and public API baseline.
- Verification plan: Run cluster error/publication tests, workspace tests, rustdoc, and the semver/API gate. Run `just bench-cluster`; the change is error-path-only, so all maintained ratios must pass and successful offer/claim assembly must be unchanged.

## T-8: Document the standalone group-encoder framing contract

- Type: DOCS
- Stage: 0.1.xx
- Priority: P1 · Effort: S
- Symptom: Generated group encoders expose `wrap(buf, offset, count)`, `ENTRY_BLOCK_LENGTH`, and `GROUP_DIM_TEMPLATE` without operational documentation (`sbe/src/codegen/group_encoder.rs:279-298`), producing placeholders in the car golden (`sbe/tests/golden/car_example.rs:8237-8251`). The public constructor is intentionally used standalone (`sbe/tests/group_proof_state_test.rs:376-433`), but unlike the decoder's explicit dimension-header contract (`sbe/src/codegen/group_decoder.rs:288-327`), it does not say that its offset is the first entry and that it writes no dimension header.
- Change: Add authored rustdoc in `sbe/src/codegen/group_encoder.rs`: `wrap` is the low-level entries-only constructor; `offset` points immediately after a caller-written dimension header; `count` bounds `add`; and the method neither writes nor back-patches that header. Document both constants and direct normal users to the parent message's `group(...)` / `group_unknown_size(...)` stage. SBE-pattern rationale: a repeating-group wire image is dimension header followed by entries, and the parent typestate API normally owns both that framing and tail order.
- What breaks (API only), what it buys: No API break. A caller reaching for the public low-level constructor can no longer plausibly copy the decoder's opposite offset convention and emit malformed group bytes.
- Acceptance criteria: Generated rustdoc states all four contract points, documents the constants operationally, and names the preferred parent APIs; the car golden contains authored text rather than fallbacks; standalone fixed/dynamic proof tests remain green; the public signature snapshot is unchanged.
- Verification plan: Run generated-doc tests, group proof-state tests, denied-warning rustdoc, `just check-golden`, and generated API checks. Run `just bench` in both LTO modes and require `<= 1.00`; confirm that the generated diff contains documentation only.

## T-9: Correct domain encoded-length rustdoc

- Type: DOCS
- Stage: 0.1.xx
- Priority: P1 · Effort: S
- Symptom: The domain generator documents `encoded_length()` as a body length and then says it matches `encode()` (`sbe/src/codegen/domain_cluster.rs:1127-1145`). For dynamic messages, `encode()` actually returns body plus `HEADER_LENGTH` (`sbe/src/codegen/domain_cluster.rs:1095-1114`); the same contradiction is checked into the car golden (`sbe/tests/golden/car_example.rs:6415-6477`). Tests correctly size buffers with `encoded_length_with_header` (`sbe/tests/domain_objects_test.rs:230-245`), so the documentation, not the implementation, is wrong.
- Change: Document `encoded_length()` as body-only and explicitly say that `encode()` returns `encoded_length_with_header()`. Document `encoded_length_with_header()` as the exact buffer size and exact successful `encode()` return for both fixed and dynamic messages. Add a runnable rustdoc example that allocates from the header-inclusive method. SBE-pattern rationale: SBE deliberately separates message body/block length from framed header-plus-body length; generated names and examples must keep that boundary explicit to prevent under-allocation.
- What breaks (API only), what it buys: No API break. Users get one unambiguous sizing method and no longer risk allocating a body-sized buffer for a framed encode.
- Acceptance criteria: Update the generator and golden text; add a generated-doc assertion for fixed and dynamic domain messages; the example compiles and proves `buf.len() == encode()`; implementation tokens and wire bytes remain unchanged.
- Verification plan: Run generated-doc and domain-object tests, doctests, `just check-golden`, and generated API checks. No executable code should change; prove that by normalized token diff. Under the repository's generator policy, run `just bench` and require both LTO modes `<= 1.00`.

## T-10: Mark decision-valued cluster observers `must_use`

- Type: API
- Stage: 0.1.xx
- Priority: P2 · Effort: S
- Symptom: Cluster compile-fail coverage currently checks returned stateful types only (`cluster/tests/must_use_api.rs:1-30`). Pure decision values remain unannotated: retryability/raw classification (`cluster/src/error.rs:59-105`, `cluster/src/error.rs:304-310`), session IDs/connectivity/state (`cluster/src/client.rs:865-935`), claim position (`cluster/src/client.rs:1050-1060`), and async-connect `step`/`is_complete` (`cluster/src/client.rs:1149-1179`). Discarding these values cannot advance the protocol and commonly means a missing branch.
- Change: Add message-bearing `#[must_use]` to this bounded set of pure observers; do not annotate mutating poll/dispatch/transition methods or values whose discard is intentionally useful. Extend `cluster/tests/must_use_api.rs` with downstream `#![deny(unused_must_use)]` fixtures for retry, readiness, and session-state categories. SBE/Aeron-pattern rationale: protocol-state observers report the decision a consuming loop must make; unlike transitions, calling and discarding them performs no useful work.
- What breaks (API only), what it buys: Code that intentionally discards a newly annotated result under `deny(unused_must_use)` must bind it to `_`; ordinary source remains compatible. Users gain compile-time detection of ignored retryability, readiness, connectivity, and state checks that otherwise produce stuck loops.
- Acceptance criteria: Compile-fail fixtures catch one ignored observer in each named category; positive fixtures show explicit handling; an AST/source audit pins the bounded list and proves mutating APIs remain unannotated; rustdoc exposes actionable messages.
- Verification plan: Run cluster must-use/trybuild, client, and error tests; workspace tests; rustdoc; and public API checks. Run `just bench-cluster`; attributes must not alter maintained benchmark performance.

## T-11: Bind historical benchmark numbers to provenance or remove them

- Type: DOCS
- Stage: 0.1.xx
- Priority: P2 · Effort: S
- Symptom: The benchmark policy says the result of record is a run-stamped artifact and that a result without run ID, commit, host, rustc, target, and manifest hash must not be quoted (`sbe/BENCHMARKS.md:15-32`; `book/src/sbe/benchmarks.md:11-14`). Both pages then retain numeric tables without that provenance (`sbe/BENCHMARKS.md:119-164`; `book/src/sbe/benchmarks.md:52-97`), so a reader cannot tell whether those figures describe the live code or an obsolete cycle.
- Change: For each retained numeric table in those exact sections, locate and link the matching packaged artifact and state its run ID, commit, host, rustc, target, profiles, and manifest hash. If that evidence cannot be located, remove the numbers from current-facing pages and replace them with the mechanism/result conclusion plus the command and artifact location; optionally move the unverified table into a clearly dated historical note. SBE-pattern rationale: parity claims are only meaningful for equal wire work under a specific generated artifact/toolchain; provenance is part of the result, not metadata that can be omitted.
- What breaks (API only), what it buys: No API break. Readers stop treating stale point estimates as current guarantees, and release reviewers can trace every number to reproducible evidence.
- Acceptance criteria: `sbe/BENCHMARKS.md` and `book/src/sbe/benchmarks.md` contain no unprovenanced point estimate; every retained number resolves to an in-repo or release artifact with all required fields; links and book checks pass; the definition of “current result” remains singular and consistent.
- Verification plan: Run `just check-book`, link checks, and `just policy`; inspect `git diff --name-only` to prove no source/benchmark implementation changed. Because this is documentation-only, timing cannot regress and no new benchmark run is warranted; if a fresh run is cited, package it with the existing artifact script and validate both LTO profiles before publishing the numbers.

## Main tickets — 0.1.xx

## T-12: Snapshot complete generated API signatures

- Type: CORRECTNESS
- Stage: 0.1.xx
- Priority: P0 · Effort: M
- Symptom: The new “generated public API” gate is names-only: `public_names` records `struct Foo`, enum variant names, `fn name`, and `Type::method`, but omits fields, payloads, generics, receivers, arguments, return types, safety/constness, where-clauses, and associated items (`sbe/tests/generated_public_api_test.rs:16-83`). It explicitly skips every `#[cfg]` item (`sbe/tests/generated_public_api_test.rs:29-30`, `sbe/tests/generated_public_api_test.rs:36-71`). The snapshots therefore contain entries such as `CarDecoder::decode` with no signature (`api/generated/car_lean.txt:1-50`), while the roadmap claims generated surfaces are enforced (`book/src/project/road-to-1.0.md:50-59`). The passing test only proves a name removal is visible (`sbe/tests/generated_public_api_test.rs:312-324`); a breaking receiver or return-type change passes undetected.
- Change: Replace `public_names` with `public_surface` that canonically serialises the semver-relevant `syn` surface: public struct fields/tuple positions and generics, enum payloads/discriminants and non-exhaustive/repr/cfg attributes, full free/impl/trait method signatures, associated types/consts, type aliases, where-clauses, and visibility. Preserve `#[cfg(...)]` in the snapshot instead of dropping the item. Keep the fixture manifest and offline text snapshots, but rename comments/scripts that claim they are name diffs. SBE-pattern rationale: generated codecs are the consumer's real protocol API; receiver/typestate/field shapes are at least as important as names and must be frozen as one deterministic schema-derived surface.
- What breaks (API only), what it buys: No library API breaks; snapshot format is intentionally replaced and must be reviewed once. The 1.0 gate begins detecting the breaking generated changes it currently claims to detect.
- Acceptance criteria: Add mutation tests showing that changing only (1) a receiver, (2) an argument or return type, (3) a generic/where-clause, (4) a public field or enum payload, and (5) a cfg-gated method each changes/fails the snapshot; private/body/doc-only edits do not. Regenerate all `api/generated/*.txt`, update `api/public-api-baseline.toml:1-11` and `scripts/check-generated-public-api.sh:1-28`, and keep the gate offline and deterministic.
- Verification plan: Run the mutation tests, `scripts/check-generated-public-api.sh`, `just check-public-api`, `just check-golden`, and the full SBE suite twice to prove deterministic output. Snapshot/test tooling does not enter generated runtime code; prove benchmark sources/binaries are unchanged, then run `just bench` in both LTO modes as the release guard and require every maintained ratio `<= 1.00`.

## T-13: Make schema-scoped codegen state re-entrant

- Type: CORRECTNESS
- Stage: 0.1.xx
- Priority: P1 · Effort: M
- Symptom: The sealing trait path lives in a thread-local `RefCell<String>` and is set without restoration; its comment assumes generation is not re-entrant (`sbe/src/codegen/runtime.rs:371-400`). `gen_schema` sets it once (`sbe/src/codegen/mod.rs:1024-1043`), message decoder/encoder generation reads it later (`sbe/src/codegen/message_decoder.rs:150`, `sbe/src/codegen/message_encoder.rs:125`), and public hooks execute between decoder and encoder emission (`sbe/src/codegen/mod.rs:1123-1175`; hook API at `sbe/src/config.rs:886-917`). A temporary review probe used the first message-decoder hook to run a nested generator with `with_external_sbe_rt("crate::nested::sbe_rt")`; the outer source then emitted `crate::nested::__sbe_message_sealed` impls, confirming cross-generation leakage on the current tree.
- Change: Introduce an explicit per-schema `GenerationContext` holding a parsed `sealed_path: syn::Path` and pass `&GenerationContext` into message decoder/encoder generation; delete `SEALED_PATH`, `set_sealed_path`, and `sealed_path_tokens`. Do not solve this with another unscoped global; the already scoped keyword/deprecation helpers may remain. SBE-pattern rationale: runtime ownership and sealing are schema/module identity, so they must travel with that generation pass and cannot be ambient mutable state observable by a re-entrant extension hook.
- What breaks (API only), what it buys: No public API break. Hooks may safely invoke another `Generator`, and output becomes a pure function of the outer schema/config rather than hook call order.
- Acceptance criteria: Add a two-message regression where the first decoder hook performs nested external-runtime generation and assert both outer decoder and encoder impls retain the outer path; add the inverse outer-external/inner-local case; run concurrent/determinism cases; an `rg` check finds no sealing-path thread-local; ordinary goldens remain semantically identical.
- Verification plan: Run hook, external-runtime, multi-schema, generated-runtime API, determinism, golden, and full SBE tests. Compare generated source before/after for non-reentrant fixtures; only mechanical context plumbing may differ. Run `just bench` for LTO on/off and require all maintained ratios `<= 1.00`; run instruction probes on a supported Linux host or explicitly record that this macOS host cannot verify that lane.

## T-14: Stage module-set writes before replacing outputs

- Type: CORRECTNESS
- Stage: 0.1.xx
- Priority: P1 · Effort: M
- Symptom: `generate_multi_to_dir` generates the complete set and then calls `write_module_set` (`sbe/src/build.rs:267-296`), but that function writes destinations sequentially (`sbe/src/build.rs:361-384`). If writing module two fails, module one has already replaced its prior version, leaving a mixed generated graph. The existing “late consumer failure writes nothing” test only covers a parse failure before writing (`sbe/src/build.rs:793-825`), not an I/O failure during the module loop.
- Change: Split `write_module_set` into validate/stage/commit phases. Validate every basename and destination first; write every source to unique sibling temporary files; only after all stages succeed, move existing destinations to backups and promote the complete set. On a promotion failure, restore backups and remove promoted/temp files before returning the path-aware T-3 error. Document that this guarantees rollback for reported in-process failures, not crash-atomicity across power loss. SBE-pattern rationale: shared and consumer modules form one generated protocol graph; publishing a mixture of schema generations violates the same all-or-nothing boundary enforced before code emission.
- What breaks (API only), what it buys: No public signature break. A failed build leaves the last complete generated set usable instead of a mixture that may fail later or, worse, compile with inconsistent schema identity.
- Acceptance criteria: Fault-injection tests fail staging of module two and promotion of module two, assert every pre-existing output remains byte-identical, and assert no temp/backup debris; first generation into an empty directory also rolls back to no outputs; duplicate/path validation happens before any write; warnings print only after commit. Keep the current pre-generation failure test.
- Verification plan: Run focused build-helper fault tests, multi-schema compile tests, workspace tests, denied-warning rustdoc, golden/API checks, and a real sample `build.rs` regeneration. This is build-time-only, but run `just bench` in both LTO modes and require `<= 1.00`; compare generated module contents to prove successful output is byte-identical.

## Performance evidence boundary

No standalone PERF ticket is included. The repository itself says that only a provenance-stamped run artifact is a current result (`sbe/BENCHMARKS.md:15-32`), and this review did not run the full maintained LTO matrix or produce such an artifact. Static mechanisms without measured evidence would violate the PERF-ticket rule. The executable changes above therefore carry explicit benchmark gates in their verification plans; do not promote a plausible optimisation into a ticket until a fresh run identifies a regressing arm and the mechanism is isolated with equal-work, wire-parity evidence.

## 1.0-only tickets

## T-100: Derive schema identity from one `Ir`

- Type: API
- Stage: 1.0
- Priority: P1 · Effort: M
- Symptom: Public `Schema` duplicates `package`, `id`, and `version` beside a public `ir` containing the same fields (`sbe/src/schema.rs:27-40`), and `from_ir` merely clones them once (`sbe/src/schema.rs:67-88`). Callers can mutate either copy independently. Codegen uses `schema.ir` for codec headers and SHA-256 (`sbe/src/codegen/mod.rs:983`, `sbe/src/codegen/mod.rs:1241-1251`) but the outer generated comment and `SCHEMA_HASH` use the duplicate fields (`sbe/src/codegen/mod.rs:991-995`, `sbe/src/codegen/mod.rs:1239-1240`), so one public value can describe two schema identities.
- Change: Store only private `ir: Ir` in `Schema`; add `package() -> &str`, `id() -> u16`, `version() -> u16`, `ir() -> &Ir`, `ir_mut() -> &mut Ir`, and `into_ir() -> Ir`; migrate codegen and docs to those accessors. `ir_mut` preserves advanced/manual IR workflows while keeping one identity source. SBE-pattern rationale: schema ID/version/package are wire identity and must have exactly one source of truth; a divergent identity state should be unrepresentable.
- What breaks (API only), what it buys: Direct field reads/writes (`schema.id`, `schema.ir`) migrate to methods, and struct literals stop compiling. Users can no longer accidentally emit headers, hashes, and provenance that disagree about the schema.
- Acceptance criteria: Add compile-fail migration fixtures for old field access and a positive `ir_mut` test showing every accessor/codegen constant follows the mutation; remove all duplicate identity storage; update public docs/examples/changelog and 1.0 migration guide; golden wire bytes remain unchanged for normal parsed schemas.
- Verification plan: Run schema/codegen/hash/header tests, goldens, generated/public API gates, doctests, and the full workspace suite. Run `just bench` in both LTO modes and require `<= 1.00`; verify ordinary generated bodies and wire fixtures are byte-identical.

## T-101: Preserve the exact SBE deprecation version

- Type: API
- Stage: 1.0
- Priority: P2 · Effort: M
- Symptom: XML `deprecated` is a non-negative schema version, but `parse_deprecated_attr` discards the parsed `u16` and returns `bool` (`sbe/src/xml/attr.rs:233-241`). The loss propagates through public `Encoding::deprecated: bool` (`sbe/src/ir.rs:141-175`), structured message/field metadata (`sbe/src/structured_ir.rs:130-164`), and public hook `FieldInfo::deprecated: bool` (`sbe/src/config.rs:202-224`). Migration hooks can tell that an item is deprecated but not when it became deprecated.
- Change: Represent deprecation as `Option<u16>` throughout XML, IR, structured IR, and hook metadata; combine inherited type/field deprecations by the earliest applicable version; keep `with_deprecated_attrs` as the emission switch and emit `#[deprecated(note = "SBE schema deprecated since version N")]`. Rename hook metadata to `deprecated_since` where ambiguity would remain. SBE-pattern rationale: SBE evolution is version-indexed; collapsing a version to a flag destroys information needed for acting-version compatibility and migrations.
- What breaks (API only), what it buys: Public `Encoding::deprecated` and `FieldInfo::deprecated` users migrate from `bool` to `Option<u16>`/`deprecated_since`. Schema tooling and hooks gain the exact version needed to generate migration warnings and reports.
- Acceptance criteria: Parser tests preserve `0`, ordinary versions, inherited deprecation, direct-vs-inherited minimum, and invalid/overflow input; hook tests expose the exact version for types/messages/fields/groups/data; generated warnings include the version; update docs, changelog, migration guide, and API snapshots.
- Verification plan: Run XML/IR/structured-IR, deprecated-attribute, hook, golden, generated API, doctest, and workspace suites. Run `just bench` for both LTO profiles and require `<= 1.00`; prove no wire layout or non-attribute generated body changes.

## T-102: Remove lossy generated error conversions

- Type: API
- Stage: 1.0
- Priority: P1 · Effort: S
- Symptom: `GenerationConfig::with_error_from_impls` is already deprecated for 1.0 removal because it formats typed encode/decode errors through `String`, losing fields such as `needed` and `available` (`sbe/src/config.rs:716-737`; `CHANGELOG.md:58-62`). Codegen still emits those lossy `From` impls when configured (`sbe/src/codegen/mod.rs:1261-1275`).
- Change: Delete `error_from_path`, `with_error_from_impls`, its path validation, and generated `From<String>`-based impl emission. Keep/document the direct user implementation of `From<generated::sbe_rt::{EncodeError, DecodeError}>` as the only supported conversion. SBE-pattern rationale: codec errors are structured evidence about a failed wire boundary; flattening them into display text defeats typed recovery and diagnostics.
- What breaks (API only), what it buys: Builds using the deprecated helper must add explicit typed `From` impls. Error variants, fields, and sources remain inspectable instead of being irreversibly flattened.
- Acceptance criteria: The deprecated method/config field/emission path is absent; a migration compile fixture shows direct typed conversions with `?`; docs, changelog, and 1.0 migration guide contain the replacement; generated API/goldens no longer contain the lossy impls.
- Verification plan: Run configuration, conversion, generated-source compile, golden/API, doctest, and workspace suites. Run `just bench` in both LTO modes and require `<= 1.00`; confirm codec hot paths and wire bytes are unchanged.
