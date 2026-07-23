//! L3 order book demo — run with `cargo run`, test with `cargo test`.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o1 = [(1001u64, 5u64, 50800i64), (1002, 10, 50801)];
    let o2 = [(1003u64, 25u64, 50750i64)];
    let bids = [(50800i64, 15i64, o1.as_slice()), (50750, 40, o2.as_slice())];
    let o3 = [(2001u64, 10u64, 50850i64)];
    let o4 = [(2002u64, 20u64, 50900i64)];
    let o5 = [(2003u64, 40u64, 50950i64)];
    let asks = [(50850i64, 20i64, o3.as_slice()), (50900, 30, o4.as_slice()), (50950, 50, o5.as_slice())];
    let symbol = b"BTCUSDT";

    let len = l3_book::L3BookEncoder::compute_encoded_length_with_message_header(2, 3, symbol.len());
    println!("computed_len = {len}");
    let mut buf = vec![0u8; len];
    let actual = l3_book::encode_book(&mut buf, &bids, &asks, symbol)?;
    assert_eq!(len, actual);

    println!("{}", l3_book::L3BookDecoder::try_from(&buf[..])?);
    assert!(l3_book::L3BookDecoder::verify(&buf[..]).is_ok());
    println!("\nOK");
    Ok(())
}
