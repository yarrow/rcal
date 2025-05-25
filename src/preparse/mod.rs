//! Operations related to RFC 5545 validation.
use crate::error::{PreparseError, Problem};
use std::str;
mod with_regex;
pub use with_regex::regex_preparse;
mod byte_by_byte;
pub use byte_by_byte::preparse;

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

// Content lines must be valid UTF8 and contain no ASCII control characters. When the initial
// problem found is something like "Property name isn't followed by a colon or semicolon" —
// because the property name was followed by a control character or invalid UTF8 — then the
// diagnose_character_errors function will record the presence of a control character or invalid
// UTF8 in err.problem
fn diagnose_character_errors(mut err: PreparseError, v: &[u8]) -> Result<Prop, PreparseError> {
    let bad_place = err.valid_up_to;
    let remaining = &v[bad_place..];
    if remaining.is_empty() {
        return Err(err);
    }
    let this_byte = remaining[0];
    if this_byte.is_ascii_control() && this_byte != b'\t' {
        err.problem = Problem::ControlCharacter;
    } else if this_byte >= 128 {
        #[allow(clippy::cast_possible_truncation)]
        if let Err(utf8) = str::from_utf8(remaining) {
            if utf8.valid_up_to() == 0 {
                err.problem = Problem::Utf8Error(utf8.error_len().map(|len| len as u8));
            }
        }
    }
    Err(err)
}

#[cfg(test)]
mod tests;
