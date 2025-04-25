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
