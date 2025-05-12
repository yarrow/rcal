//! Operations related to RFC 5545 validation.

use crate::error::PreparseError;
use std::ops::Range;
use std::str::from_utf8;
use std::{mem, str};

const NO_PROPERTY_NAME: &str = "Content line doesn't start with a property name";
const NO_COLON_OR_SEMICOLON: &str =
    "Property name must be followed by a colon (:) or a semicolon (;)";
const NO_COMMA_ETC: &str =
    "Parameter value must be followed by a comma (,) or colon (:) or semicolon(;)";
const NO_PARAM_NAME: &str = "No parameter name after semicolon";
const NO_EQUAL_SIGN: &str = "Parameter name must be follow by an equal sign (=)";
const UNEXPECTED_DOUBLE_QUOTE: &str = r#"unexpected double quote (")"#;
const CONTROL_CHARACTER_IN_PARAMETER: &str =
    "Found a control character while scanning a parameter value";
const CONTROL_CHARACTER_IN_PROPERTY: &str =
    "Found a control character while scanning the property value";
const UTF8_ERROR: &str = "UTF8 error";
const NO_PROPERTY_VALUE: &str = "Content line has no property value";

/// A located `str`: a substring of a larger string, along with its location in that string.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LocStr<'a> {
    loc: usize,
    val: &'a str,
}
#[derive(Debug, PartialEq)]
pub struct Param<'a> {
    name: LocStr<'a>,
    values: Vec<LocStr<'a>>,
}
#[derive(Debug, PartialEq)]
pub struct Prop<'a> {
    name: LocStr<'a>,
    parameters: Vec<Param<'a>>,
    value: LocStr<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    ParamName,
    StartParamValue,
    ParamText,
    ParamQuoted,
    Value,
    NonAscii,
}
pub fn preparse<'a>(v: &'a [u8]) -> Result<Prop<'a>, PreparseError> {
    // INVARIANT: `v[start..index]` is a valid UTF8 string. (Implies `start <= index && index <= v.len()`)
    //
    // We only modify `start` by setting it to the currrent value of `index`, which establishes
    // the invariant (since `v[start..index]` is then the empty string).
    //
    // Since the concatenation of two valid UTF8 strings is also valid, if `v[index]`
    // is ASCII we can increment `index` while maintaining the invariant. For a multi-byte
    // UTF8 code point we may briefly break the invariant while checking that it is indeed
    // a valid code point, but if it's not valid we'll return an error and if it is valid
    // we'll restore the invariant by setting `index` to the byte past the end of the code
    // point.
    let mut index = 0;
    let mut start = index;

    // Given the invariant, `loc_str!(start, index)` always produces a `LocStr{loc, val}` where
    // `v[loc]` is the start of a UTF8 code point and `val` is a valid UTF8 string — again as long
    // as we don't call `loc_str!(start, index)` in the middle of scanning a multi-byte UTF8 code
    // point.
    //
    // And we never call `loc_str!` except as `loc_str!(start, index)`
    macro_rules! loc_str {
        ($start: ident, $index: ident) => {{
            debug_assert!(str::from_utf8(&v[$start..$index]).is_ok());
            LocStr {
                loc: $start,
                val: unsafe { str::from_utf8_unchecked(v.get_unchecked($start..$index)) },
            }
        }};
    }

    // Return an error: the input doesn't correspond to the basic grammar in RFC 5545 § 3.1
    macro_rules! rfc_err {
        ($reason: expr) => {
            return Err(PreparseError { reason: $reason, valid_up_to: index, error_len: None })
        };
    }

    let len = v.len();
    if len == 0 {
        rfc_err!("Empty content line")
    }
    let mut state = State::NonAscii;
    while index < len {
        match v[index] {
            b'-' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => index += 1, // Maintains the invariant since
            b':' => {
                state = State::Value;
                break;
            }
            b';' => {
                state = State::ParamName;
                break;
            }
            _ => rfc_err!(if index == 0 { NO_PROPERTY_NAME } else { NO_COLON_OR_SEMICOLON }),
        };
    }
    debug_assert!(matches!(state, State::Value | State::ParamName));

    if index == 0 {
        rfc_err!(NO_PROPERTY_NAME);
    }
    let mut param_name = LocStr::default();
    let mut param_values = Vec::<LocStr>::new();
    let mut parameters = Vec::<Param<'a>>::new();
    let property_name = loc_str!(start, index);
    index += 1;

    let mut start = index;
    macro_rules! finish_parameter {
        () => {
            if !param_name.val.is_empty() {
                parameters.push(Param {
                    name: mem::take(&mut param_name),
                    values: mem::take(&mut param_values),
                });
            }
        };
    }
    let mut old_state = state;
    'outer: while index < len {
        let mut this_byte = v[index];

        // Set `this_byte` to the next byte of `v` or break the 'outer loop if there are none left
        macro_rules! next_byte_or_finish {
            () => {{
                index += 1;
                if index >= len {
                    break 'outer;
                }
                this_byte = v[index];
            }};
        }
        // Discard the next byte by advancing `index`, set `start` to the advanced `index`,
        // and go to the head of the 'outer loop
        macro_rules! change_state_to {
            ($new_state: expr) => {{
                index += 1;
                start = index;
                state = $new_state;
                continue 'outer;
            }};
        }

        // If the current byte isn't ASCII, ensure that we handle that (with code stolen from the
        // Rust compiler), while remembering the current state in order to return to it. In effect,
        // perform a manual subroutine call.
        macro_rules! handle_non_ascii {
            () => {
                if this_byte >= 128 {
                    old_state = state;
                    state = State::NonAscii;
                    continue 'outer;
                }
            };
        }

        match state {
            State::ParamName => {
                finish_parameter!();
                loop {
                    match this_byte {
                        b'-' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => next_byte_or_finish!(),
                        b'=' => {
                            if index == start {
                                rfc_err!(NO_PARAM_NAME)
                            }
                            param_name = loc_str!(start, index);
                            change_state_to!(State::StartParamValue);
                        }
                        _ => rfc_err!(if index == start { NO_PARAM_NAME } else { NO_EQUAL_SIGN }),
                    };
                }
            }
            State::StartParamValue => {
                if v[index] == b'"' {
                    index += 1;
                    state = State::ParamQuoted;
                } else {
                    state = State::ParamText
                }
                continue 'outer;
            }
            State::ParamText => loop {
                handle_non_ascii!();
                match this_byte {
                    b'\t' | b' '..b'"' | b'#'..b',' | b'-'..b':' | b'<'..127 => {
                        next_byte_or_finish!()
                    }
                    b'"' => rfc_err!(UNEXPECTED_DOUBLE_QUOTE),
                    b',' => {
                        param_values.push(loc_str!(start, index));
                        change_state_to!(State::StartParamValue);
                    }
                    b':' => {
                        param_values.push(loc_str!(start, index));
                        change_state_to!(State::Value);
                    }
                    b';' => {
                        param_values.push(loc_str!(start, index));
                        change_state_to!(State::ParamName);
                    }
                    _ => rfc_err!(CONTROL_CHARACTER_IN_PARAMETER),
                }
            },
            State::ParamQuoted => loop {
                handle_non_ascii!();
                match this_byte {
                    b'\t' | b' '..b'"' | b'#'..127 => next_byte_or_finish!(),
                    b'"' => {
                        start += 1;
                        param_values.push(loc_str!(start, index));
                        next_byte_or_finish!();
                        match this_byte {
                            b',' => change_state_to!(State::StartParamValue),
                            b':' => change_state_to!(State::Value),
                            b';' => change_state_to!(State::ParamName),
                            _ => rfc_err!(NO_COMMA_ETC),
                        }
                    }
                    _ => rfc_err!(CONTROL_CHARACTER_IN_PARAMETER),
                }
            },
            State::Value => {
                finish_parameter!();
                loop {
                    handle_non_ascii!();
                    match this_byte {
                        b'\t' | b' '..127 => next_byte_or_finish!(),
                        _ => rfc_err!(CONTROL_CHARACTER_IN_PROPERTY),
                    }
                }
            }
            State::NonAscii => {
                loop {
                    state = old_state; // Return from whence we came at the end of the loop

                    // Taken from `run_utf8_validation` in
                    // lib/rustlib/src/rust/library/core/src/str/validations.rs
                    let old_offset = index;
                    macro_rules! next {
                        () => {{
                            index += 1;
                            // we needed data, but there was none: error!
                            if index >= len {
                                utf8_err!(None)
                            }
                            v[index]
                        }};
                    }
                    macro_rules! utf8_err {
                        ($error_len: expr) => {
                            return Err(PreparseError {
                                reason: UTF8_ERROR,
                                valid_up_to: old_offset,
                                error_len: $error_len,
                            })
                        };
                    }

                    let w = utf8_char_width(this_byte);
                    // 2-byte encoding is for codepoints  \u{0080} to  \u{07ff}
                    //        first  C2 80        last DF BF
                    // 3-byte encoding is for codepoints  \u{0800} to  \u{ffff}
                    //        first  E0 A0 80     last EF BF BF
                    //   excluding surrogates codepoints  \u{d800} to  \u{dfff}
                    //               ED A0 80 to       ED BF BF
                    // 4-byte encoding is for codepoints \u{10000} to \u{10ffff}
                    //        first  F0 90 80 80  last F4 8F BF BF
                    //
                    // Use the UTF-8 syntax from the RFC
                    //
                    // https://tools.ietf.org/html/rfc3629
                    // UTF8-1      = %x00-7F
                    // UTF8-2      = %xC2-DF UTF8-tail
                    // UTF8-3      = %xE0 %xA0-BF UTF8-tail / %xE1-EC 2( UTF8-tail ) /
                    //               %xED %x80-9F UTF8-tail / %xEE-EF 2( UTF8-tail )
                    // UTF8-4      = %xF0 %x90-BF 2( UTF8-tail ) / %xF1-F3 3( UTF8-tail ) /
                    //               %xF4 %x80-8F 2( UTF8-tail )
                    match w {
                        2 => {
                            if next!() as i8 >= -64 {
                                utf8_err!(Some(1))
                            }
                        }
                        3 => {
                            match (this_byte, next!()) {
                                (0xE0, 0xA0..=0xBF)
                                | (0xE1..=0xEC, 0x80..=0xBF)
                                | (0xED, 0x80..=0x9F)
                                | (0xEE..=0xEF, 0x80..=0xBF) => {}
                                _ => utf8_err!(Some(1)),
                            }
                            if next!() as i8 >= -64 {
                                utf8_err!(Some(2))
                            }
                        }
                        4 => {
                            match (this_byte, next!()) {
                                (0xF0, 0x90..=0xBF)
                                | (0xF1..=0xF3, 0x80..=0xBF)
                                | (0xF4, 0x80..=0x8F) => {}
                                _ => utf8_err!(Some(1)),
                            }
                            if next!() as i8 >= -64 {
                                utf8_err!(Some(2))
                            }
                            if next!() as i8 >= -64 {
                                utf8_err!(Some(3))
                            }
                        }
                        _ => utf8_err!(Some(1)),
                    }
                    next_byte_or_finish!();
                    if this_byte < 128 {
                        break;
                    }
                }
            }
        }
    }
    debug_assert!(state != State::Value || index == v.len());

    match state {
        State::Value => Ok(Prop { name: property_name, parameters, value: loc_str!(start, index) }),
        State::NonAscii => panic!("BUG: should be impossible to exit loop with state == NonAscii"),
        State::ParamName | State::StartParamValue | State::ParamText | State::ParamQuoted => {
            rfc_err!(NO_PROPERTY_VALUE)
        }
    }
}
// Taken from lib/rustlib/src/rust/library/core/src/str/validations.rs
// (which itself credits ietf.org in the commented link)
//
// https://tools.ietf.org/html/rfc3629
const UTF8_CHAR_WIDTH: &[u8; 256] = &[
    // 1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 1
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 2
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 3
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 4
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 5
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 6
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 7
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 8
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 9
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // A
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // B
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // C
    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // D
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // E
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F
];

