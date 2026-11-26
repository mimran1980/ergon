#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(dead_code)]
#![allow(unused)]

mod little_endian {
    include!(concat!(env!("OUT_DIR"), "/little_endian.rs"));
}

mod big_endian {
    include!(concat!(env!("OUT_DIR"), "/big_endian.rs"));
}

mod nested {
    include!(concat!(env!("OUT_DIR"), "/nested.rs"));
}

mod zero_length_header {
    include!(concat!(env!("OUT_DIR"), "/zero_length_header.rs"));
}

mod setters {
    include!(concat!(env!("OUT_DIR"), "/setters.rs"));
}

#[cfg(test)]
mod tests {
    use super::{big_endian, little_endian, nested, setters, zero_length_header};

    /// The composite, enum, set and bool setters write through
    /// `get_unchecked_mut` or a raw pointer. On a buffer that ends exactly at
    /// the block, from an unaligned offset, every setter on every encoder stage
    /// and on the raw writer must stay in bounds and land its bytes.
    #[test]
    fn unchecked_fixed_setters_stay_in_bounds() -> Result<(), Box<dyn std::error::Error>> {
        use setters::{
            BooleanType, Flags, Pair, SettersDecoder, SettersEncoder, SettersFixedFields, Side,
        };
        const OFFSET: usize = 3;
        let check = |buffer: &[u8]| -> Result<(), Box<dyn std::error::Error>> {
            let decoder = SettersDecoder::try_decode(buffer, OFFSET)?;
            assert_eq!(decoder.side(), Side::Sell);
            assert_eq!(decoder.flags().0, 0b11);
            assert_eq!(
                (decoder.pair().left(), decoder.pair().right()),
                (0x0102, 0x0304)
            );
            assert!(decoder.active_bool()?);
            Ok(())
        };
        let mut buffer = [0u8; OFFSET + SettersEncoder::compute_length_with_header()];

        let mut unfixed = SettersEncoder::try_wrap_and_apply_header(&mut buffer, OFFSET)?;
        unfixed
            .side(Side::Sell)
            .flags(Flags(0b11))
            .pair(Pair::new(0x0102, 0x0304))
            .active(BooleanType::F)
            .active_bool(true);
        check(&buffer)?;

        buffer.fill(0);
        let fields = SettersFixedFields {
            side: Side::Buy,
            flags: Flags(0),
            pair: Pair::new(0, 0),
            active: BooleanType::F,
        };
        let mut fixed =
            SettersEncoder::try_wrap_and_apply_header(&mut buffer, OFFSET)?.fixed(&fields);
        fixed
            .side(Side::Sell)
            .flags(Flags(0b11))
            .pair(Pair::new(0x0102, 0x0304))
            .active(BooleanType::F)
            .active_bool(true);
        check(&buffer)?;

        buffer.fill(0);
        let mut raw = SettersEncoder::try_wrap_and_apply_header(&mut buffer, OFFSET)?.raw_fixed();
        raw.side(Side::Sell)
            .flags(Flags(0b11))
            .pair(Pair::new(0x0102, 0x0304))
            .active(BooleanType::F)
            .active_bool(true);
        check(&buffer)?;
        Ok(())
    }

    /// A zero-length header and body make every length guard see N = 0: an
    /// offset past the buffer must be an error, never a pointer offset out
    /// of the allocation.
    #[test]
    fn zero_length_frame_rejects_offset_past_buffer() -> Result<(), Box<dyn std::error::Error>> {
        use zero_length_header::{AnyMessage, EmptyDecoder, EmptyEncoder};
        let buf = [0u8; 4];
        let mut out = [0u8; 4];
        for offset in [5usize, 100, usize::MAX] {
            assert!(EmptyDecoder::try_decode(&buf, offset).is_err());
            assert!(AnyMessage::try_decode(&buf, offset).is_err());
            assert!(AnyMessage::decode_frame(&buf, offset, 0).is_err());
            assert!(EmptyEncoder::try_wrap_and_apply_header(&mut out, offset).is_err());
            assert!(EmptyEncoder::try_wrap(&mut out, offset).is_err());
        }
        EmptyDecoder::try_decode(&buf, 4)?;
        AnyMessage::try_decode(&buf, 4)?;
        AnyMessage::decode_frame(&buf, 4, 0)?;
        Ok(())
    }

