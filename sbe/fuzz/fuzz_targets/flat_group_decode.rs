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
    let Ok(mut levels) = message.into_levels() else {
        return;
    };
    // Flat group — entries have no nested tails, next() returns EntryDecoder directly
    while let Some(entry) = levels.next() {
        let _ = entry.price();
        let _ = entry.qty();
        let _ = entry.num_orders();
    }
    // Consuming stage after group — flat message has none, finish returns ()
    let _ = levels.finish();
});
