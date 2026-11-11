//! Group entries write the schema null image for every optional field they
//! are not given, at message, group-entry and nested-group-entry level.
//!
//! The image must match the message-level `fixed()` path byte for byte:
//! optional scalars and enums take their null value, an optional array takes
//! the element null in every slot, and an optional composite is zeroed and
//! then has its optional members nulled (recursively, at the member offset).
//! A reused buffer full of garbage must encode exactly as a zeroed one.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(clippy::nursery)]

mod common;
use common::{Paths, compile_and_run};
use ergo_sbe::{GenerationConfig, Generator, Schema, SchemaSource, parse};
use std::error::Error;
use std::path::PathBuf;

/// `group-null-init-{le,be}-schema.xml`: the same optional field set (every
/// shape) at message, group-entry and nested-group-entry level. The
/// sbe-tool wire-parity test reads the same files.
fn schema_path(big_endian: bool) -> PathBuf {
    Paths::sbe_tool_test_resource(if big_endian {
        "group-null-init-be-schema.xml"
    } else {
        "group-null-init-le-schema.xml"
    })
}

/// The expected fixed block for one location, `req` set and every optional
/// left unset, in the schema byte order.
fn expected_block(big_endian: bool, with_late: bool) -> Vec<u8> {
    let u32b = |v: u32| {
        if big_endian {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    };
    let i32b = |v: i32| {
        if big_endian {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    };
    let mut out = Vec::new();
    out.extend(u32b(0x1122_3344)); // req
    out.extend(i32b(i32::MIN)); // optScalar
    out.push(255); // optEnum: uint8 encoding null
    out.extend([0xFF; 4]); // optSmall: element null in every slot
    out.extend([0xFF; 4]); // optPair: two uint16 nulls
    for _ in 0..4 {
        out.extend(i32b(i32::MIN)); // optWide
    }
    out.extend([0u8; 12]); // optText: char null
    out.extend(u32b(u32::MAX)); // optComp.lo (optional member)
    out.extend(u32b(0)); // optComp.hi (required member, zeroed)
    out.extend(u32b(u32::MAX)); // optNest.inner.lo
    out.extend(u32b(0)); // optNest.inner.hi
    out.push(7); // optNest.tag, declared nullValue
    if with_late {
        out.extend([0xFF; 4]); // optLate
    }
    out
}

fn generate(
    big_endian: bool,
    module: &str,
    encode_version: Option<u16>,
) -> Result<String, Box<dyn Error>> {
    let schema = Schema::from_ir(parse(SchemaSource::file(schema_path(big_endian)))?);
    let mut config = GenerationConfig::new(module);
    if let Some(v) = encode_version {
        config = config.with_encode_version(v);
    }
    let modules = Generator::new(config).generate(&schema)?;
    Ok(modules
        .modules()
        .next()
        .ok_or("expected generated module")?
        .source
        .clone())
}

fn bytes_lit(bytes: &[u8]) -> String {
    let items: Vec<String> = bytes.iter().map(u8::to_string).collect();
    format!("[{}u8]", items.join("u8, "))
}

/// How the flat (fixed-stride) nested entry is written: each entry path must
/// null the entry before the caller's setters run.
const CELL_ADD: &str = "g.add(|e| { e.req(0x1122_3344); Ok(()) })";
const CELL_ADD_CHECKED: &str = "g.add_checked(|mut e| { e.req(0x1122_3344); Ok(e.complete()) })";
const CELL_START_ENTRY: &str = "{ let mut e = g.start_entry()?; e.req(0x1122_3344); Ok(()) }";

/// Encode one message with one row holding one cell, setting only `req`,
/// over a garbage-filled and a zeroed buffer; both must match the expected
/// image at all three locations.
fn run(
    big_endian: bool,
    encode_version: Option<u16>,
    cell_write: &str,
    tag: &str,
) -> Result<(), Box<dyn Error>> {
    let with_late = encode_version.is_none();
    let block = expected_block(big_endian, with_late);
    let late = if with_late { "opt_late: None," } else { "" };
    let module = format!("group_null_{}_{tag}", if big_endian { "be" } else { "le" });
    let src = generate(big_endian, &module, encode_version)?;
    let body = format!(
        r#"
        const EXPECTED: [u8; {n}] = {expected};
        const L: usize = {n};
        fn encode(buf: &mut [u8]) -> usize {{
            MsgEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&MsgFixedFields {{
                    req: 0x1122_3344,
                    opt_scalar: None,
                    opt_enum: None,
                    opt_small: None,
                    opt_pair: None,
                    opt_wide: None,
                    opt_text: None,
                    opt_comp: None,
                    opt_nest: None,
                    {late}
                }})
                .rows(1, |g| {{
                    g.add(|mut e| {{
                        e.req(0x1122_3344);
                        e.cells(1, |g| {cell_write})
                    }})
                }})
                .expect("encode")
                .encoded_length_with_header()
        }}
        let mut dirty = [0xABu8; 512];
        let mut clean = [0u8; 512];
        let len = encode(&mut dirty);
        assert_eq!(len, encode(&mut clean));
        assert_eq!(len, 8 + 3 * L + 8, "header + three blocks + two dimensions");
        assert_eq!(&dirty[..len], &clean[..len], "garbage leaked into the frame");
        assert_eq!(&dirty[8..8 + L], &EXPECTED[..], "message block");
        assert_eq!(&dirty[12 + L..12 + 2 * L], &EXPECTED[..], "group entry");
        assert_eq!(&dirty[16 + 2 * L..16 + 3 * L], &EXPECTED[..], "nested group entry");

        // The null writes allocate nothing: re-encode over garbage under a
        // counting allocator (the calls above were the warm-up).
        use std::alloc::{{GlobalAlloc, Layout, System}};
        use std::sync::atomic::{{AtomicUsize, Ordering}};
        static ALLOCS: AtomicUsize = AtomicUsize::new(0);
        struct Counting;
        unsafe impl GlobalAlloc for Counting {{
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {{
                ALLOCS.fetch_add(1, Ordering::Relaxed);
                unsafe {{ System.alloc(layout) }}
            }}
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {{
                unsafe {{ System.dealloc(ptr, layout) }}
            }}
        }}
        #[global_allocator]
        static GLOBAL: Counting = Counting;
        dirty.fill(0xAB);
        let before = ALLOCS.load(Ordering::Relaxed);
        let again = encode(std::hint::black_box(&mut dirty));
        let allocs = ALLOCS.load(Ordering::Relaxed) - before;
        assert_eq!(allocs, 0, "group null init allocated");
        assert_eq!(&dirty[..again], &clean[..len]);

        // ergon's own decoder reads every unset optional back as null.
        macro_rules! assert_nulls {{
            ($d:expr) => {{{{
                let d = &$d;
                assert_eq!(d.req(), 0x1122_3344);
                assert_eq!(d.opt_scalar(), None);
                assert_eq!(d.opt_enum(), Side::NullVal);
                assert_eq!(d.opt_small(), [u8::MAX; 4]);
                assert_eq!(d.opt_pair(), [u16::MAX; 2]);
                assert_eq!(d.opt_wide(), [i32::MIN; 4]);
                assert_eq!(d.opt_text(), [0u8; 12]);
                assert_eq!((d.opt_comp().lo(), d.opt_comp().hi()), (None, 0));
                assert_eq!(d.opt_nest().tag(), None);
                assert_eq!((d.opt_nest().inner().lo(), d.opt_nest().inner().hi()), (None, 0));
                {late_check}
            }}}};
        }}
        let msg = MsgDecoder::try_from(&dirty[..len]).expect("decode");
        assert_nulls!(msg);
        let mut rows = 0;
        msg.into_rows(|row| -> Result<_, sbe_rt::DecodeError> {{
            rows += 1;
            assert_nulls!(row);
            let mut cells = row.into_cells()?;
            let cell = (&mut cells).next().expect("one cell");
            assert_nulls!(cell);
            cells.finish()
        }})
        .expect("decode rows");
        assert_eq!(rows, 1);
        "#,
        late_check = if with_late {
            "assert_eq!(d.opt_late(), [u16::MAX; 2]);"
        } else {
            ""
        },
        n = block.len(),
        expected = bytes_lit(&block),
    );
    compile_and_run(&module, &src, &body);
    Ok(())
}

#[test]
fn group_entries_null_every_optional_shape_little_endian() -> Result<(), Box<dyn Error>> {
    run(false, None, CELL_ADD, "add")
}

#[test]
fn group_entries_null_every_optional_shape_big_endian() -> Result<(), Box<dyn Error>> {
    run(true, None, CELL_ADD_CHECKED, "add_checked")
}

/// Projected to an older acting version, the dropped `sinceVersion` field is
/// neither written nor counted in the entry block.
#[test]
fn group_entries_null_at_projected_encode_version() -> Result<(), Box<dyn Error>> {
    run(false, Some(0), CELL_ADD, "v0")
}

/// `start_entry()` hands out a borrowed entry encoder; it nulls the entry too.
#[test]
fn start_entry_nulls_the_entry() -> Result<(), Box<dyn Error>> {
    run(false, None, CELL_START_ENTRY, "start_entry")
}
