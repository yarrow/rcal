use crate::error::{EMPTY_CONTENT_LINE, PreparseError, Problem, Segment};
use regex::Regex;
use std::{mem, str, sync::LazyLock};

use super::{LocStr, Param, Prop, ToPreparseError, control_character_or};
static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\A[a-zA-Z0-9-]+"#).unwrap());
static VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\A[^\x00-\x08\x0A-\x1F\x7F]*").unwrap());
static TEXT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\A[^\x00-\x08\x0A-\x1F\x7F";:,]*"#).unwrap());
static QUOTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\A"[^\x00-\x08\x0A-\x1F\x7F"]*"#).unwrap());

pub fn cautious_preparse(v: &[u8]) -> Result<Prop<'_>, PreparseError> {
    if v.is_empty() {
        return Err(EMPTY_CONTENT_LINE);
    }
    match inner_preparse(v) {
        Ok(value) => Ok(value),
        Err(err) => Err(control_character_or(err, v)),
    }
}
fn inner_preparse(v: &[u8]) -> Result<Prop<'_>, PreparseError> {
    use Problem::*;
    use Segment::*;

    let mut v = match str::from_utf8(v) {
        Ok(v) => v,
        Err(utf8_err) => return Err(utf8_err.to_preparse_error()),
    };
    let mut start = 0;
    let mut param_name;
    let mut param_values = Vec::<LocStr>::new();
    let mut parameters = Vec::<Param>::new();

    macro_rules! loc_str {
        ($m: ident) => {
            LocStr { loc: start, val: &v[..$m.end()] }
        };
    }
    macro_rules! err {
        ($prob: expr, $valid: expr) => {
            return Err(PreparseError { problem: $prob, valid_up_to: $valid })
        };
    }
    macro_rules! advance_by {
        ($n: expr) => {
            start += $n;
            v = &v[$n..];
        };
    }
    macro_rules! advance_past {
        ($m: ident) => {
            advance_by!($m.end())
        };
    }
    macro_rules! consume {
        ($ch: literal) => {
            !v.is_empty() && v.as_bytes()[0] == $ch && {
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
        err!(Empty(PropertyName), start);
    };
    let property_name = loc_str!(m);
    advance_past!(m);
    if v.is_empty() || !matches!(v.as_bytes()[0], b';' | b':') {
        err!(Unterminated(PropertyName), start);
    }

    let mut segment = PropertyValue;
    while consume!(b';') {
        let Some(m) = NAME.find(v) else { err!(Empty(ParamName), start) };
        param_name = loc_str!(m);
        advance_past!(m);
        consume!(b'=' else err!(Unterminated(ParamName), start));

        segment = ParamValue;
        loop {
            if let Some(m) = QUOTED.find(v) {
                let quote = m.end();
                if quote < v.len() && v.as_bytes()[quote] == b'"' {
                    let loc = start + 1;
                    let val = &v[1..quote];
                    param_values.push(LocStr { loc, val });
                    advance_by!(quote + 1);
                } else {
                    err!(UnclosedQuote(ParamValue), start + quote)
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
            err!(Unterminated(PropertyValue), start + m.end());
        }
    } else if segment == ParamValue && !v.is_empty() {
        let problem = if v.as_bytes()[0] == b'"' {
            DoubleQuote(ParamValue)
        } else {
            Unterminated(ParamValue)
        };
        err!(problem, start);
    } else {
        err!(Empty(PropertyValue), start);
    }
}