    #[test]
    fn little_endian_fixed_codec() -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; little_endian::ProbeEncoder::compute_length_with_header()];
        let len = little_endian::ProbeEncoder::try_wrap_and_apply_header(&mut buffer, 0)?
            .fixed(&little_endian::ProbeFixedFields { value: 0x0102_0304 })
            .encoded_length_with_header();
        assert_eq!(&buffer[8..len], &[4, 3, 2, 1]);
        assert_eq!(
            little_endian::ProbeDecoder::try_from(&buffer[..len])?.value(),
            0x0102_0304
        );
        Ok(())
    }

    #[test]
    fn big_endian_fixed_codec() -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; big_endian::ProbeEncoder::compute_length_with_header()];
        let len = big_endian::ProbeEncoder::try_wrap_and_apply_header(&mut buffer, 0)?
            .fixed(&big_endian::ProbeFixedFields { value: 0x0102_0304 })
            .encoded_length_with_header();
        assert_eq!(&buffer[8..len], &[1, 2, 3, 4]);
        assert_eq!(
            big_endian::ProbeDecoder::try_from(&buffer[..len])?.value(),
            0x0102_0304
        );
        Ok(())
    }

    #[test]
    fn nested_group_codec() -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; 256];
        let len = nested::TreeEncoder::try_wrap_and_apply_header(&mut buffer, 0)?
            .fixed(&nested::TreeFixedFields {})
            .outer(1, |outer| {
                // Each entry completes with its tail (nested group / var-data),
                // so the closure returns that proof instead of `Ok(())`.
                outer.add(|mut entry| {
                    entry.value(7);
                    entry.inner(1, |inner| {
                        inner.add(|mut row| {
                            row.quantity(9);
                            row.label(b"miri")
                        })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        nested::TreeDecoder::verify(&buffer[..len])?;
        let tree = nested::TreeDecoder::try_from(&buffer[..len])?;
        // Rows carry `label` var-data, so both levels are visit closures and
        // the inner walk returns the outer entry's completion directly.
        tree.into_outer(|entry| -> Result<_, nested::sbe_rt::DecodeError> {
            assert_eq!(entry.value(), 7);
            entry.into_inner(|row| {
                assert_eq!(row.quantity(), 9);
                let (label, complete) = row.into_label()?;
                assert_eq!(label, b"miri");
                Ok(complete)
            })
        })?;
        Ok(())
    }

    /// The entry-level nested-group methods reborrow the entry buffer for the
    /// closure and then keep writing through it. Both the counted and the
    /// unknown-size form, in one frame, with a write after each nested group.
    #[test]
    fn nested_group_reborrow_keeps_buffer_valid() -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; 256];
        let len = nested::TreeEncoder::try_wrap_and_apply_header(&mut buffer, 0)?
            .fixed(&nested::TreeFixedFields {})
            .outer(2, |outer| {
                outer.add(|mut entry| {
                    entry.value(1);
                    entry.inner(2, |inner| {
                        inner.add(|mut row| {
                            row.quantity(10);
                            row.label(b"a")
                        })?;
                        inner.add(|mut row| {
                            row.quantity(11);
                            row.label(b"b")
                        })?;
                        Ok(())
                    })
                })?;
                outer.add(|mut entry| {
                    entry.value(2);
                    entry.inner_unknown_size(|inner| {
                        inner.add(|mut row| {
                            row.quantity(20);
                            row.label(b"cc")
                        })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        // header 8 + outer dim 4 + entry 1 (4 + dim 4 + 2 * 13) + entry 2 (4 + dim 4 + 14)
        assert_eq!(len, 68);
        nested::TreeDecoder::verify(&buffer[..len])?;
        Ok(())
    }
}
