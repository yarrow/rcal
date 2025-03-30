#![cfg_attr(
    debug_assertions,
    allow(dead_code, unused_imports, unused_macros, unused_mut, unused_variables)
)]

pub mod error;
pub mod rrule;
pub use jiff::civil::Weekday;
