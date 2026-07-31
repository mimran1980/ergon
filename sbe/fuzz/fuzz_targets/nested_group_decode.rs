#![no_main]

use ergo_sbe_fuzz::l3_codec::L3BookDecoder;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if L3BookDecoder::verify(data).is_err() {
        return;
    }
    let Ok(message) = L3BookDecoder::try_from(data) else {
        return;
    };
    let Ok(mut bids) = message.into_bids() else {
        return;
    };
    while let Some(Ok(level)) = bids.next() {
        let Ok(mut orders) = level.into_orders() else {
            return;
        };
        while let Some(Ok(order)) = orders.next() {
            let _ = order.into_order_id();
        }
        if orders.finish().is_err() {
            return;
        }
    }
    let Ok(after_bids) = bids.finish() else {
        return;
    };
    let Ok(mut asks) = after_bids.into_asks() else {
        return;
    };
    while let Some(Ok(level)) = asks.next() {
        let Ok(mut orders) = level.into_orders() else {
            return;
        };
        while let Some(Ok(order)) = orders.next() {
            let _ = order.into_order_id();
        }
        if orders.finish().is_err() {
            return;
        }
    }
    let _ = asks.finish();
});
