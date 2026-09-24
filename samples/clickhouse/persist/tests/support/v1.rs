//! The `shapes_v1` test schema, its codecs, and one message using every field
//! shape. Shared by the integration tests and `examples/record_latency.rs`.

#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
pub mod codec {
    include!(concat!(env!("OUT_DIR"), "/shapes_v1.rs"));
}

use codec::{Colour, EntriesEntry, ShapesEncoder, ShapesFixedFields, sbe_rt::EncodeError};

pub const SCHEMA: &str = include_str!("../schemas/shapes_v1.xml");
pub const TEMPLATE_ID: u16 = ShapesEncoder::TEMPLATE_ID;
const NOTE: &[u8] = b"hello";
/// Exact length of the message [`encode`] writes.
pub const LEN: usize = ShapesEncoder::compute_length_with_header(2, NOTE.len());

/// Encode the message: every field shape, two group entries, `note = hello`.
pub fn encode(buf: &mut [u8]) -> Result<usize, EncodeError> {
    Ok(ShapesEncoder::wrap_and_apply_header(buf, 0)
        .fixed(&ShapesFixedFields {
            ts: 1_700_000_000_123_456_789,
            i8: -8,
            i16: -16,
            i32: -32,
            i64: -64,
            u8: 8,
            u16: 16,
            u32: 32,
            u64: u64::MAX,
            f32: 1.5,
            f64: 2.25,
            opt_i32: Some(-7),
            opt_u64: None,
            opt_f64: Some(3.5),
            colour: Colour::Green,
            code: *b"ABC\0\0\0",
        })
        .entries(2, |e| {
            e.add_struct(&EntriesEntry {
                qty: 10,
                side: Colour::Red,
                maybe: f64::NAN,
            })?;
            e.add_struct(&EntriesEntry {
                qty: -20,
                side: Colour::Green,
                maybe: 0.5,
            })?;
            Ok(())
        })?
        .note(NOTE)?
        .encoded_length_with_header())
}
