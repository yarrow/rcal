//! # Errors
//! `RRule` uses a very simple base error: just static string error message(s).
//! We reply on `winnow::error::ParseError` to keep track of the position of
//! the error.
use winnow::error::{AddContext, ErrMode, ParserError};
use winnow::stream::Stream;

pub(crate) type ModalResult<T> = winnow::ModalResult<T, Error>;

/// Our `Error` type is modeled on `winnow::error::ContextError`
#[derive(Debug)]
pub struct Error {
    message: Vec<&'static str>,
    cause: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}
impl Error {
    /// Create an empty error
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self { message: Vec::new(), cause: None }
    }
    /// Return the list of error messages
    #[must_use]
    #[inline]
    pub fn context(&self) -> Vec<&'static str> {
        self.message.clone()
    }

    /// The underlying [`std::error::Error`] (if any)  
    #[must_use]
    #[inline]
    pub fn cause(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        self.cause.as_deref()
    }
}

impl Clone for Error {
    fn clone(&self) -> Self {
        Self {
            message: self.message.clone(),
            cause: self.cause.as_ref().map(|e| e.to_string().into()),
        }
    }
}

impl Default for Error {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl AddContext<&[u8], &'static str> for Error {
    #[inline]
    fn add_context(
        mut self,
        _input: &&[u8],
        _token_start: &<&[u8] as Stream>::Checkpoint,
        context: &'static str,
    ) -> Self {
        self.message.push(context);
        self
    }
}

impl ParserError<&[u8]> for Error {
    type Inner = Self;

    #[inline]
    fn from_input(_input: &&[u8]) -> Self {
        Self::new()
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn into_inner(self) -> Result<Self::Inner, Self> {
        Ok(self)
    }
}
