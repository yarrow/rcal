use crate::error::{PreparseError, Problem, Segment};
use regex::bytes::Regex;
use std::{mem, str, sync::LazyLock};

use super::{LocStr, Param, Prop, tweak_err};

static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\A[a-zA-Z0-9-]+"#).unwrap());
static VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\A[^\x00-\x08\x0A-\x1F\x7F]*$").unwrap());
static TEXT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\A[^\x00-\x08\x0A-\x1F\x7F";:,]*"#).unwrap());
static QUOTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\A"[^\x00-\x08\x0A-\x1F\x7F"]*""#).unwrap());

pub fn regex_preparse(v: &[u8]) -> Result<Prop, PreparseError> {
    match pre_preparse(v) {
        Ok(value) => Ok(value),
        Err(err) => tweak_err(err, v),
    }
}
pub fn pre_preparse(mut v: &[u8]) -> Result<Prop, PreparseError> {
    let mut start = 0;
    let mut segment = Segment::PropertyName;
    macro_rules! loc_str {
        ($m: ident) => {
            LocStr {
                loc: start,
                val: unsafe { str::from_utf8_unchecked(v.get_unchecked(..$m.end())) },
            }
        };
    }
    macro_rules! loc_quoted_str {
        ($m: ident) => {
            LocStr {
                loc: start + 1,
                val: unsafe { str::from_utf8_unchecked(v.get_unchecked(1..$m.end() - 1)) },
            }
        };
    }
    macro_rules! rfc_err {
        ($problem: expr) => {
            return Err(PreparseError {
                segment,
                problem: $problem,
                valid_up_to: start,
                error_len: None,
            })
        };
    }
    macro_rules! advance_start_by {
        ($n: expr) => {
            start += $n;
            v = unsafe { v.get_unchecked($n..) };
        };
    }
    macro_rules! consume {
        ($ch: literal) => {
            !v.is_empty() && v[0] == $ch && {
                advance_start_by!(1);
                true
            }
        };
        ($ch: literal else $bail: stmt) => {
            if !consume!($ch) {
                $bail
            }
        };
    }

    let mut param_name = LocStr::default();
    let mut param_values = Vec::<LocStr>::new();
    let mut parameters = Vec::new();
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

    let Some(m) = NAME.find(v) else {
        rfc_err!(if v.is_empty() { Problem::EmptyContentLine } else { Problem::Empty })
    };
    let property_name = loc_str!(m);
    advance_start_by!(m.end());

    while consume!(b';') {
        segment = Segment::ParamName;
        finish_parameter!();

        let Some(m) = NAME.find(v) else { rfc_err!(Problem::Empty) };
        param_name = loc_str!(m);
        advance_start_by!(m.end());

        consume!(b'=' else rfc_err!(Problem::Unterminated));
        loop {
            if let Some(m) = QUOTED.find(v) {
                param_values.push(loc_quoted_str!(m));
                advance_start_by!(m.end());
            } else if let Some(m) = TEXT.find(v) {
                param_values.push(loc_str!(m));
                advance_start_by!(m.end());
            } else {
                param_values.push(LocStr { loc: start, val: "" });
            }
            consume!(b',' else break);
        }
    }

    if consume!(b':') {
        segment = Segment::PropertyValue;
        let Some(m) = VALUE.find(v) else { rfc_err!(Problem::Empty) };
        finish_parameter!();
        Ok(Prop { name: property_name, value: loc_str!(m), parameters })
    } else {
        let problem;
        (segment, problem) = match (v.first(), param_name.val.is_empty()) {
            (None, _) => (Segment::PropertyValue, Problem::Empty),
            (Some(&b'"'), true) => (Segment::PropertyName, Problem::Unterminated),
            (Some(&b'"'), false) => (Segment::ParamValue, Problem::DoubleQuote),
            (_, true) => (Segment::PropertyName, Problem::Unterminated),
            (_, false) => (Segment::ParamValue, Problem::Unterminated),
        };
        rfc_err!(problem);
    }
}
