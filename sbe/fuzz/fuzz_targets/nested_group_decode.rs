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
    let Ok(mut bids) = message.into_bids() else {
        return;
    };
    for level in &mut bids {
        let Ok(level) = level else {
            return;
        };
        let Ok(mut orders) = level.into_orders() else {
            return;
        };
        for order in &mut orders {
            let Ok(order) = order else {
                return;
            };
            let _ = order.into_order_id();
        }
    }
    let Ok(mut asks) = bids.into_asks() else {
        return;
    };
    for level in &mut asks {
        let Ok(level) = level else {
            return;
        };
        let Ok(mut orders) = level.into_orders() else {
            return;
        };
        for order in &mut orders {
            let Ok(order) = order else {
                return;
            };
            let _ = order.into_order_id();
        }
    }
});
