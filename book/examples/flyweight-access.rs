//! Per-field flyweight access — zero-copy, no allocation.
//! Compiled against the feature-tour codec by the book-fence test.

// ANCHOR: flyweight_access
// Encode a sample car first (normally bytes come from the wire)
let complete_len = CarEncoder::compute_length()
    .fuel_figures_ragged(0, |_| Ok(()))?
    .performance_figures_ragged(0, |_| Ok(()))?
    .manufacturer(5)?
    .model(5)?
    .activation_code(3)?
    .encoded_length_with_header();
const PAD: usize = 256;
assert!(complete_len <= PAD, "car length {complete_len} exceeds pad {PAD}");
let mut storage = [0u8; PAD];
let buf = &mut storage[..complete_len];
let fields = CarFixedFields {
    serial_number: 1234,
    model_year: 2013,
    available: true.into(),
    code: Model::A,
    some_numbers: [10, 20, 30, 40],
    vehicle_code: *b"ABCDEF",
    extras: OptionalExtras::default(),
    engine: Engine::new(2000, 4, *b"123", 0i8, false.into(), Booster::new(BoostType::TURBO, 210)),
};
let n = CarEncoder::try_wrap_and_apply_header(buf, 0)?
    .fixed(&fields)
    .fuel_figures(0, |_| Ok(()))?
    .performance_figures(0, |_| Ok(()))?
    .manufacturer_as_str("Honda")?
    .model_as_str("Civic")?
    .activation_code_as_str("abc")?
    .encoded_length_with_header();
assert_eq!(n, complete_len);
// Now decode — read only the fields you need, no DTO allocation:
let car = CarDecoder::try_from(&buf[..n])?;
assert_eq!(car.serial_number(), 1234);
assert_eq!(car.model_year(), 2013);
assert_eq!(car.code(), Model::A);
assert_eq!(car.engine().capacity(), 2000);
// ANCHOR_END: flyweight_access
