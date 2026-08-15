//! Composite / fixed-field layout contracts.
//!
//! Documents and locks the design for little-endian schemas:
//!
//! 1. Generated composites are **wire images**, not `#[repr(C)]` native-field
//!    overlays: `#[repr(transparent)] pub struct Engine(pub [u8; N])`.
//! 2. Member accessors use explicit `from_le_bytes` / `to_le_bytes` at schema
//!    offsets (portable; on LE hosts LLVM lowers these to plain loads).
//! 3. Default decoder path is a **flyweight** (`EngineDecoder { buf, offset }`)
//!    — zero-copy into the message buffer. `pos` is the byte offset of the
//!    composite body within `buf`.
//!    body address once at construction so every member accessor is a single
//!    struct load + immediate-offset wire load. `engine_value()` is an eager
//!    copy of the `N`-byte wire image.
//! 4. We intentionally do **not** transmute the buffer to a padded `repr(C)`
//!    struct: SBE is packed (no Rust alignment padding), fields may be
//!    unaligned, and big-endian schemas must keep working.
//!
//! See README "Composite layout & little-endian".

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, generate};

/// Source shape: transparent `[u8; N]`, LE accessors, size_of assert, flyweight.
#[test]
fn composite_is_transparent_wire_image_not_repr_c_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let (_schema, src) = generate(&Paths::example_schema(), "comp_layout_src");

    assert!(
        src.contains("#[repr(transparent)]") && src.contains("pub struct Engine(pub [u8; 10])"),
        "Engine must be a transparent wire-image over [u8; 10], not a native-field struct"
    );
    assert!(
        src.contains("u16::from_le_bytes") && src.contains("to_le_bytes"),
        "LE composite accessors must use explicit from_le_bytes / to_le_bytes"
    );
    assert!(
        src.contains("const _: () = assert!(core::mem::size_of")
            && src.contains("Engine")
            && src.contains("== 10"),
        "compile-time size_of::<Engine>() == wire size must be present"
    );
    // Must NOT invent a native-layout overlay type.
    assert!(
        !src.contains("#[repr(C)]\npub struct Engine {") && !src.contains("#[repr(C, packed)]"),
        "must not generate #[repr(C)] / packed native-field Engine overlays"
    );
    assert!(
        src.contains("pub struct EngineDecoder")
            && src.contains("buf: &'a [u8]")
            && src.contains("offset: usize"),
        "flyweight EngineDecoder {{ buf, offset }} must be generated"
    );
    Ok(())
}

/// Composite flyweights are private products of a checked/trusted message
/// decoder, so their fixed-width member loads must not re-check slice bounds.
#[test]
fn composite_flyweight_accessors_use_trusted_reads() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "comp_layout_trusted_reads");
    let start = src
        .find("impl<'a> EngineDecoder<'a>")
        .ok_or_else(|| std::io::Error::other("missing EngineDecoder impl"))?;
    let decoder_and_rest = &src[start..];
    let end = decoder_and_rest
        .find("pub struct CarDecoder")
        .ok_or_else(|| std::io::Error::other("missing CarDecoder after EngineDecoder"))?;
    let engine_decoder = &decoder_and_rest[..end];

    assert!(
        engine_decoder.contains("read_bytes_unchecked"),
        "fixed-width EngineDecoder accessors must use trusted reads"
    );
    Ok(())
}

/// Runtime: `.0` is the exact LE wire image; accessors match hand-decoded LE.
#[test]
fn composite_value_bytes_are_exact_le_wire_image() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "comp_layout_wire");
    compile_and_run(
        "comp_layout_wire",
        &src,
        r#"
        // Hand-built LE wire image for Engine (10 bytes):
        // capacity=2000u16 LE, cylinders=4, mfr="123", efficiency=35i8,
        // boosterEnabled=T(1), booster=NITROUS@200 → 'N', 200
        let wire: [u8; 10] = [
            0xd0, 0x07, // 2000 LE
            0x04,       // 4 cylinders
            b'1', b'2', b'3',
            35,         // efficiency
            1,          // BooleanType::T
            b'N', 200,  // BoostType::NITROUS + horsePower
        ];

        // Constructing via the typed ctor must produce that exact image.
        let eng = Engine::new(
            2000,
            4,
            [b'1', b'2', b'3'],
            35,
            BooleanType::T,
            Booster::new(BoostType::NITROUS, 200),
        );
        assert_eq!(eng.0, wire, "Engine.0 must be the raw LE wire image");
        assert_eq!(core::mem::size_of_val(&eng), 10);
        assert_eq!(core::mem::size_of::<Engine>(), 10);

        // Accessors must match hand-decoding the same bytes with from_le_bytes.
        assert_eq!(eng.capacity(), u16::from_le_bytes([wire[0], wire[1]]));
        assert_eq!(eng.num_cylinders(), wire[2]);
        assert_eq!(eng.manufacturer_code(), [wire[3], wire[4], wire[5]]);
        assert_eq!(eng.efficiency(), wire[6] as i8);
        assert_eq!(eng.booster().horse_power(), wire[9]);

        // Wrapping the raw image (as encoder write / decoder copy does) is a
        // pure reinterpret of those bytes — no field-by-field re-pack.
        let from_bytes = Engine(wire);
        assert_eq!(from_bytes.capacity(), 2000);
        assert_eq!(from_bytes.num_cylinders(), 4);
        assert_eq!(from_bytes.0, eng.0);
    "#,
    );
    Ok(())
}

