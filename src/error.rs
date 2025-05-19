use std::fmt;
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
    Utf8Error(Option<u8>),
    DoubleQuote,
    UnclosedQuote,
    EmptyContentLine,
    Empty,
    Unterminated,
}
#[derive(Clone, Debug, Error, PartialEq)]
pub struct PreparseError {
    pub(crate) segment: Segment,
    pub(crate) problem: Problem,
    pub(crate) valid_up_to: usize,
}
pub(crate) const EMPTY_CONTENT_LINE: PreparseError = PreparseError {
    segment: Segment::PropertyName,
    problem: Problem::EmptyContentLine,
    valid_up_to: 0,
};

impl fmt::Display for PreparseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Problem::*;
        use Segment::*;
        match self.problem {
            ControlCharacter => {
                write!(f, "invalid control character at index {}", self.valid_up_to)
            }
            Utf8Error(error_len) => {
                if let Some(error_len) = error_len {
                    write!(
                        f,
                        "invalid utf-8 sequence of {} bytes from index {}",
                        error_len, self.valid_up_to
                    )
                } else {
                    write!(f, "incomplete utf-8 byte sequence from index {}", self.valid_up_to)
                }
            }
            DoubleQuote => write!(f, "unexpected double quote (\") at index {}", self.valid_up_to),
            UnclosedQuote => write!(f, "expected double quote (\") at index {}", self.valid_up_to),
            EmptyContentLine => write!(f, "content line is empty"),
            Empty => match self.segment {
                ParamName => write!(f, "expected a parameter name after the semicolon (;)"),
                PropertyName => write!(f, "content line doesn't start with a property name"),
                PropertyValue => {
                    write!(f, "content line does end with a property value — missing colon (:)?")
                }
                ParamValue => write!(
                    f,
                    "BUG: the error claims there's an empty parameter value, but parameter values \
                    can be empty"
                ),
            },
            Unterminated => match self.segment {
                ParamName => write!(
                    f,
                    "expecting an equals sign (=) at index {}, after the parameter name",
                    self.valid_up_to
                ),
                PropertyName => write!(
                    f,
                    "expecting a colon (:) or semicolon (;) at index {}, after the property name",
                    self.valid_up_to
                ),
                ParamValue => write!(
                    f,
                    "expecting a a comma (,) or colon (:) or semicolon(;) at index {}, after the \
                    parameter value",
                    self.valid_up_to
                ),
                PropertyValue => write!(
                    f,
                    "BUG: the error claims the property value is ended by an unexpected character, \
                    but the only candidates (ASCII control characters and invalid utf8 sequences) \
                    have separate error messages"
                ),
            },
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_line_breaks() {
        // Make use I ended each broken line with a line feed (and have no extra spaces)
        use Problem::*;
        use Segment::*;
        let segments = [PropertyName, PropertyValue, ParamName, ParamValue];
        let problems = [
            ControlCharacter,
            Utf8Error(None),
            DoubleQuote,
            UnclosedQuote,
            EmptyContentLine,
            Empty,
            Unterminated,
        ];
        for p in problems {
            for s in segments {
                let err = PreparseError { segment: s, problem: p, valid_up_to: 0 };
                let message = err.to_string();
                let bad = message.find('\n').or_else(|| message.find("  "));
                if bad.is_some() {
                    panic!("\n{err:?}\n{message}");
                }
            }
        }
    }
}
