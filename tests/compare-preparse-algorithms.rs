use bstr::ByteSlice;
use pretty_assertions::assert_eq;
use rcal::preparse::with_regex::regex_preparse;
use rcal::preparse::{CONTROL_CHARACTER as CTRL, UTF8_ERROR, preparse};

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
        (false, false) => {
            let pre = pre.unwrap_err().reason();
            let reg = regex_pre.unwrap_err().reason();
            if pre != reg && (pre == CTRL || pre == UTF8_ERROR || reg == CTRL || reg == UTF8_ERROR)
            {
                assert_eq!(pre, reg, "(preparse != regex_preparse, data is |{:?}|)", data.as_bstr())
            }
        }
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
#[test]
fn zero_255() {
    compare(b"\x00\xFF");
}
#[test]
fn bytes_239_0() {
    compare(b"\xEF\x00");
}
#[test]
fn y_semi_z_semi_ctrl_r() {
    compare(b"y;z=;\x12");
}
#[test]
fn semi_255() {
    compare(b";\xFF");
}
#[test]
fn two_4_equal_tab_ctrl_a() {
    compare(b"2;4=\"\t\x01");
}
#[test]
fn z_quote() {
    compare(b"z\"");
}
#[test]
fn three_z_ux() {
    compare("3zǙ".as_bytes());
}
