#![no_main]

use libfuzzer_sys::fuzz_target;
use rcal::preparse::with_regex::regex_preparse;
use rcal::preparse::{CONTROL_CHARACTER as CTRL, UTF8_ERROR, preparse};

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    let pre = preparse(data);
    let regex_pre = regex_preparse(data);

    match (pre.is_ok(), regex_pre.is_ok()) {
        (true, true) | (true, false) | (false, true) => {
            assert_eq!(pre, regex_pre)
        }
        (false, false) => {
            let pre = pre.unwrap_err().reason();
            let reg = regex_pre.unwrap_err().reason();
            if pre != reg && (pre == CTRL || pre == UTF8_ERROR || reg == CTRL || reg == UTF8_ERROR)
            {
                assert_eq!(pre, reg);
            }
        }
    }
});
