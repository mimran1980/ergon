# Buffer Sizing

**Why this exists:** true zero-copy publish on Aeron (and similar systems) uses
**`try_claim` / a pre-sized slot**. The transport hands you a buffer of a
**known length**; you must know the full encoded message size **before** you
write. Guessing with an oversized scratch `Vec` and copying later defeats that
model and is easy to get wrong for groups and var-data.

ergo-sbe therefore generates **schema-aware length APIs** so you describe the
shape you are about to encode (counts, nested groups, var-data byte lengths)
and get an **exact** size first — safer and easier than hand-computing header +
block + Σ(groups) + Σ(var-data).

| Message shape | Generated sizing | Prefer |
|---------------|------------------|--------|
| Fixed only | `{Msg}Encoder::compute_length_with_header()` (**const**) | stack / claim of that length |
| Groups / nested / ragged | `{Msg}EncodedLength` staged builder | `len` then encode into a claim/slot of `len` |

```text
// Exact size first (Car example), then encode into a slot of that length —
// e.g. Aeron try_claim, or any &mut [u8] with len == claim.
let len = CarEncodedLength::new()
    .fuel_figures(2)
    .usage_description(5)?
    .performance_figures(1)
    .acceleration(2)?
    .manufacturer(5)?
    .model(9)?
    .activation_code(6)?
    .encoded_length_with_header()
        .expect("header present");

// claim_or_slot.len() == len — no oversize guess buffer.
let actual_len = CarEncoder::try_wrap_and_apply_header(&mut claim_or_slot, 0)?
    .fixed(&fields)
    .fuel_figures(2, |g| { /* … */ Ok(()) })?
    .performance_figures(1, |g| { /* … */ Ok(()) })?
    .manufacturer(b"Honda")?
    .model(b"Civic")?
    .activation_code(b"active")?
    .encoded_length_with_header()
        .expect("header present");
assert_eq!(actual_len, len);
```

Nested books:
[`book_encoded_length`](../../../../samples/l3-book/src/lib.rs).
API matrix:
[`encoded_length_api_test`](../../../../sbe/tests/encoded_length_api_test.rs).
