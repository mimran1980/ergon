#![no_main]

use ergo_sbe_fuzz::orderbook_codec::BookSnapshotDecoder;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if BookSnapshotDecoder::verify(data).is_err() {
        return;
    }
    let Ok(message) = BookSnapshotDecoder::try_from(data) else {
        return;
    };
    let Ok(levels) = message.levels() else {
        return;
    };
    for entry in levels {
        let _ = entry.price();
        let _ = entry.qty();
        let _ = entry.num_orders();
    }
});
