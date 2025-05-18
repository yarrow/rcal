use crate::error::{PreparseError, Problem, Segment};
use regex::bytes::Regex;
use std::{mem, str, sync::LazyLock};

use super::{LocStr, Param, Prop};

static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\A[a-zA-Z0-9-]+"#).unwrap());
static VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\A[^\x00-\x08\x0A-\x1F\x7F]*").unwrap());
static TEXT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\A[^\x00-\x08\x0A-\x1F\x7F";:,]*"#).unwrap());
static QUOTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\A"[^\x00-\x08\x0A-\x1F\x7F"]*"#).unwrap());

pub fn regex_preparse(v: &[u8]) -> Result<Prop, PreparseError> {
    use Problem::*;
    #[allow(clippy::cast_possible_truncation)]
    match pre_preparse(v) {
        Ok(value) => Ok(value),
        Err(mut err) => {
            if v.is_empty() {
                err.problem = EmptyContentLine;
            } else if err.valid_up_to < v.len() {
                let byte = v[err.valid_up_to];
                if byte.is_ascii_control() && byte != b'\t' {
                    err.problem = ControlCharacter;
                } else if byte > 127 {
                    let rest = &v[err.valid_up_to..];
                    if let Err(utf) = str::from_utf8(rest) {
                        if utf.valid_up_to() == 0 {
                            err.problem = Utf8Error(utf.error_len().map(|len| len as u8));
                        }
                    }
                }
            }
            Err(err)
        }
    }
}
fn pre_preparse(mut v: &[u8]) -> Result<Prop, PreparseError> {
    use Problem::*;
    use Segment::*;

    let mut start = 0;
    let mut param_name;
    let mut param_values = Vec::<LocStr>::new();
    let mut parameters = Vec::<Param>::new();

    macro_rules! loc_str {
        ($m: ident) => {
            LocStr {
                loc: start,
                val: unsafe { str::from_utf8_unchecked(v.get_unchecked(..$m.end())) },
            }
        };
    }
    macro_rules! err {
        ($seg: expr, $prob: expr, $valid: expr) => {
            return Err(PreparseError { segment: $seg, problem: $prob, valid_up_to: $valid })
        };
    }
    macro_rules! advance_by {
        ($n: expr) => {
            start += $n;
            v = unsafe { v.get_unchecked($n..) }
        };
    }
    macro_rules! advance_past {
        ($m: ident) => {
            advance_by!($m.end())
        };
    }
    macro_rules! consume {
        ($ch: literal) => {
            !v.is_empty() && v[0] == $ch && {
                advance_by!(1);
                true
            }
        };
        ($ch: literal else $bail: stmt) => {
            if !consume!($ch) {
                $bail
            }
        };
    }

    let Some(m) = NAME.find(v) else {
        err!(PropertyName, Empty, start);
    };
    let property_name = loc_str!(m);
    advance_past!(m);
    if v.is_empty() || !matches!(v[0], b';' | b':') {
        err!(PropertyName, Unterminated, start);
    }

    let mut segment = PropertyValue;
    while consume!(b';') {
        let Some(m) = NAME.find(v) else { err!(ParamName, Empty, start) };
        param_name = loc_str!(m);
        advance_past!(m);
        consume!(b'=' else err!(ParamName, Unterminated, start));

        segment = ParamValue;
        loop {
            if let Some(m) = QUOTED.find(v) {
                let quote = m.end();
                if quote < v.len() && v[quote] == b'"' {
                    let loc = start + 1;
                    let val = unsafe { str::from_utf8_unchecked(v.get_unchecked(1..quote)) };
                    param_values.push(LocStr { loc, val });
                    advance_by!(quote + 1);
                } else {
                    err!(ParamValue, UnclosedQuote, start + quote)
                }
            } else {
                let m = TEXT.find(v).unwrap(); // SAFETY: TEXT matches the empty string
                param_values.push(loc_str!(m));
                advance_past!(m);
            }
            consume!(b',' else break);
        }
        parameters
            .push(Param { name: mem::take(&mut param_name), values: mem::take(&mut param_values) });
    }

    if consume!(b':') {
        let m = VALUE.find(v).unwrap(); // SAFETY: VALUE matches the empty string
        if m.end() == v.len() {
            return Ok(Prop { name: property_name, value: loc_str!(m), parameters });
        } else {
            err!(PropertyValue, Unterminated, start + m.end());
        }
    } else if segment == ParamValue && !v.is_empty() {
        let problem = if v[0] == b'"' { DoubleQuote } else { Unterminated };
        err!(segment, problem, start);
    } else {
        err!(PropertyValue, Empty, start);
    }
}
