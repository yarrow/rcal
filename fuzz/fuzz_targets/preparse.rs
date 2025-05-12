#![no_main]

use libfuzzer_sys::fuzz_target;
use rcal::preparse::preparse;
use rcal::preparse::with_regex::regex_preparse;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    let pre = preparse(data);
    let regex_pre = regex_preparse(data);

    match (pre.is_ok(), regex_pre.is_ok()) {
        (true, true) | (true, false) | (false, true) => assert_eq!(pre, regex_pre),
        (false, false) => assert_eq!(pre.unwrap_err().reason(), regex_pre.unwrap_err().reason()),
    }
});
