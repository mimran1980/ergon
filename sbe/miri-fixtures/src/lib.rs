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

#[cfg(test)]
mod tests {
    use super::{big_endian, little_endian, nested};

    #[test]
    fn little_endian_fixed_codec() -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; little_endian::ProbeEncoder::ENCODED_LENGTH];
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
        let mut buffer = [0u8; big_endian::ProbeEncoder::ENCODED_LENGTH];
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
            .outer(1, |outer| {
                outer.add(|entry| {
                    entry.value(7).inner(1, |inner| {
                        inner.add(|row| {
                            row.quantity(9).label(b"miri")?;
                            Ok(())
                        })?;
                        Ok(())
                    })?;
                    Ok(())
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        nested::TreeDecoder::verify(&buffer[..len])?;
        let tree = nested::TreeDecoder::try_from(&buffer[..len])?;
        let mut outer = tree.into_outer()?;
        let entry = outer.next().expect("one outer entry")?;
        assert_eq!(entry.value(), 7);
        let mut inner = entry.into_inner()?;
        let row = inner.next().expect("one inner entry")?;
        assert_eq!(row.quantity(), 9);
        assert_eq!(row.into_label()?.0, b"miri");
        Ok(())
    }
}
