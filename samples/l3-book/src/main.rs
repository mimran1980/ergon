//! L3 order book demo — run with `cargo run`, test with `cargo test`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o1 = [(1001u64, l3_book::Decimal::new(5, 0)), (1002, l3_book::Decimal::new(10, 0))];
    let o2 = [(1003u64, l3_book::Decimal::new(25, 0))];
    let bids = [
        (l3_book::Decimal::new(50800, 0), l3_book::Decimal::new(15, 0), o1.as_slice()),
        (l3_book::Decimal::new(50750, 0), l3_book::Decimal::new(40, 0), o2.as_slice()),
    ];
    let o3 = [(2001u64, l3_book::Decimal::new(10, 0))]; let o4 = [(2002u64, l3_book::Decimal::new(20, 0))]; let o5 = [(2003u64, l3_book::Decimal::new(40, 0))];
    let asks = [
        (l3_book::Decimal::new(50850, 0), l3_book::Decimal::new(20, 0), o3.as_slice()),
        (l3_book::Decimal::new(50900, 0), l3_book::Decimal::new(30, 0), o4.as_slice()),
        (l3_book::Decimal::new(50950, 0), l3_book::Decimal::new(50, 0), o5.as_slice()),
    ];
    let symbol = b"BTCUSDT";

    // Exact-length pre-compute — all counts known from slice lengths
    let len = l3_book::L3BookEncodedLength::new()
        .bids(bids.len() as u16, |b| {
            for (_, _, orders) in &bids {
                b.add()?;
                b.orders(orders.len() as u16, |o| { for _ in *orders { o.add()?; } Ok(()) })?;
            }
            Ok(())
        })?
        .asks(asks.len() as u16, |a| {
            for (_, _, orders) in &asks {
                a.add()?;
                a.orders(orders.len() as u16, |o| { for _ in *orders { o.add()?; } Ok(()) })?;
            }
            Ok(())
        })?
        .symbol(symbol.len())?
        .encoded_length_with_header();
    println!("computed_len = {len}");

    let mut buf = vec![0u8; len];
    let actual = l3_book::encode_book(&mut buf, &bids, &asks, symbol)?;
    assert_eq!(len, actual, "length builder vs actual encode mismatch");

    let dec = l3_book::L3BookDecoder::try_from(&buf[..])?;
    println!("{dec}");
    assert!(dec.is_active_bool());
    assert!(l3_book::L3BookDecoder::verify(&buf[..]).is_ok());
    println!("\nOK");
    Ok(())
}
