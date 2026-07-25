//! L3 order book demo — nested repeating groups with domain-type converters.
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

fn d(val: i64) -> Decimal { Decimal::new(val, 0) }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o1 = [(1001u64, d(5)), (1002, d(10))];
    let o2 = [(1003u64, d(25))];
    let bids = [(d(50800), d(15), o1.as_slice()), (d(50750), d(40), o2.as_slice())];
    let o3 = [(2001u64, d(10))]; let o4 = [(2002u64, d(20))]; let o5 = [(2003u64, d(40))];
    let asks = [(d(50850), d(20), o3.as_slice()), (d(50900), d(30), o4.as_slice()), (d(50950), d(50), o5.as_slice())];
    let symbol = b"BTCUSDT";

    // Encode — ragged entries with different order counts per level.
    // Size the buffer EXACTLY up-front via the staged L3BookEncodedLength
    // (no oversized buffer). The pre-computed length must equal the encoded length.
    let len = l3_book::book_encoded_length(&bids, &asks, symbol)?;
    let mut buf = vec![0u8; len];
    let actual = l3_book::encode_book(&mut buf, &bids, &asks, symbol)?;
    assert_eq!(len, actual, "pre-computed length must match encoded length");
    println!("encoded_len = {actual}");

    // Decode with concrete converter accessors — no turbofish.
    let dec = l3_book::L3BookDecoder::try_from(&buf[..actual])?;
    println!("{dec}");
    let _ts: DateTime<Utc> = dec.exchange_timestamp();
    assert!(dec.is_active());

    // DTO round-trip: decode -> L3BookDomain (owned, domain-typed fields),
    // then re-encode the DTO and verify the bytes are identical to the
    // original wire buffer.
    let dto = l3_book::L3BookDomain::from(
        l3_book::L3BookDecoder::try_from(&buf[..actual])?,
    );
    println!("DTO: {dto:?}");
    let mut buf2 = vec![0u8; len];
    let encoded2 = dto.encode(&mut buf2)?;
    assert_eq!(actual, encoded2, "DTO re-encode length must match original");
    assert_eq!(
        &buf[..actual], &buf2[..encoded2],
        "DTO round-trip must produce byte-identical output",
    );
    println!("DTO round-trip: byte-identical ({actual} bytes)");

    println!("\nOK");
    Ok(())
}
