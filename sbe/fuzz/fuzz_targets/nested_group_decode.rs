#![no_main]

use ergo_sbe_fuzz::l3_codec::{L3BookDecoder, sbe_rt};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if L3BookDecoder::verify(data).is_err() {
        return;
    }
    let Ok(message) = L3BookDecoder::try_from(data) else {
        return;
    };
    let Ok(after_bids) = message.into_bids(|level| -> Result<_, sbe_rt::DecodeError> {
        level.into_orders(|order| -> Result<_, sbe_rt::DecodeError> {
            let (_id, complete) = order.into_order_id()?;
            Ok(complete)
        })
    }) else {
        return;
    };
    let _ = after_bids.into_asks(|level| -> Result<_, sbe_rt::DecodeError> {
        level.into_orders(|order| -> Result<_, sbe_rt::DecodeError> {
            let (_id, complete) = order.into_order_id()?;
            Ok(complete)
        })
    });
});
