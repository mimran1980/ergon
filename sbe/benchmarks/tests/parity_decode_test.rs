//! Correctness guard for direct body-offset parity benchmarks.

use ergo_sbe_benchmarks::{ergo_car::CarDecoder, sbe_tool_car_body_decoder};

const BASELINE: &[u8] = include_bytes!("../benches/fixtures/car_example_baseline_data.sbe");

#[test]
fn direct_body_decoders_read_identical_values_after_the_header()
-> Result<(), Box<dyn std::error::Error>> {
    let ergon = CarDecoder::try_from(BASELINE)?;
    let block_length = u16::from_le_bytes(BASELINE[0..2].try_into()?);
    let version = u16::from_le_bytes(BASELINE[6..8].try_into()?);
    let sbe_tool = sbe_tool_car_body_decoder(BASELINE, 0, block_length, version);

    assert_eq!(ergon.serial_number(), 1234);
    assert_eq!(sbe_tool.serial_number(), 1234);
    assert_eq!(ergon.model_year(), 2013);
    assert_eq!(sbe_tool.model_year(), 2013);
    assert_eq!(ergon.some_numbers(), [1, 2, 3, 4]);
    assert_eq!(sbe_tool.some_numbers(), [1, 2, 3, 4]);
    assert_eq!(ergon.engine().capacity(), 2000);
    assert_eq!(sbe_tool.engine_decoder().capacity(), 2000);

    Ok(())
}
