#![no_main]

use ergo_sbe_fuzz::orderbook_codec::BookSnapshotDecoder;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Decode only input that verifies.
    if BookSnapshotDecoder::verify(data).is_err() {
        return;
    }
    let Ok(message) = BookSnapshotDecoder::try_from(data) else {
        return;
    };
    let Ok(mut levels) = message.levels() else {
        return;
    };
    // bulk_decode with its hoisted bounds check must not panic.
    let _ = levels.bulk_decode();
});
