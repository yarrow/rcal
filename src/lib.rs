#![cfg_attr(
    debug_assertions,
    allow(dead_code, unused_imports, unused_macros, unused_mut, unused_variables)
)]

pub mod rrule;
pub mod rrule_error;
pub mod values;
pub use jiff::civil::Weekday;
pub mod error;
pub mod names;
pub mod parameters;
pub mod properties;
pub mod unfolded;
