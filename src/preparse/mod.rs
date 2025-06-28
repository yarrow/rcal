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

// Content lines must be valid UTF8 and contain no ASCII control characters except tabs.
//
//`invalid_character_or` ensures that invalid UTF8 is reported even if other errors occur
// earlier in `v`, and if `v` is valid UTF8, ensures that invalid ASCII control characters are
//reported even if parsing errors occur earlier.

fn invalid_character_or(err: PreparseError, v: &[u8]) -> PreparseError {
    #[allow(clippy::cast_possible_truncation)]
    if let Err(utf8) = str::from_utf8(v) {
        PreparseError {
            problem: Problem::Utf8Error(utf8.error_len().map(|len| len as u8)),
            valid_up_to: utf8.valid_up_to(),
        }
    } else if let Some(bad_ctrl) = v.iter().position(|&b| b.is_ascii_control() && b != b'\t') {
        PreparseError { problem: Problem::ControlCharacter, valid_up_to: bad_ctrl }
    } else {
        err
    }
}

#[cfg(test)]
mod tests;