/// Flyweight is zero-copy into the message buffer; value is an N-byte copy.
#[test]
fn composite_flyweight_is_zero_copy_value_is_eager_wire_copy()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "comp_layout_fw");
    compile_and_run(
        "comp_layout_fw",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let eng = Engine::new(
            2000,
            4,
            [b'1', b'2', b'3'],
            35,
            BooleanType::T,
            Booster::new(BoostType::NITROUS, 200),
        );
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: eng,
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let complete = car.activation_code(b"abc").unwrap();
        let encoded = complete.as_bytes_with_header();

        let dec = CarDecoder::try_decode(encoded, 0).unwrap();

        // Default accessor: flyweight — reads live from the message buffer.
        let fly = dec.engine();
        assert_eq!(fly.capacity(), 2000);
        assert_eq!(fly.num_cylinders(), 4);
        assert_eq!(fly.efficiency(), 35);

        // Value accessor: eager copy of the 10-byte wire image.
        let owned = dec.engine_value();
        assert_eq!(owned.0.len(), 10);
        assert_eq!(owned.capacity(), fly.capacity());
        assert_eq!(owned.num_cylinders(), fly.num_cylinders());
        assert_eq!(owned.efficiency(), fly.efficiency());
        assert_eq!(owned.booster().horse_power(), fly.booster().horse_power());

        // The owned image must match the bytes sitting at the engine offset
        // in the encoded frame (header 8 + body engine offset 35 = 43).
        const ENGINE_ABS: usize = 8 + 35;
        assert_eq!(&owned.0[..], &encoded[ENGINE_ABS..ENGINE_ABS + 10]);
    "#,
    );
    Ok(())
}

/// Encoding a composite is a bulk write of its wire image (not field-by-field
/// through a native struct overlay).
#[test]
fn composite_encoder_writes_wire_image_bulk() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "comp_layout_enc");
    compile_and_run(
        "comp_layout_enc",
        &src,
        r#"
        let eng = Engine::new(
            0xABCD,
            7,
            [b'X', b'Y', b'Z'],
            -3i8,
            BooleanType::F,
            Booster::new(BoostType::TURBO, 99),
        );
        let mut buf = [0u8; 256];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: eng,
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let complete = car.activation_code(b"").unwrap();
        let encoded = complete.as_bytes_with_header();

        const ENGINE_ABS: usize = 8 + 35;
        assert_eq!(
            &encoded[ENGINE_ABS..ENGINE_ABS + 10],
            &eng.0[..],
            "encoder must copy Engine.0 bulk into the frame"
        );
        // Spot-check LE capacity bytes in-place.
        assert_eq!(
            u16::from_le_bytes(encoded[ENGINE_ABS..ENGINE_ABS + 2].try_into().unwrap()),
            0xABCD
        );
    "#,
    );
    Ok(())
}

/// Fixed message scalars also use explicit LE loads from the buffer — same
/// model as composite members (not a `repr(C)` view of the whole fixed block).
#[test]
fn fixed_scalar_fields_use_explicit_le_loads() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "comp_layout_scalar");
    assert!(
        src.contains("from_le_bytes"),
        "message fixed-field readers must use from_le_bytes"
    );
    compile_and_run(
        "comp_layout_scalar",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 0x0102_0304_0506_0708,
                model_year: 0xBEEF,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(
                    1, 1, [0, 0, 0], 0, BooleanType::F, Booster::new(BoostType::TURBO, 0),
                ),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let complete = car.activation_code(b"").unwrap();
        let encoded = complete.as_bytes_with_header();

        // Body starts after 8-byte LE header.
        let body = &encoded[8..];
        let serial_le = u64::from_le_bytes(body[0..8].try_into().unwrap());
        let year_le = u16::from_le_bytes(body[8..10].try_into().unwrap());

        let dec = CarDecoder::try_decode(encoded, 0).unwrap();
        assert_eq!(dec.serial_number(), serial_le);
        assert_eq!(dec.model_year(), year_le);
        assert_eq!(dec.serial_number(), 0x0102_0304_0506_0708);
        assert_eq!(dec.model_year(), 0xBEEF);
    "#,
    );
    Ok(())
}
