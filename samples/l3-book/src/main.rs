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

    // 1. Pre-compute exact buffer size with the staged length builder.
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

    // 2. Encode — method chain reads top-to-bottom in wire order.
    let mut buf = vec![0u8; len];
    let actual = l3_book::encode_book(&mut buf, &bids, &asks, symbol)?;
    assert_eq!(len, actual);

    // 3. Decode with concrete converter accessors — no turbofish.
    let dec = l3_book::L3BookDecoder::try_from(&buf[..])?;
    println!("{dec}");
    let _ts: DateTime<Utc> = dec.exchange_timestamp();
    assert!(dec.is_active());

    println!("\nOK");
    Ok(())
}
