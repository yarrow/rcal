use crate::error::PreparseError;
use regex::bytes::Regex;
use std::{mem, str, sync::LazyLock};

use super::{
    CONTROL_CHARACTER, EMPTY_CONTENT_LINE, NO_COLON_OR_SEMICOLON, NO_COMMA_ETC, NO_EQUAL_SIGN,
    NO_PARAM_NAME, NO_PROPERTY_NAME, NO_PROPERTY_VALUE, UNEXPECTED_DOUBLE_QUOTE, UTF8_ERROR,
};
use super::{LocStr, Param, Prop};

static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\A[a-zA-Z0-9-]+"#).unwrap());
static VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\A[^\x00-\x08\x0A-\x1F\x7F]*$").unwrap());
static TEXT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\A[^\x00-\x08\x0A-\x1F\x7F";:,]*"#).unwrap());
static QUOTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\A"[^\x00-\x08\x0A-\x1F\x7F"]*""#).unwrap());

pub fn regex_preparse(v: &[u8]) -> Result<Prop, PreparseError> {
    let is_non_tab_control = |index: usize| {
        if index >= v.len() { false } else { v[index].is_ascii_control() && v[index] != b'\t' }
    };
    match pre_preparse(v) {
        Ok(value) => Ok(value),
        Err(rfc_err) => {
            let mut err = rfc_err.clone();
            if str::from_utf8(v).is_err() {
                err.reason = UTF8_ERROR;
            } else if is_non_tab_control(rfc_err.valid_up_to + 1) {
                err.reason = CONTROL_CHARACTER;
            } else if is_non_tab_control(rfc_err.valid_up_to) {
                err.reason = CONTROL_CHARACTER;
                err.valid_up_to = err.valid_up_to.saturating_sub(1);
            }
            Err(err)
        }
    }
}
pub fn pre_preparse(mut v: &[u8]) -> Result<Prop, PreparseError> {
    let mut start = 0;
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
        ($reason: expr) => {
            return Err(PreparseError { reason: $reason, valid_up_to: start, error_len: None })
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
        rfc_err!(if v.is_empty() { EMPTY_CONTENT_LINE } else { NO_PROPERTY_NAME })
    };
    let property_name = loc_str!(m);
    advance_start_by!(m.end());

    while consume!(b';') {
        finish_parameter!();

        let Some(m) = NAME.find(v) else { rfc_err!(NO_PARAM_NAME) };
        param_name = loc_str!(m);
        advance_start_by!(m.end());

        consume!(b'=' else rfc_err!(NO_EQUAL_SIGN));

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
    finish_parameter!();

    if consume!(b':') {
        let Some(m) = VALUE.find(v) else { rfc_err!(NO_PROPERTY_VALUE) };
        Ok(Prop { name: property_name, value: loc_str!(m), parameters })
    } else {
        let reason = match v.first() {
            None => NO_PROPERTY_VALUE,
            Some(&b'"') => UNEXPECTED_DOUBLE_QUOTE,
            _ => {
                if parameters.is_empty() {
                    NO_COLON_OR_SEMICOLON
                } else {
                    NO_COMMA_ETC
                }
            }
        };
        rfc_err!(reason);
    }
}

#[cfg(test)]
use crate::preparse_tests;
#[cfg(test)]
preparse_tests!(regex_preparse);
