#![no_main]

use ergo_sbe_fuzz::l3_codec::{L3BookDecoder, sbe_rt};
use libfuzzer_sys::fuzz_target;

// Every tail in this schema is dynamic — levels carry a nested `orders` group,
// and orders carry `orderId` var-data — so the whole walk is visit closures.
// Each closure returns the entry's completion, which is the next cursor.
fuzz_target!(|data: &[u8]| {
    if L3BookDecoder::verify(data).is_err() {
        return;
    }
    let Ok(message) = L3BookDecoder::try_from(data) else {
        return;
    };
    let Ok(after_bids) = message.into_bids::<sbe_rt::DecodeError, _>(|level| {
        level.into_orders(|order| order.into_order_id().map(|(_, complete)| complete))
    }) else {
        return;
    };
    let _ = after_bids.into_asks::<sbe_rt::DecodeError, _>(|level| {
        level.into_orders(|order| order.into_order_id().map(|(_, complete)| complete))
    });
});
