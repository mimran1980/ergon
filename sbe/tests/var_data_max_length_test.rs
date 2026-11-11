//! Checked decoders reject a var-data length above the schema `maxValue` even
//! when the buffer holds every byte of it, at message, group-entry and
//! nested-entry level, and accept a length equal to it.
//!
//! Every read rejects it for the field it returns, even when the buffer is
//! short. The whole-message paths (`verify`, `encoded_length`, staged reads
//! and skips, `skip_n`, `ordered`, the memoized lane) reject it wherever it
//! is. A random-access or group-entry read that only passes over an earlier
//! field to locate a later one, and group iteration, check bounds only: the
//! later read succeeds and a short buffer reports `BufferTooShort`.

#![allow(clippy::all, clippy::pedantic, clippy::restriction)]
use std::error::Error;

mod common;
use common::compile_and_run;

const XML: &str = r#"<messageSchema package="vdmax" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="groupSizeEncoding">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="numInGroup" primitiveType="uint16"/>
    </composite>
    <composite name="data8">
      <type name="length" primitiveType="uint8" maxValue="254"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
    <composite name="data16">
      <type name="length" primitiveType="uint16" maxValue="300"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
    <composite name="data32">
      <type name="length" primitiveType="uint32" maxValue="1024"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
  </types>
  <message name="M" id="1">
    <group name="rows" id="1" dimensionType="groupSizeEncoding">
      <group name="items" id="2" dimensionType="groupSizeEncoding">
        <data name="c" id="3" type="data32"/>
      </group>
      <data name="b" id="4" type="data16"/>
    </group>
    <data name="a" id="5" type="data8"/>
  </message>
  <message name="N" id="2">
    <group name="pairs" id="6" dimensionType="groupSizeEncoding">
      <data name="p" id="7" type="data8"/>
      <data name="q" id="8" type="data16"/>
    </group>
    <data name="x" id="9" type="data8"/>
    <data name="y" id="10" type="data16"/>
  </message>
</messageSchema>"#;

