use bstr::ByteSlice;
use pretty_assertions::assert_eq;
use rcal::preparse::preparse;
use rcal::preparse::with_regex::regex_preparse;

fn compare(data: &[u8]) {
    let pre = preparse(data);
    let regex_pre = regex_preparse(data);
    //if (pre.is_ok() && regex_pre.is_ok()) || pre.is_ok() != regex_pre.is_ok() {
    //    assert_eq!(pre, regex_pre, "data is |{}|", data.as_bstr());
    //}
    match (pre.is_ok(), regex_pre.is_ok()) {
        (true, true) | (true, false) | (false, true) => {
            assert_eq!(
                pre,
                regex_pre,
                "(preparse != regex_preparse, data is |{:?}|)",
                data.as_bstr()
            )
        }
        (false, false) => assert_eq!(
            pre.unwrap_err().reason(),
            regex_pre.unwrap_err().reason(),
            "(preparse != regex_preparse, data is |{:?}|)",
            data.as_bstr()
        ),
    }
}
#[test]
fn regression_2a() {
    compare("2;a=:".as_bytes());
}
#[test]
fn regression_2a_quote_lt() {
    compare(r#"2;a="<":"#.as_bytes());
}
#[test]
fn regression_2a_quote_lt_and_a_trailing_quote() {
    compare(r#"2;a="<":""#.as_bytes());
}
#[test]
fn leading_x7f() {
    compare(b"\x7f");
}

#[test]
fn z_comma() {
    compare("z,".as_bytes());
}
#[test]
fn null_dash() {
    compare(b"\x00-");
}
#[test]
fn z_semi_two() {
    compare("z;2".as_bytes());
}
#[test]
fn unpaired_quote() {
    compare("2;4=\"".as_bytes());
}
#[test]
fn unpaired_quote_bang() {
    compare("2;A=\"!".as_bytes());
}
