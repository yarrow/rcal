//! # Errors
//! `RRule` uses a very simple base error: just static string error message(s).
//! We reply on `winnow::error::ParseError` to keep track of the position of
//! the error.
use winnow::error::{AddContext, ErrMode, ParserError};
use winnow::stream::Stream;

pub(crate) type ModalResult<T> = winnow::ModalResult<T, RRuleError>;

/// Our `Error` type is modeled on `winnow::error::ContextError`
#[derive(Debug)]
pub struct RRuleError {
    message: Vec<&'static str>,
    cause: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}
impl RRuleError {
    /// Create an error with message and cause
    #[must_use]
    #[inline]
    pub fn new(
        msg: &'static str,
        cause: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        Self { message: vec![msg], cause }
    }
    /// Create an `ErrMode::Cut` error with message and cause
    #[must_use]
    #[inline]
    pub fn cut(
        msg: &'static str,
        cause: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> ErrMode<Self> {
        ErrMode::Cut(Self { message: vec![msg], cause })
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

impl Clone for RRuleError {
    fn clone(&self) -> Self {
        Self {
            message: self.message.clone(),
            cause: self.cause.as_ref().map(|e| e.to_string().into()),
        }
    }
}

/// Default error is empty
impl Default for RRuleError {
    #[inline]
    fn default() -> Self {
        Self { message: Vec::new(), cause: None }
    }
}

impl AddContext<&[u8], &'static str> for RRuleError {
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

impl ParserError<&[u8]> for RRuleError {
    type Inner = Self;

    #[inline]
    fn from_input(_input: &&[u8]) -> Self {
        Self::default()
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn into_inner(self) -> Result<Self::Inner, Self> {
        Ok(self)
    }
}
