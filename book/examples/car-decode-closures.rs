//! Compiled against the feature-tour codec by the book-fence test.

// ANCHOR: var_data_tuples
fn count_car_text_bytes(wire: &[u8]) -> Result<usize, sbe_rt::DecodeError> {
    let (manufacturer, next) = CarDecoder::try_from(wire)?
        .skip_fuel_figures()?
        .skip_performance_figures()?
        .into_manufacturer()?;
    let (model, next) = next.into_model()?;
    let (activation_code, _complete) = next.into_activation_code()?;
    Ok(manufacturer.len() + model.len() + activation_code.len())
}
// ANCHOR_END: var_data_tuples
