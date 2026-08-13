//! L3 order book demos — four SBE patterns: uniform, ragged, var-data, depth-3.
//!
//! Each example computes the exact encoded length up-front via the staged
//! EncodedLength builder (zero user-defined constants), allocates the exact
//! buffer, encodes, decodes, and prints the result.
//!
//! Run: `cargo run`

use chrono::{DateTime, Utc};
use rust_decimal::dec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    example_1_uniform()?;
    example_2_ragged()?;
    example_3_vardata()?;
    example_4_depth3()?;
    Ok(())
}

// ── Example 1: Uniform groups (all entries have the same shape) ───────────
//
// Every bid has exactly 2 orders, every ask has exactly 1 order.
// The `uniform(count)` shortcut registers identical entries without a loop.

fn example_1_uniform() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 1: Uniform groups ===\n");

    // All bids have 2 orders, all asks have 1 order.
    let o = [(1u64, dec!(5.00)), (2, dec!(10.00))];
    let bids = [
        (dec!(50000), dec!(10), o.as_slice()),
        (dec!(50100), dec!(20), o.as_slice()),
    ];
    let o2 = [(3u64, dec!(7.50))];
    let asks = [(dec!(50200), dec!(15), o2.as_slice())];
    let symbol = b"ETH";

    let len = l3_book::book_encoded_length(&bids, &asks, symbol)?;
    let mut storage = [0u8; 4096];
    assert!(len <= storage.len(), "book len {len} exceeds stack pad");
    let buf = &mut storage[..len];
    let actual = l3_book::encode_book(buf, &bids, &asks, symbol)?;
    assert_eq!(len, actual);

    let dec = l3_book::L3BookDecoder::try_from(&buf[..actual])?;
    println!("  {dec}");
    println!("  encoded_len = {actual}\n");
    Ok(())
}

// ── Example 2: Ragged groups (entries have different shapes) ──────────────
//
// Bid 1 has 2 orders, bid 2 has 1 order. Each bid entry's inner group
// count differs — the ragged path handles this by calling `add()` per entry.

fn example_2_ragged() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 2: Ragged groups ===\n");

    let o1 = [(101u64, dec!(5.25)), (102, dec!(10.50))];
    let o2 = [(103u64, dec!(25.75))];
    let bids = [
        (dec!(50800.50), dec!(15.25), o1.as_slice()),
        (dec!(50750.25), dec!(40.10), o2.as_slice()),
    ];
    let o3 = [(201u64, dec!(10.30))];
    let o4 = [(202u64, dec!(20.00))];
    let o5 = [(203u64, dec!(40.15))];
    let asks = [
        (dec!(50850.00), dec!(20.50), o3.as_slice()),
        (dec!(50900.75), dec!(30.25), o4.as_slice()),
        (dec!(50950.10), dec!(50.00), o5.as_slice()),
    ];
    let symbol = b"BTCUSDT";

    let len = l3_book::book_encoded_length(&bids, &asks, symbol)?;
    let mut storage = [0u8; 4096];
    assert!(len <= storage.len(), "book len {len} exceeds stack pad");
    let buf = &mut storage[..len];
    let actual = l3_book::encode_book(buf, &bids, &asks, symbol)?;
    assert_eq!(len, actual);

    let dec = l3_book::L3BookDecoder::try_from(&buf[..actual])?;
    let _ts: DateTime<Utc> = dec.try_exchange_timestamp()?;
    println!("  {dec}");

    // DTO round-trip — byte-identical.
    let dto =
        l3_book::L3BookDomain::try_from_decoder(l3_book::L3BookDecoder::try_from(&buf[..actual])?)?;
    let mut storage2 = [0u8; 4096];
    let encoded2 = dto.encode(&mut storage2[..len])?;
    assert_eq!(
        &buf[..actual],
        &storage2[..encoded2],
        "DTO round-trip must be byte-identical"
    );
    println!("  DTO round-trip: byte-identical ({actual} bytes)\n");
    Ok(())
}

// ── Example 3: Var-data ragged (orders carry variable-length order_id) ────
//
// Each order has a var-data `order_id` of differing length (e.g. "A" vs "BBB").
// The ragged wrapper exposes `og.add()?.order_id(len)?` — field-named,
// chained, zero constants.

fn example_3_vardata() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 3: Var-data ragged ===\n");

    let o1 = [(dec!(1.50), &b"ORD-1"[..]), (dec!(2.00), &b"ORD-22"[..])];
    let o2 = [(dec!(3.75), &b"X"[..])];
    let bids = [
        (dec!(100), dec!(10), o1.as_slice()),
        (dec!(200), dec!(5), o2.as_slice()),
    ];
    let o3 = [(dec!(0.50), &b"AA"[..])];
    let asks = [(dec!(150), dec!(8), o3.as_slice())];
    let symbol = b"LINK";

    let len = l3_book::vardata_book_encoded_length(&bids, &asks, symbol)?;
    let mut storage = [0u8; 4096];
    assert!(
        len <= storage.len(),
        "vardata book len {len} exceeds stack pad"
    );
    let buf = &mut storage[..len];
    let actual = l3_book::encode_vardata_book(buf, &bids, &asks, symbol)?;
    assert_eq!(len, actual);

    let dec = l3_book::L3BookVarDataDecoder::try_from(&buf[..actual])?;
    println!("  {dec}");
    println!("  encoded_len = {actual}\n");
    Ok(())
}

// ── Example 4: Depth-3 nesting (levels → items → tag var-data) ───────────
//
// Three levels deep: message → levels group → items nested group → tag var-data.
// Tests recursive entry-tail generation in the staged length builder.

fn example_4_depth3() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 4: Depth-3 nesting ===\n");

    let i1 = [(1u64, &b"A"[..]), (2u64, &b"BB"[..])];
    let i2 = [(3u64, &b"CCC"[..])];
    let levels = [(10u32, i1.as_slice()), (20u32, i2.as_slice())];
    let description = b"depth-3 test message";

    let len = l3_book::depth3_encoded_length(&levels, description)?;
    let mut storage = [0u8; 4096];
    assert!(len <= storage.len(), "depth3 len {len} exceeds stack pad");
    let buf = &mut storage[..len];
    let actual = l3_book::encode_depth3(buf, 42, &levels, description)?;
    assert_eq!(len, actual);

    let dec = l3_book::Depth3TestDecoder::try_from(&buf[..actual])?;
    println!("  {dec}");

    // Verify ragged structure.
    let mut lvl = dec.into_levels()?;
    let l1 = lvl.next().transpose()?.unwrap();
    let mut it1 = l1.into_items()?;
    assert_eq!(it1.next().transpose()?.unwrap().tag()?, b"A");
    assert_eq!(it1.next().transpose()?.unwrap().tag()?, b"BB");
    assert!(it1.next().is_none());

    let l2 = lvl.next().transpose()?.unwrap();
    let mut it2 = l2.into_items()?;
    assert_eq!(it2.next().transpose()?.unwrap().tag()?, b"CCC");
    assert!(it2.next().is_none());
    assert!(lvl.next().is_none());
    println!("  encoded_len = {actual}\n");

    println!("All examples OK");
    Ok(())
}
