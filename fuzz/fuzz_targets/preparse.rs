#![no_main]

use bstr::ByteSlice;
use libfuzzer_sys::fuzz_target;
use rcal::preparse::preparse;
use rcal::preparse::with_regex::regex_preparse;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    let pre = preparse(data);
    let reg = regex_preparse(data);

    match (pre.is_ok(), reg.is_ok()) {
        (true, true) | (true, false) | (false, true) => {
            assert_eq!(pre, reg, "pre!=reg text:{:?}\n{pre:#?}\n{reg:#?}", data.as_bstr())
        }
        (false, false) => {
            let pre = pre.unwrap_err();
            let reg = reg.unwrap_err();
            if pre != reg
                && (pre.is_control_char_error()
                    || pre.is_utf8_error()
                    || reg.is_control_char_error()
                    || reg.is_utf8_error())
            {
                assert_eq!(pre, reg, "pre!=reg text:{:?}\n{pre:?}\n{reg:?}", data.as_bstr())
            }
        }
    }
});
