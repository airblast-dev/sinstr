#![no_main]

use libfuzzer_sys::fuzz_target;
use sinstr::SinStr;

fuzz_target!(|data: Vec<&str>| {
    let mut ss = SinStr::default();
    for s in data.iter() {
        ss.set_str(s);
    }
    for s in data.iter() {
        ss.push_str(s);
    }
});
