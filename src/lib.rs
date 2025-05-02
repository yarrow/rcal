#![cfg_attr(
    debug_assertions,
    allow(dead_code, unused_imports, unused_macros, unused_mut, unused_variables)
)]

pub mod property;
pub mod rrule;
pub mod rrule_error;
pub use jiff::civil::Weekday;
pub use property::PropertyValue;
pub mod error;
pub(crate) use error::{ParseError, ParseResult, err};
pub mod names;
pub mod parameter;
pub mod unfolded;
