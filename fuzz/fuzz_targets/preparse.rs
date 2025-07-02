#![no_main]

use bstr::ByteSlice;
use libfuzzer_sys::fuzz_target;
use rcal::cautious_preparse;
use rcal::bold_preparse;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    let bold = bold_preparse(data);
    let cautious = cautious_preparse(data);
    assert_eq!(
        cautious,
        bold,
        "cautious!=bold text:{:?}\n{cautious:?}\n{bold:?}",
        data.as_bstr()
    )
});
