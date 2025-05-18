#![no_main]

use bstr::ByteSlice;
use libfuzzer_sys::fuzz_target;
use rcal::preparse::preparse;
use rcal::preparse::with_regex::regex_preparse;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    let pre = preparse(data);
    let reg = regex_preparse(data);
    assert_eq!(pre, reg, "pre!=reg text:{:?}\n{pre:?}\n{reg:?}", data.as_bstr())
});