// Taken from lib/rustlib/src/rust/library/core/src/str/validations.rs
#[must_use]
#[inline]
fn utf8_char_width(b: u8) -> usize {
    UTF8_CHAR_WIDTH[b as usize] as usize
}

#[cfg(test)]
mod test {
    use super::*;
    use bstr::BString;
    use pretty_assertions::assert_eq;

    fn validate(s: &str) -> Result<(), PreparseError> {
        preparse(s.as_bytes()).map(|_| ())
    }
    fn error_for(s: &str) -> &'static str {
        validate(s).unwrap_err().reason
    }

    #[test]
    fn minimal() {
        assert!(validate("-:").is_ok())
    }

    #[test]
    fn attach() {
        assert_eq!(
            validate(
                "ATTACH;FMTTYPE=text/plain;ENCODING=BASE64;VALUE=BINARY:VGhlIHF1aWNrIGJyb3duIGZveCBqdW1wcyBvdmVyIHRoZSBsYXp5IGRvZy4"
            ),
            Ok(())
        );
    }

    #[test]
    fn no_property_value() {
        assert_eq!(validate("A;B=").unwrap_err().reason, NO_PROPERTY_VALUE);
    }

    #[test]
    fn quotes_allow_punctutation_in_values() {
        assert_eq!(error_for(r#"A;B=",C=:""#), NO_PROPERTY_VALUE);
        assert_eq!(error_for(r#"A;B=":C=:""#), NO_PROPERTY_VALUE);
        assert_eq!(error_for(r#"A;B=";C=:""#), NO_PROPERTY_VALUE);
    }

    #[test]
    fn property_name_required() {
        assert_eq!(error_for(":foo"), NO_PROPERTY_NAME);
        assert_eq!(error_for("/foo"), NO_PROPERTY_NAME);
    }
    #[test]
    fn parameter_name_required() {
        assert_eq!(error_for("Foo;=bar:"), NO_PARAM_NAME);
        assert_eq!(error_for("Foo;/:"), NO_PARAM_NAME);
    }

    #[test]
    fn must_utf8() {
        let mut bad = BString::from("FOO:bá");
        let len = bad.len();
        bad[len - 1] = b'a';
        assert_eq!(preparse(bad.as_slice()).unwrap_err().reason, UTF8_ERROR);
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
    fn delocate<'a>(prop: Prop<'a>) -> StrProp<'a> {
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
    fn parse<'a>(text: &'a str) -> StrProp<'a> {
        delocate(preparse(text.as_bytes()).unwrap())
    }

    #[test]
    fn minimal_value() {
        let text = "-:";
        let expected = StrProp { name: "-", value: "", parameters: Vec::new() };
        let result = parse(text);
        assert_eq!(result, expected);
    }

    #[test]
    fn vanilla() {
        let text = "FOO;BAR=baz:bex";
        let expected = StrProp {
            name: "FOO",
            value: "bex",
            parameters: vec![StrParam { name: "BAR", values: vec!["baz"] }],
        };
        let result = parse(text);
        assert_eq!(result, expected);
    }

    #[test]
    fn non_ascii() {
        let text = r#"FOO;BAR=íííí,,"óu":béééé"#;
        let expected = StrProp {
            name: "FOO",
            value: "béééé",
            parameters: vec![StrParam { name: "BAR", values: vec!["íííí", "", "óu"] }],
        };
        let result = parse(text);
        assert_eq!(result, expected);
    }

    #[test]
    fn comma_comma_comma() {
        let text = "FOO;BAR=,,,:bex";
        let expected = StrProp {
            name: "FOO",
            value: "bex",
            parameters: vec![StrParam { name: "BAR", values: vec!["", "", "", ""] }],
        };
        let result = parse(text);
        assert_eq!(result, expected);
    }

    #[test]
    fn empty_param_value_list() {
        let text = "FOO;BAR=:bex";
        let expected = StrProp {
            name: "FOO",
            value: "bex",
            parameters: vec![StrParam { name: "BAR", values: vec![""] }],
        };
        let result = parse(text);
        assert_eq!(result, expected);
        // does what!?
    }
}
