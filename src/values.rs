use jiff::{SignedDuration, Timestamp, Zoned, civil};
use nonempty::NonEmpty;

use crate::rrule::RRule;

pub enum PropertyValue {
    Binary(Vec<u8>),
    Boolean(bool),
    CalAddress(String),
    Date(NonEmpty<civil::Date>),
    DateTime(NonEmpty<civil::DateTime>),
    DateTimeUtc(NonEmpty<Timestamp>),
    DateTimeZoned(NonEmpty<Zoned>),
    Duration(NonEmpty<SignedDuration>),
    Float(NonEmpty<f64>),
    Period((Timestamp, Timestamp)), // Is it always Timestamp? Do we need to remember start/end vs start/duration?
    Recur(Box<RRule>),
    Text(NonEmpty<String>),
    Time(NonEmpty<civil::Time>),
    Uri(String),
    UtcOffset(SignedDuration),
}
