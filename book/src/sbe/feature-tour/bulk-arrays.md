# Bulk Arrays

Generated bulk helpers avoid per-element boilerplate, while constants and
`MetaAttribute` expose schema metadata:

```rust,no_run
  let mut buf = [0u8; QuoteEncoder::compute_length_with_header(0, 0)];
  let len = QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)?
      .fixed(&QuoteFixedFields {
          seq: 7,
          some_numbers: [1, 2, 3, 4],
          vehicle_code: *b"EURUSD",
          qty: 10,
      })
      .legs(0, |_| Ok(()))?
      .note(b"")?
      .encoded_length_with_header();
  let quote = QuoteDecoder::try_from(&buf[..len])?;
  assert_eq!(quote.some_numbers(), [1, 2, 3, 4]);
  let mut code = [0u8; 6];
  assert_eq!(quote.copy_vehicle_code(&mut code), code.len());
  assert_eq!(&code, b"EURUSD");
  assert_eq!(QuoteDecoder::SEQ_ID, 1);
  assert_eq!(
      QuoteDecoder::seq_meta_attribute(sbe_rt::MetaAttribute::Presence),
      Some("required")
  );
```
