use std::io;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum CalendarError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Error in content starting at input line {0}: {1}")]
    AtLine(usize, ParseError),
}

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Error, Debug)]
#[error("{0}")]
pub struct ParseError(pub String);
macro_rules! err {
    ($msg:literal $(,)?) => { ParseError(format!($msg))

    };
    ($fmt:expr, $($arg:tt)*) => {
       ParseError(format!($fmt, $($arg)*))
    };
}
pub(crate) use err;

#[derive(Error, Debug, PartialEq)]
#[error("{reason} at column {valid_up_to}")]
pub struct PreparseError {
    pub(crate) reason: &'static str,
    pub(crate) valid_up_to: usize,
    pub(crate) error_len: Option<u8>,
}
