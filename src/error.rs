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
impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Segment::*;
        let display = match self {
            PropertyName => "property name",
            PropertyValue => "property value",
            ParamName => "parameter name",
            ParamValue => "parameter value",
        };
        write!(f, "{display}",)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Problem {
    Utf8Error(Option<u8>),
    ControlCharacter,
    EmptyContentLine,
    DoubleQuote(Segment),
    UnclosedQuote(Segment),
    Empty(Segment),
    Unterminated(Segment),
}
#[derive(Clone, Debug, Error, PartialEq)]
pub struct PreparseError {
    pub(crate) problem: Problem,
    pub(crate) valid_up_to: usize,
}
pub(crate) const EMPTY_CONTENT_LINE: PreparseError =
    PreparseError { problem: Problem::EmptyContentLine, valid_up_to: 0 };

impl fmt::Display for PreparseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Problem::*;
        use Segment::*;
        let valid_up_to = self.valid_up_to;
        match self.problem {
            ControlCharacter => {
                write!(f, "invalid control character at index {valid_up_to}")
            }
            Utf8Error(error_len) => {
                if let Some(error_len) = error_len {
                    write!(
                        f,
                        "invalid utf-8 sequence of {error_len} bytes from index {valid_up_to}"
                    )
                } else {
                    write!(f, "incomplete utf-8 byte sequence from index {valid_up_to}")
                }
            }
            EmptyContentLine => write!(f, "content line is empty"),
            DoubleQuote(segment) => {
                write!(f, "unexpected double quote (\") in {segment} at index {valid_up_to}")
            }
            UnclosedQuote(segment) => {
                write!(f, "expected double quote (\") in {segment} at index {valid_up_to}")
            }
            Empty(segment) => match segment {
                ParamName => {
                    write!(f, "expected a {segment} after the semicolon (;) at index {valid_up_to}")
                }
                PropertyName => write!(f, "content line doesn't start with a {segment}"),
                PropertyValue => {
                    write!(f, "content line doesn't end with a {segment} — missing colon (:)?")
                }
                ParamValue => write!(
                    f,
                    "BUG: the error claims there's an empty {segment}, but {segment}s can be empty"
                ),
            },
            Unterminated(segment) => match segment {
                ParamName => write!(
                    f,
                    "expecting an equals sign (=) at index {valid_up_to}, after the {segment}"
                ),
                PropertyName => write!(
                    f,
                    "expecting a colon (:) or semicolon (;) at index {valid_up_to}, after the {segment}",
                ),
                ParamValue => write!(
                    f,
                    "expecting a a comma (,) or colon (:) or semicolon(;) at index {valid_up_to}, \
                    after the {segment}",
                ),
                PropertyValue => write!(
                    f,
                    "BUG: the error claims the {segment} is ended by an unexpected character, \
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
        let problems = [
            Utf8Error(None),
            ControlCharacter,
            EmptyContentLine,
            DoubleQuote(ParamValue),
            UnclosedQuote(ParamValue),
            Empty(PropertyName),
            Empty(PropertyValue),
            Empty(ParamName),
            Empty(ParamValue),
            Unterminated(PropertyName),
            Unterminated(PropertyValue),
            Unterminated(ParamName),
            Unterminated(ParamValue),
        ];
        for p in problems {
            let err = PreparseError { problem: p, valid_up_to: 0 };
            let message = err.to_string();
            let bad = message.find('\n').or_else(|| message.find("  "));
            if bad.is_some() {
                panic!("\n{err:?}\n{message}");
            }
        }
    }
}
