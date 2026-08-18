//! Fixed-width char array access.
//! Compiled against the feature-tour codec by the book-fence test.

// ANCHOR: char_arrays
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
    .manufacturer(b"Honda")?
    .model(b"Civic")?
    .activation_code(b"abc")?
    .encoded_length_with_header();
assert_eq!(n, complete_len);

let car = CarDecoder::try_from(&buf[..n])?;
let mut dst = [0u8; 6];
assert_eq!(car.copy_vehicle_code(&mut dst), 6);
assert_eq!(&dst, b"ABCDEF");
assert_eq!(car.vehicle_code(), *b"ABCDEF");
// ANCHOR_END: char_arrays
