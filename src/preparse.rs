//! Operations related to RFC 5545 validation.
use crate::error::{PreparseError, Problem, Segment};
use std::{mem, str};
pub mod with_regex;

/// A located `str`: a substring of a larger string, along with its location in that string.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LocStr<'a> {
    pub loc: usize,
    pub(crate) val: &'a str,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Param<'a> {
    pub(crate) name: LocStr<'a>,
    pub(crate) values: Vec<LocStr<'a>>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Prop<'a> {
    pub name: LocStr<'a>,
    pub(crate) parameters: Vec<Param<'a>>,
    pub(crate) value: LocStr<'a>,
}

// Content lines must be UTF8 and contain no ASCII control characters. We give those requirements
// precedence over any other parsing errors.
fn detect_utf8_error(mut err: PreparseError, v: &[u8]) -> Result<Prop, PreparseError> {
    // If the position where we stopped parsing is the start of bad UTF8, report that as our `problem`
    let bad_place = err.valid_up_to;
    if bad_place < v.len() && v[bad_place] >= 128 {
        let remaining = &v[bad_place..];
        #[allow(clippy::cast_possible_truncation)]
        if let Err(utf8) = str::from_utf8(remaining) {
            if utf8.valid_up_to() == 0 {
                err.problem = Problem::Utf8Error(utf8.error_len().map(|len| len as u8));
            }
        }
    }
    Err(err)
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
#[allow(
    clippy::unnested_or_patterns,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::cast_possible_wrap,
    non_contiguous_range_endpoints
)]
pub fn preparse<'a>(v: &'a [u8]) -> Result<Prop<'a>, PreparseError> {
    // INVARIANT: `v[start..index]` is a valid UTF8 string. (Implies `start <= index && index <= v.len()`)
    //
    // We only modify `start` by setting it to the current value of `index`, which establishes
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

    let mut segment = Segment::PropertyName;
    let mut state = State::NonAscii; // Anything that's not `Value` or `ParamName`
    //
    // Return an error: the input doesn't correspond to the basic grammar in RFC 5545 § 3.1
    macro_rules! rfc_err {
        ($problem: expr) => {
            return Err(PreparseError { segment, problem: $problem, valid_up_to: index })
        };
    }

    macro_rules! check_for_utf8_error {
        ($problem: expr) => {
            return detect_utf8_error(
                PreparseError { segment, problem: $problem, valid_up_to: index },
                v,
            )
        };
    }

    let len = v.len();
    if len == 0 {
        rfc_err!(Problem::EmptyContentLine);
    }
    while index < len {
        match v[index] {
            b'-' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => index += 1, // Maintains the invariant since
            b':' => {
                segment = Segment::ParamValue;
                state = State::Value;
                break;
            }
            b';' => {
                segment = Segment::ParamName;
                state = State::ParamName;
                break;
            }
            ..b'\t' | 10..b' ' | 127 => {
                rfc_err!(Problem::ControlCharacter)
            }
            _ => check_for_utf8_error!(if index == 0 {
                Problem::Empty
            } else {
                Problem::Unterminated
            }),
        }
    }
    if index == 0 {
        segment = Segment::PropertyName;
        check_for_utf8_error!(Problem::Empty);
    }
    if segment == Segment::PropertyName {
        check_for_utf8_error!(if index == 0 { Problem::Empty } else { Problem::Unterminated });
    }
    let mut param_name = LocStr::default();
    let mut param_values = Vec::<LocStr>::new();
    let mut parameters = Vec::<Param<'a>>::new();
    let property_name = loc_str!(start, index);
    index += 1;

    start = index;
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
    let mut pending_quote = false;
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
                segment = Segment::ParamName;
                finish_parameter!();
                loop {
                    match this_byte {
                        b'-' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => next_byte_or_finish!(),
                        b'=' => {
                            if index == start {
                                rfc_err!(Problem::Empty)
                            }
                            param_name = loc_str!(start, index);
                            change_state_to!(State::StartParamValue);
                        }
                        ..b'\t' | 10..b' ' | 127 => {
                            rfc_err!(Problem::ControlCharacter)
                        }
                        _ => {
                            let problem =
                                if index == start { Problem::Empty } else { Problem::Unterminated };
                            check_for_utf8_error!(problem);
                        }
                    }
                }
            }
            State::StartParamValue => {
                segment = Segment::ParamValue;
                if v[index] == b'"' {
                    index += 1;
                    state = State::ParamQuoted;
                    pending_quote = true;
                } else {
                    state = State::ParamText;
                }
            }
            State::ParamText => loop {
                handle_non_ascii!();
                match this_byte {
                    b'\t' | b' '..b'"' | b'#'..b',' | b'-'..b':' | b'<'..127 => {
                        next_byte_or_finish!();
                    }
                    b'"' => rfc_err!(Problem::DoubleQuote),
                    b',' => {
                        param_values.push(loc_str!(start, index));
                        change_state_to!(State::StartParamValue);
                    }
                    b':' => {
                        param_values.push(loc_str!(start, index));
                        finish_parameter!();
                        segment = Segment::PropertyValue;
                        change_state_to!(State::Value);
                    }
                    b';' => {
                        param_values.push(loc_str!(start, index));
                        segment = Segment::ParamName;
                        change_state_to!(State::ParamName);
                    }
                    _ => rfc_err!(Problem::ControlCharacter),
                }
            },
            State::ParamQuoted => loop {
                handle_non_ascii!();
                match this_byte {
                    b'\t' | b' '..b'"' | b'#'..127 => next_byte_or_finish!(),
                    b'"' => {
                        start += 1;
                        pending_quote = false;
                        param_values.push(loc_str!(start, index));
                        next_byte_or_finish!();
                        match this_byte {
                            b',' => change_state_to!(State::StartParamValue),
                            b':' => {
                                finish_parameter!();
                                segment = Segment::ParamValue;
                                change_state_to!(State::Value)
                            }
                            b';' => change_state_to!(State::ParamName),
                            ..b'\t' | 10..b' ' | 127 => {
                                rfc_err!(Problem::ControlCharacter)
                            }
                            b'"' => rfc_err!(Problem::DoubleQuote),
                            _ => check_for_utf8_error!(Problem::Unterminated),
                        }
                    }
                    _ => rfc_err!(Problem::ControlCharacter),
                }
            },
            State::Value => {
                segment = Segment::PropertyValue;
                finish_parameter!();
                loop {
                    handle_non_ascii!();
                    match this_byte {
                        b'\t' | b' '..127 => next_byte_or_finish!(),
                        _ => rfc_err!(Problem::ControlCharacter),
                    }
                }
            }
            State::NonAscii => {
                loop {
                    state = old_state; // Return from whence we came at the end of the loop

                    // Taken from `run_utf8_validation` in
                    // lib/rustlib/src/rust/library/core/src/str/validations.rs
                    let old_offset = index;
                    macro_rules! utf8_err {
                        ($error_len: expr) => {
                            return Err(PreparseError {
                                segment,
                                problem: Problem::Utf8Error($error_len),
                                valid_up_to: old_offset,
                            })
                        };
                    }
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
        State::ParamName => {
            segment = Segment::ParamName;
            if index == start { rfc_err!(Problem::Empty) } else { rfc_err!(Problem::Unterminated) }
        }
        State::ParamQuoted => {
            if pending_quote {
                rfc_err!(Problem::UnclosedQuote)
            } else {
                segment = Segment::PropertyValue;
                rfc_err!(Problem::Empty);
            }
        }
        State::StartParamValue | State::ParamText => {
            segment = Segment::PropertyValue;
            rfc_err!(Problem::Empty)
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
mod tests;
