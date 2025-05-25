use super::with_regex::regex_preparse;
use super::*;
use crate::error::{Problem, Segment};
use Problem::*;
use Segment::*;
use bstr::{BString, ByteSlice};
use pretty_assertions::assert_eq;

fn equivalent_from_bytes(text: &[u8]) -> Result<Prop, PreparseError> {
    let pre = preparse(text);
    let reg = regex_preparse(text);
    assert_eq!(pre, reg, "pre!=reg, text: {:?}\n pre {:?}\nreg {:?}", text.as_bstr(), pre, reg);
    pre
}
fn equivalent(text: &str) -> Result<Prop, PreparseError> {
    let pre = preparse(text.as_bytes());
    let reg = regex_preparse(text.as_bytes());
    assert_eq!(pre, reg, "pre!=reg, text: {text}");
    pre
}
fn err_for(text: &str) -> (Segment, Problem) {
    let err = equivalent(text).unwrap_err();
    (err.segment, err.problem)
}
fn err_from_bytes(text: &[u8]) -> (Segment, Problem) {
    let err = equivalent_from_bytes(text).unwrap_err();
    (err.segment, err.problem)
}

fn parse(text: &str) -> StrProp<'_> {
    delocate(&equivalent(text).unwrap())
}

fn err_is(text: &str, expected: (Segment, Problem)) {
    assert_eq!(err_for(text), expected, "text: |{text}|");
}
#[test]
fn property_name_only() {
    err_is("A", (PropertyName, Unterminated));
}
#[test]
fn property_name_semicolon_only() {
    err_is("A;", (ParamName, Empty));
}
#[test]
fn no_property_value() {
    err_is("A;B=", (PropertyValue, Empty));
    err_is("A;B=c", (PropertyValue, Empty));
}
#[test]
fn quotes_allow_punctuation_in_values() {
    err_is(r#"A;B=",C=:""#, (PropertyValue, Empty));
    err_is(r#"A;B=":C=:""#, (PropertyValue, Empty));
    err_is(r#"A;B=";C=:""#, (PropertyValue, Empty));
}
#[test]
fn forbid_embedded_dquotes() {
    err_is(r#"A;B=ab"c":val"#, (ParamValue, DoubleQuote));
}
#[test]
fn forbid_space_after_ending_dquote() {
    err_is(r#"A;B="c" ,"d":val"#, (ParamValue, Unterminated));
}
#[test]
fn forbid_dquote_after_ending_dquote() {
    err_is(r#"A;B="c"","d":val"#, (ParamValue, DoubleQuote));
}
#[test]
fn forbid_control_character_after_ending_dquote() {
    let mut text = BString::from(r#"A;B="c" ,"d":val"#);
    text[7] = 3;
    assert_eq!(err_from_bytes(&text), (ParamValue, ControlCharacter));
}
#[test]
fn property_name_required() {
    err_is(":foo", (PropertyName, Empty));
    err_is("/foo", (PropertyName, Empty));
}
#[test]
fn forbid_empty_content_line() {
    err_is("", (PropertyName, EmptyContentLine));
}
#[test]
fn parameter_name_required() {
    err_is("Foo;=bar:", (ParamName, Empty));
    err_is("Foo;/:", (ParamName, Empty));
}
#[test]
fn must_be_utf8_len_2() {
    let mut bad = BString::from("FOO:bá");
    //let mut bad = BString::from("abc𒀁");
    let len = bad.len();
    bad[len - 1] = b'a';
    assert_eq!(
        err_from_bytes(bad.as_slice()),
        (PropertyValue, Utf8Error(Some(1))),
        "text: {:?}",
        bad
    );
}
#[test]
fn must_be_utf8_len_4() {
    let mut bad = BString::from("abc𒀁");
    let len = bad.len();
    bad[len - 2] = b'a';
    assert_eq!(
        err_from_bytes(bad.as_slice()),
        (PropertyName, Utf8Error(Some(2))),
        "text: {:?}",
        bad
    );
}
// Tests for the result returned
#[derive(Debug, PartialEq)]
struct StrParam<'a> {
    name: &'a str,
    values: Vec<&'a str>,
}
#[derive(Debug, PartialEq)]
struct StrProp<'a> {
    name: &'a str,
    parameters: Vec<StrParam<'a>>,
    value: &'a str,
}
fn delocate<'a>(prop: &Prop<'a>) -> StrProp<'a> {
    StrProp {
        name: prop.name.val,
        value: prop.value.val,
        parameters: prop
            .parameters
            .iter()
            .map(|param| StrParam {
                name: param.name.val,
                values: param.values.iter().map(|value| value.val).collect(),
            })
            .collect(),
    }
}
#[allow(clippy::needless_pass_by_value)]
fn as_expected(text: &str, expected: StrProp) {
    assert_eq!(parse(text), expected, "text: |{text}|");
}

#[test]
fn minimal() {
    let text = "-:";
    let expected = StrProp { name: "-", value: "", parameters: Vec::new() };
    as_expected(text, expected);
}
#[test]
fn attach() {
    let text =
        "ATTACH;FMTTYPE=text/plain;ENCODING=BASE64;VALUE=BINARY:VGhlIHF1aWNrIGJyb3duIGZveAo=";
    let expected = StrProp {
        name: "ATTACH",
        value: "VGhlIHF1aWNrIGJyb3duIGZveAo=",
        parameters: vec![
            StrParam { name: "FMTTYPE", values: vec!["text/plain"] },
            StrParam { name: "ENCODING", values: vec!["BASE64"] },
            StrParam { name: "VALUE", values: vec!["BINARY"] },
        ],
    };
    as_expected(text, expected);
}
#[test]
fn vanilla() {
    let text = "FOO;BAR=baz:bex";
    let expected = StrProp {
        name: "FOO",
        value: "bex",
        parameters: vec![StrParam { name: "BAR", values: vec!["baz"] }],
    };
    as_expected(text, expected);
}
#[test]
fn non_ascii() {
    let text = r#"FOO;BAR=íííí,,"óu":béééé"#;
    let expected = StrProp {
        name: "FOO",
        value: "béééé",
        parameters: vec![StrParam { name: "BAR", values: vec!["íííí", "", "óu"] }],
    };
    as_expected(text, expected);
}
#[test]
fn comma_comma_comma() {
    let text = "FOO;BAR=,,,:bex";
    let expected = StrProp {
        name: "FOO",
        value: "bex",
        parameters: vec![StrParam { name: "BAR", values: vec!["", "", "", ""] }],
    };
    as_expected(text, expected);
}
#[test]
fn empty_param_value_list() {
    let text = "FOO;BAR=:bex";
    let expected = StrProp {
        name: "FOO",
        value: "bex",
        parameters: vec![StrParam { name: "BAR", values: vec![""] }],
    };
    as_expected(text, expected);
}

// Comparisons
fn compare(text: &[u8]) {
    let _ = equivalent_from_bytes(text);
}
#[test]
fn two_a() {
    compare("2;a=:".as_bytes());
}
#[test]
fn two_a_quote_lt() {
    compare(r#"2;a="<":"#.as_bytes());
}
#[test]
fn two_a_quote_lt_and_a_trailing_quote() {
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
#[test]
fn six_t_null() {
    compare(b"6:t\0");
}
#[test]
fn z_a_qmark() {
    compare(b"z;A\xDF");
}
#[test]
fn b_z_sema_2_comma_semi() {
    compare(b"z;2=,;");
}
#[test]
fn two_a_empty_quote_semi() {
    let text = "2;A=\"\";";
    compare(text.as_bytes());
}
#[test]
fn two_a_empty_quote_semi_ctrl_a() {
    let text = "2;A=\"\"\x01".as_bytes();
    compare(text);
}
#[test]
fn z_semi_bad() {
    let text = b"z;\xD9".as_bytes();
    compare(text);
}
#[test]
fn z_semi_z_qqq() {
    let text = r#"z;z=""""#;
    eprintln!("{text}");
    compare(text.as_bytes());
}