#[test]
fn var_data_above_max_value_is_rejected_on_every_lane() -> Result<(), Box<dyn Error>> {
    use ergo_sbe::{GenerationConfig, Generator, Schema, SchemaSource, parse};
    let schema = Schema::from_ir(parse(SchemaSource::xml(XML))?);
    let src = Generator::new(GenerationConfig::new("vdmax"))
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("one module")?
        .source
        .clone();
    compile_and_run(
        "vdmax",
        &src,
        r#"
        // One row with one item; each length prefix is written as given and
        // followed by exactly that many bytes, so every frame is in bounds.
        fn frame(a: usize, b: usize, c: usize) -> Vec<u8> {
            let mut w = vec![0u8, 0, 1, 0, 1, 0, 0, 0];
            w.extend_from_slice(&[0, 0, 1, 0]);
            w.extend_from_slice(&[0, 0, 1, 0]);
            w.extend_from_slice(&(c as u32).to_le_bytes());
            w.resize(w.len() + c, b'c');
            w.extend_from_slice(&(b as u16).to_le_bytes());
            w.resize(w.len() + b, b'b');
            w.push(a as u8);
            w.resize(w.len() + a, b'a');
            w
        }

        fn random(w: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
            let dec = MDecoder::try_decode(w, 0)?;
            let mut n = 0;
            for row in dec.rows()? {
                let row = row?;
                for item in row.items()? {
                    n += item?.c()?.len();
                }
                n += row.b()?.len();
            }
            Ok(n + dec.a()?.len())
        }

        fn staged(w: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
            let mut n = 0;
            let after = MDecoder::try_decode(w, 0)?.into_rows(|row| {
                let row = row.into_items(|item| {
                    let (c, done) = item.into_c()?;
                    n += c.len();
                    Ok(done)
                })?;
                n += row.b_slice()?.len();
                let (b, done) = row.into_b()?;
                n += b.len();
                Ok(done)
            })?;
            n += after.a_slice()?.len();
            let (a, _) = after.into_a()?;
            Ok(n + a.len())
        }

        fn ordered(w: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
            let mut n = 0;
            MDecoder::try_decode(w, 0)?
                .ordered()
                .rows(|row, _| {
                    row.ordered()
                        .items(|item, _| item.ordered().c(|c| { n += c.len(); Ok(()) }))?
                        .b(|b| { n += b.len(); Ok(()) })
                })?
                .a(|a| { n += a.len(); Ok(()) })?;
            Ok(n)
        }

        // Lanes that pass over earlier tails without reading them: each must
        // still reject an earlier length above its maximum.
        fn encoded_length(w: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
            MDecoder::try_decode(w, 0)?.encoded_length()
        }

        fn random_a_alone(w: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
            Ok(MDecoder::try_decode(w, 0)?.a()?.len())
        }

        fn skip_n_then_a(w: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
            let dec = MDecoder::try_decode(w, 0)?;
            dec.rows()?.skip_n(1)?;
            Ok(dec.a()?.len())
        }

        fn skip_rows_then_a(w: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
            let (a, _) = MDecoder::try_decode(w, 0)?.skip_rows()?.into_a()?;
            Ok(a.len())
        }

        fn skip_items_then_b(w: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
            let mut n = 0;
            let after = MDecoder::try_decode(w, 0)?.into_rows(|row| {
                let (b, done) = row.skip_items()?.into_b()?;
                n += b.len();
                Ok(done)
            })?;
            let (a, _) = after.into_a()?;
            Ok(n + a.len())
        }

        fn memoized_a_alone(w: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
            Ok(MDecoder::try_decode(w, 0)?.memoized().a()?.len())
        }

        fn memoized_encoded_length(w: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
            MDecoder::try_decode(w, 0)?.memoized().encoded_length()
        }

        // The length prefix of one field is its type's maximum and the
        // buffer ends right after it: the schema bound is reported first,
        // the same way on every lane.
        fn short(field: &str) -> Vec<u8> {
            let mut w = frame(0, 0, 0);
            match field {
                "c" => {
                    w.truncate(20);
                    w[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
                }
                "b" => {
                    w.truncate(22);
                    w[20..22].copy_from_slice(&u16::MAX.to_le_bytes());
                }
                _ => w[22] = u8::MAX,
            }
            w
        }

        // What a lane must report for one over-max field: the schema bound,
        // `BufferTooShort` for that field (a bounds-only pass over a huge
        // length), or success (a bounds-only pass over an in-bounds one).
        #[derive(Clone, Copy, Debug, PartialEq)]
        enum Want { Max, Short, Pass }
        use Want::{Max, Pass, Short};
        const STRICT: [Want; 6] = [Max; 6];

        // Every lane yields `VerifyError`, which wraps a `DecodeError`, so
        // `verify` sits in the same table as the decode lanes. Expectations
        // follow the case order below: a, b, c in bounds, then a, b, c short.
        let lanes: [(&str, fn(&[u8]) -> Result<usize, sbe_rt::VerifyError>, [Want; 6]); 11] = [
            // Iteration passes over b and c bounds-only; their getters reject.
            ("random", |w| Ok(random(w)?), [Max, Max, Max, Max, Short, Short]),
            ("staged", |w| Ok(staged(w)?), STRICT),
            ("ordered", |w| Ok(ordered(w)?), STRICT),
            ("verify", |w| MDecoder::verify(w).map(|()| 0), STRICT),
            ("encoded_length", |w| Ok(encoded_length(w)?), STRICT),
            ("random a() alone", |w| Ok(random_a_alone(w)?), [Max, Pass, Pass, Max, Short, Short]),
            ("skip_n then a()", |w| Ok(skip_n_then_a(w)?), STRICT),
            ("skip_rows then into_a", |w| Ok(skip_rows_then_a(w)?), STRICT),
            ("skip_items then into_b", |w| Ok(skip_items_then_b(w)?), STRICT),
            ("memoized a() alone", |w| Ok(memoized_a_alone(w)?), STRICT),
            ("memoized encoded_length", |w| Ok(memoized_encoded_length(w)?), STRICT),
        ];
        let mut failures = Vec::new();
        for (lane, decode, wants) in lanes {
            // Exactly maxValue at every level decodes.
            assert!(decode(&frame(254, 300, 1024)).is_ok(), "{lane}: maxValue must decode");
            // maxValue + 1 at one level, with all of its bytes in the buffer,
            // then the length type's maximum with the buffer ending after it.
            let cases = [
                (frame(255, 300, 1024), "a", 255_u64, 254_u64),
                (frame(254, 301, 1024), "b", 301, 300),
                (frame(254, 300, 1025), "c", 1025, 1024),
                (short("a"), "a", u64::from(u8::MAX), 254),
                (short("b"), "b", u64::from(u16::MAX), 300),
                (short("c"), "c", u64::from(u32::MAX), 1024),
            ];
            for ((w, field, length, max), want) in cases.into_iter().zip(wants) {
                let got = decode(&w);
                let ok = match (want, &got) {
                    (Max, Err(sbe_rt::VerifyError::DecodeError(sbe_rt::DecodeError::InvalidVarDataLength {
                        field: f,
                        length: l,
                        max_length: m,
                    }))) => *f == field && *l == length && *m == max,
                    (Short, Err(sbe_rt::VerifyError::DecodeError(sbe_rt::DecodeError::BufferTooShort {
                        field: f, ..
                    }))) => *f == field,
                    (Pass, Ok(_)) => true,
                    _ => false,
                };
                if !ok {
                    failures.push(format!(
                        "{lane}: {field} length {length} over max {max} wanted {want:?}, gave {got:?}"
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));

        // N: two var-data fields per owner, so a read can pass over an earlier
        // over-max field at the same level. Entry: p, q; message: x, y.
        fn nframe(p: usize, q: usize, x: usize, y: usize) -> Vec<u8> {
            let mut w = vec![0u8, 0, 2, 0, 1, 0, 0, 0];
            w.extend_from_slice(&[0, 0, 1, 0]);
            w.push(p as u8);
            w.resize(w.len() + p, b'p');
            w.extend_from_slice(&(q as u16).to_le_bytes());
            w.resize(w.len() + q, b'q');
            w.push(x as u8);
            w.resize(w.len() + x, b'x');
            w.extend_from_slice(&(y as u16).to_le_bytes());
            w.resize(w.len() + y, b'y');
            w
        }
        fn over(r: Result<impl core::fmt::Debug, sbe_rt::DecodeError>, field: &str, length: u64) -> bool {
            matches!(r, Err(sbe_rt::DecodeError::InvalidVarDataLength { field: f, length: l, .. })
                if f == field && l == length)
        }
        fn verify_over(w: &[u8], field: &str, length: u64) -> bool {
            matches!(NDecoder::verify(w), Err(sbe_rt::VerifyError::DecodeError(
                sbe_rt::DecodeError::InvalidVarDataLength { field: f, length: l, .. }
            )) if f == field && l == length)
        }

        // Message level: y passes over x; x itself and the whole-message
        // paths reject it. The memoized lane checks every field it walks.
        let w = nframe(254, 300, 255, 300);
        let dec = NDecoder::try_decode(&w, 0)?;
        assert_eq!(dec.y()?.len(), 300, "y passes over over-max x bounds-only");
        assert!(over(dec.x(), "x", 255), "x() rejects its own length");
        assert!(over(dec.encoded_length(), "x", 255), "encoded_length rejects x");
        assert!(over(dec.as_body_bytes(), "x", 255), "as_body_bytes rejects x");
        assert!(verify_over(&w, "x", 255), "verify rejects x");
        assert!(over(NDecoder::try_decode(&w, 0)?.memoized().y(), "x", 255), "memoized walk rejects x");

        // Entry, earlier field p over max: iteration and q succeed, p, the
        // entry's and the message's encoded_length, verify and skip_n reject.
        let w = nframe(255, 300, 254, 300);
        let dec = NDecoder::try_decode(&w, 0)?;
        let entry = dec.pairs()?.next().ok_or("one entry")??;
        assert_eq!(entry.q()?.len(), 300, "q passes over over-max p bounds-only");
        assert!(over(entry.p(), "p", 255), "p() rejects its own length");
        assert!(over(entry.encoded_length(), "p", 255), "entry encoded_length rejects p");
        assert_eq!(dec.x()?.len(), 254, "x passes over the group bounds-only");
        assert!(over(dec.encoded_length(), "p", 255), "message encoded_length rejects p");
        assert!(over(dec.pairs()?.skip_n(1), "p", 255), "skip_n rejects p");
        assert!(verify_over(&w, "p", 255), "verify rejects p");

        // Entry, last field q over max: the iterator's warm extent must not
        // let q() skip its own check.
        let w = nframe(254, 301, 254, 300);
        let dec = NDecoder::try_decode(&w, 0)?;
        let entry = dec.pairs()?.next().ok_or("one entry")??;
        assert!(over(entry.q(), "q", 301), "q() rejects its own length on a warm entry");
        assert_eq!(entry.p()?.len(), 254, "p is unaffected");
        assert!(over(entry.encoded_length(), "q", 301), "entry encoded_length rejects q");
        assert_eq!(dec.y()?.len(), 300, "y passes over the group bounds-only");
        assert!(verify_over(&w, "q", 301), "verify rejects q");

        // Nested entry (M items.c over max): both iterators succeed, the
        // getter and both entries' encoded_length reject.
        let w = frame(254, 300, 1025);
        let dec = MDecoder::try_decode(&w, 0)?;
        let row = dec.rows()?.next().ok_or("one row")??;
        let item = row.items()?.next().ok_or("one item")??;
        assert!(over(item.c(), "c", 1025), "c() rejects its own length");
        assert!(over(item.encoded_length(), "c", 1025), "item encoded_length rejects c");
        assert!(over(row.encoded_length(), "c", 1025), "row encoded_length rejects nested c");
        assert_eq!(row.b()?.len(), 300, "b passes over nested c bounds-only");
        "#,
    );
    Ok(())
}
