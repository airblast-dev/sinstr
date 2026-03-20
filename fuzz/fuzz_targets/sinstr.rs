#![no_main]

use libfuzzer_sys::fuzz_target;
extern crate sinstr;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = str::from_utf8(data) {
        let ss = sinstr::SinStr::new(s);
        assert_eq!(data.len(), ss.len());
        assert_eq!(ss.as_str(), s);
    }
});
