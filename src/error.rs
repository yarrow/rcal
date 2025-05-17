use std::io;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum CalendarError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Error in content starting at input line {0}: {1}")]
    AtLine(usize, PreparseError),
    #[error(transparent)]
    Name(#[from] NameError),
}

#[derive(Error, Debug)]
pub enum XPreparseError {
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    #[error("ASCII Control Character at column {0}")]
    ControlCharacter(usize),
    #[error("Error at column {valid_up_to}: {reason}")]
    LineError { reason: &'static str, valid_up_to: usize },
}
pub type NameResult<T> = Result<T, NameError>;

#[derive(Error, Debug)]
#[error("{0}")]
pub struct NameError(pub String);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Segment {
    PropertyName,
    PropertyValue,
    ParamName,
    ParamValue,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Problem {
    ControlCharacter,
    Utf8Error { valid_up_to: usize, error_len: Option<u8> },
    DoubleQuote,
    EmptyContentLine,
    Empty,
    Unterminated,
}
#[derive(Clone, Debug, Error, PartialEq)]
#[error("FIXME at column {valid_up_to}")] // FIXME!
pub struct PreparseError {
    pub(crate) segment: Segment,
    pub(crate) problem: Problem,
    pub(crate) valid_up_to: usize,
    pub(crate) error_len: Option<usize>,
}
impl PreparseError {
    pub fn reason(&self) -> &'static str {
        use Problem::*;
        match self.problem {
            ControlCharacter => CONTROL_CHARACTER,
            Utf8Error { valid_up_to: _, error_len: _ } => UTF8_ERROR,
            DoubleQuote => UNEXPECTED_DOUBLE_QUOTE,
            EmptyContentLine => EMPTY_CONTENT_LINE,
            Empty => match self.segment {
                Segment::ParamName => NO_PARAM_NAME,
                Segment::PropertyName => NO_PROPERTY_NAME,
                Segment::PropertyValue => NO_PROPERTY_VALUE,
                _ => unimplemented!(),
            },
            Unterminated => match self.segment {
                Segment::ParamName => NO_EQUAL_SIGN,
                Segment::PropertyName => NO_COLON_OR_SEMICOLON,
                Segment::ParamValue => NO_COMMA_ETC,
                _ => unimplemented!(),
            },
        }
    }
    pub fn is_utf8_error(&self) -> bool {
        matches!(self.problem, Problem::Utf8Error { valid_up_to: _, error_len: _ })
    }
    pub fn is_control_char_error(&self) -> bool {
        matches!(self.problem, Problem::ControlCharacter)
    }
}
pub(crate) const CONTROL_CHARACTER: &str =
    "ASCII control characters are not allowed, except tab (\\t)";
pub(crate) const EMPTY_CONTENT_LINE: &str = "Empty content line";
pub(crate) const NO_COLON_OR_SEMICOLON: &str =
    "Property name must be followed by a colon (:) or a semicolon (;)";
pub(crate) const NO_COMMA_ETC: &str =
    "Parameter value must be followed by a comma (,) or colon (:) or semicolon(;)";
pub(crate) const NO_EQUAL_SIGN: &str = "Parameter name must be follow by an equal sign (=)";
pub(crate) const NO_PARAM_NAME: &str = "No parameter name after semicolon";
pub(crate) const NO_PROPERTY_NAME: &str = "Content line doesn't start with a property name";
pub(crate) const NO_PROPERTY_VALUE: &str = "Content line has no property value";
pub(crate) const UNEXPECTED_DOUBLE_QUOTE: &str = r#"unexpected double quote (")"#;
pub(crate) const UTF8_ERROR: &str = "UTF8 error";
