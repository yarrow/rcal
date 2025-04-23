use jiff::{
    SignedDuration, Timestamp, Zoned,
    civil::{Date, DateTime, Time},
};
use nonempty::NonEmpty;

use crate::rrule::RRule;

pub enum PropertyValue {
    Binary(Vec<u8>),
    Boolean(bool),
    CalAddress(String),
    Date(NonEmpty<Date>),
    DateTime(NonEmpty<DateTime>),
    DateTimeUtc(NonEmpty<Timestamp>),
    DateTimeZoned(NonEmpty<Zoned>),
    Duration(NonEmpty<SignedDuration>),
    Float(NonEmpty<f64>),
    Period((Timestamp, Timestamp)), // Is it always Timestamp? Do we need to remember start/end vs start/duration?
    Recur(Box<RRule>),
    Text(NonEmpty<String>),
    Time(NonEmpty<Time>),
    Uri(String),
    UtcOffset(SignedDuration),
}
