//! Compiled against the feature-tour codec by the book-fence test.

// ANCHOR: var_data_callbacks
fn count_car_text_bytes(wire: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
    let mut total = 0;
    let _complete = CarDecoder::try_from(wire)?
        .skip_fuel_figures()?
        .skip_performance_figures()?
        .try_manufacturer(|bytes| -> Result<(), sbe_rt::DecodeError> {
            total += bytes.len();
            Ok(())
        })?
        .try_model(|bytes| -> Result<(), sbe_rt::DecodeError> {
            total += bytes.len();
            Ok(())
        })?
        .try_activation_code(|bytes| -> Result<(), sbe_rt::DecodeError> {
            total += bytes.len();
            Ok(())
        })?;
    Ok(total)
}
// ANCHOR_END: var_data_callbacks
