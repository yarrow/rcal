//! Operations related to RFC 5545 validation.
use crate::error::{PreparseError, Problem};
use std::str;
#[cfg(feature = "cautious")]
mod with_regex;
#[cfg(feature = "cautious")]
pub use with_regex::cautious_preparse;
#[cfg(feature = "bold")]
mod byte_by_byte;
#[cfg(feature = "bold")]
pub use byte_by_byte::bold_preparse;

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

trait ToPreparseError {
    fn to_preparse_error(&self) -> PreparseError;
}
impl ToPreparseError for str::Utf8Error {
    fn to_preparse_error(&self) -> PreparseError {
        #[allow(clippy::cast_possible_truncation)]
        PreparseError {
            problem: Problem::Utf8Error(self.error_len().map(|len| len as u8)),
            valid_up_to: self.valid_up_to(),
        }
    }
}

fn control_character_or(err: PreparseError, v: &[u8]) -> PreparseError {
    if matches!(err.problem, Problem::Utf8Error(_)) || err.valid_up_to == v.len() {
        return err;
    }
    let b = v[err.valid_up_to];
    if b.is_ascii_control() && b != b'\t' {
        PreparseError { problem: Problem::ControlCharacter, valid_up_to: err.valid_up_to }
    } else {
        err
    }
}

#[cfg(test)]
mod tests;
