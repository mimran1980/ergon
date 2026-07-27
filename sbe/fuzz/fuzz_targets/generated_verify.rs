#![no_main]

use ergo_sbe_fuzz::l3_codec::L3BookDecoder;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = L3BookDecoder::verify(data);
});
