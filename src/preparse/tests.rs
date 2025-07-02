#![cfg(all(feature = "cautious", feature = "bold"))]
use super::*;
use crate::error::{Problem, Segment};
use Problem::*;
use Segment::*;
use bstr::{BString, ByteSlice};
use pretty_assertions::assert_eq;

fn equivalent_from_bytes(text: &[u8]) -> Result<Prop, PreparseError> {
    let bold = bold_preparse(text);
    let cautious = cautious_preparse(text);
    assert_eq!(
        bold,
        cautious,
        "bold!=cautious, text: {:?}\n bold {:?}\ncautious {:?}",
        text.as_bstr(),
        bold,
        cautious
    );
    bold
}
fn equivalent(text: &str) -> Result<Prop, PreparseError> {
    let bold = bold_preparse(text.as_bytes());
    let cautious = cautious_preparse(text.as_bytes());
    assert_eq!(bold, cautious, "bold!=cautious, text: {text}");
    bold
}
fn err_for(text: &str) -> Problem {
    let err = equivalent(text).unwrap_err();
    err.problem
}
fn err_from_bytes(text: &[u8]) -> Problem {
    let err = equivalent_from_bytes(text).unwrap_err();
    err.problem
}

fn parse(text: &str) -> StrProp<'_> {
    delocate(&equivalent(text).unwrap())
}

fn err_is(text: &str, expected: Problem) {
    assert_eq!(err_for(text), expected, "text: |{text}|");
}
#[test]
fn property_name_only() {
    err_is("A", Unterminated(PropertyName));
}
#[test]
fn property_name_semicolon_only() {
    err_is("A;", Empty(ParamName));
}
#[test]
fn no_property_value() {
    err_is("A;B=", Empty(PropertyValue));
    err_is("A;B=c", Empty(PropertyValue));
}
#[test]
fn quotes_allow_punctuation_in_values() {
    err_is(r#"A;B=",C=:""#, Empty(PropertyValue));
    err_is(r#"A;B=":C=:""#, Empty(PropertyValue));
    err_is(r#"A;B=";C=:""#, Empty(PropertyValue));
}
#[test]
fn forbid_embedded_dquotes() {
    err_is(r#"A;B=ab"c":val"#, DoubleQuote(ParamValue));
}
#[test]
fn forbid_space_after_ending_dquote() {
    err_is(r#"A;B="c" ,"d":val"#, Unterminated(ParamValue));
}
#[test]
fn forbid_dquote_after_ending_dquote() {
    err_is(r#"A;B="c"","d":val"#, DoubleQuote(ParamValue));
}
#[test]
fn forbid_control_character_after_ending_dquote() {
    let mut text = BString::from(r#"A;B="c" ,"d":val"#);
    text[7] = 3;
    assert_eq!(err_from_bytes(&text), ControlCharacter);
}
#[test]
fn property_name_required() {
    err_is(":foo", Empty(PropertyName));
    err_is("/foo", Empty(PropertyName));
}
#[test]
fn forbid_empty_content_line() {
    err_is("", EmptyContentLine);
}
#[test]
fn parameter_name_required() {
    err_is("Foo;=bar:", Empty(ParamName));
    err_is("Foo;/:", Empty(ParamName));
}
#[test]
fn must_be_utf8_len_2() {
    let mut bad = BString::from("FOO:bá");
    //let mut bad = BString::from("abc𒀁");
    let len = bad.len();
    bad[len - 1] = b'a';
    assert_eq!(err_from_bytes(bad.as_slice()), Utf8Error(Some(1)), "text: {:?}", bad);
}
#[test]
fn must_be_utf8_len_4() {
    let mut bad = BString::from("abc𒀁");
    let len = bad.len();
    bad[len - 2] = b'a';
    assert_eq!(err_from_bytes(bad.as_slice()), Utf8Error(Some(2)), "text: {:?}", bad);
}

#[test]
fn fuzz_says_this_is_slow_but_i_dont_know_why() {
    let bad = b"R\xc7F=;6=;A=;B=;A=\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\
              \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\
              \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\
              \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff;A=;4=;A=;A=;B=;A6;";
    assert_eq!(err_from_bytes(bad.as_slice()), Utf8Error(Some(1)));
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
