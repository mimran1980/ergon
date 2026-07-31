#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(xml) = core::str::from_utf8(data) {
        let _ = ergo_sbe::parse(xml);
    }
});
