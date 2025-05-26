// RFC 5545 has multiple cases where a "good" ASCII character range has a one-character gap
#![allow(non_contiguous_range_endpoints)]
use super::{LocStr, Param, Prop, diagnose_character_errors};
use crate::error::{EMPTY_CONTENT_LINE, PreparseError, Problem, Segment};
use std::{mem, str};
// Return an error: the input doesn't correspond to the basic grammar in RFC 5545 § 3.1
macro_rules! rfc_err {
    ($segment: expr, $problem: expr, $index: ident) => {
        return Err(PreparseError { segment: $segment, problem: $problem, valid_up_to: $index })
    };
}

fn finish_parameter<'a>(
    parameters: &mut Vec<Param<'a>>,
    name: &mut LocStr<'a>,
    values: &mut Vec<LocStr<'a>>,
) {
    if !name.val.is_empty() {
        parameters.push(Param { name: mem::take(name), values: mem::take(values) });
    }
}

//SAFETY: `0 <= start && start <= index && index <= v.len()` and `v[start..index]` is valid UTF8
unsafe fn loc_str(v: &[u8], start: usize, index: usize) -> LocStr<'_> {
    debug_assert!(str::from_utf8(&v[start..index]).is_ok());
    LocStr { loc: start, val: unsafe { str::from_utf8_unchecked(v.get_unchecked(start..index)) } }
}
pub fn preparse<'a>(v: &'a [u8]) -> Result<Prop<'a>, PreparseError> {
    if v.is_empty() {
        return Err(EMPTY_CONTENT_LINE);
    }
    use Problem::*;

    // INVARIANT: `v[start..index]` is a valid UTF8 string. (Implies `start <= index && index <= v.len()`)
    // (The invariant implies that `loc_str(v, start, index)` is safe, and that is the only way we
    // call `loc_str`.)
    //
    // If `f` is one of the following functions:
    //  * rfc5545_name
    //  * param_text
    //  * param_quoted
    //  * property_value
    //  * handle_non_ascii
    //  We have `v[start..f(v, start)]` is a valid UTF8 string (assuming `start < v.len()`).
    //
    let (mut start, mut index) = (0, rfc5545_name(v, 0));

    // We only call `loc_str!` as `unsafe { loc_str(v, start, index)` — which is safe because given the
    // invariant, `unsafe { loc_str(start, index)` always produces a `LocStr{loc, val}` where `v[loc]`
    // is the start of a UTF8 code point and `val` is a valid UTF8 string. (Again, this is true
    // only as long as we don't call `unsafe { loc_str(start, index)` in the middle of scanning a
    // multi-byte UTF8 code point.)

    macro_rules! check_for_character_error {
        ($segment: expr, $problem: expr) => {{
            let problem = if $problem == Unterminated && index == start { Empty } else { $problem };
            return diagnose_character_errors(
                PreparseError { segment: $segment, problem, valid_up_to: index },
                v,
            );
        }};
    }

    let len = v.len();
    if index == 0 || index >= len || !matches!(v[index], b';' | b':') {
        check_for_character_error!(Segment::PropertyName, Unterminated)
    }

    let mut param_name = LocStr::default();
    let mut param_values = Vec::<LocStr>::new();
    let mut parameters = Vec::<Param<'a>>::new();
    let property_name = unsafe { loc_str(v, start, index) };

    'outer: while index < len && v[index] == b';' {
        finish_parameter(&mut parameters, &mut param_name, &mut param_values);
        (start, index) = (index + 1, rfc5545_name(v, index + 1));
        if index >= len {
            check_for_character_error!(Segment::ParamName, Unterminated);
        }
        match v[index] {
            b'=' => {
                if index == start {
                    rfc_err!(Segment::ParamName, Empty, index)
                }
                param_name = unsafe { loc_str(v, start, index) };
                (start, index) = (index + 1, index + 1);
            }
            _ => check_for_character_error!(Segment::ParamName, Unterminated),
        }
        while index < len {
            if v[index] == b'"' {
                (start, index) = (index + 1, param_quoted(v, index + 1)?);
                if index >= len {
                    rfc_err!(Segment::ParamValue, UnclosedQuote, index)
                }
                match v[index] {
                    b'"' => {
                        param_values.push(unsafe { loc_str(v, start, index) });
                        index += 1;
                    }
                    _ => rfc_err!(Segment::ParamValue, ControlCharacter, index),
                }
            } else {
                (start, index) = (start, param_text(v, start)?);
                param_values.push(unsafe { loc_str(v, start, index) });
            }
            if index >= len {
                break 'outer;
            }
            match v[index] {
                b',' => (index, start) = (index + 1, index + 1),
                b':' => break 'outer,
                b';' => break,
                b'"' => rfc_err!(Segment::ParamValue, DoubleQuote, index),
                _ => check_for_character_error!(Segment::ParamValue, Unterminated),
            }
        }
    }
    if index < len && v[index] == b':' {
        finish_parameter(&mut parameters, &mut param_name, &mut param_values);
        (start, index) = (index + 1, property_value(v, index + 1)?);
        Ok(Prop { name: property_name, parameters, value: unsafe { loc_str(v, start, index) } })
    } else {
        rfc_err!(Segment::PropertyValue, Empty, index);
    }
}

// SAFETY: `v[j..rfc5545_name(v, j)]`` is a valid UTF8 string because every byte in that range is
// an ASCII character
fn rfc5545_name(v: &[u8], mut index: usize) -> usize {
    let len = v.len();
    while index < len {
        match v[index] {
            b'-' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => index += 1,
            _ => break,
        }
    }
    index
}

fn param_text(v: &[u8], mut index: usize) -> Result<usize, PreparseError> {
    while index < v.len() {
        match v[index] {
            b'\t' | b' '..b'"' | b'#'..b',' | b'-'..b':' | b'<'..127 => index += 1,
            128.. => index = handle_non_ascii(v, Segment::ParamValue, index)?,
            _ => break,
        }
    }
    Ok(index)
}
fn param_quoted(v: &[u8], mut index: usize) -> Result<usize, PreparseError> {
    while index < v.len() {
        match v[index] {
            b'\t' | b' '..b'"' | b'#'..127 => index += 1,
            128.. => index = handle_non_ascii(v, Segment::ParamValue, index)?,
            _ => break,
        }
    }
    Ok(index)
}
fn property_value(v: &[u8], mut index: usize) -> Result<usize, PreparseError> {
    while index < v.len() {
        match v[index] {
            b'\t' | b' '..127 => index += 1,
            128.. => index = handle_non_ascii(v, Segment::PropertyValue, index)?,
            _ => rfc_err!(Segment::PropertyValue, Problem::ControlCharacter, index),
        }
    }
    Ok(index)
}

// Modeled after `run_utf8_validation` in
// lib/rustlib/src/rust/library/core/src/str/validations.rs
// Panics if `index >= v.len()`
#[allow(clippy::cast_possible_wrap, clippy::unnested_or_patterns)]
fn handle_non_ascii(v: &[u8], segment: Segment, mut index: usize) -> Result<usize, PreparseError> {
    let len = v.len();
    while index < len {
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

        let this_byte = v[index];
        match this_byte {
            b'\t' | b' '..b'"' | b'#'..b',' | b'-'..b':' | b'<'..127 => index += 1,

            128.. => {
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
                index += 1;
            }
            _ => break,
        }
    }
    Ok(index)
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
