#![allow(non_upper_case_globals)]

use jiff::{
    Zoned,
    civil::{Date, DateTime},
};

use std::num::NonZeroI8;
use std::ops::RangeInclusive;
use std::str::FromStr;

use paste::paste;

use crate::Weekday;
use bstr::{B, ByteSlice};
use memchr::memchr;

use winnow::ascii::{Caseless, Int, crlf, dec_int, dec_uint};
use winnow::combinator::{alt, cut_err, fail, opt, separated};
use winnow::error::{AddContext, ErrMode, ParserError};
use winnow::stream::Stream;
use winnow::{self, Parser};
type ModalResult<T> = winnow::ModalResult<T, RRuleError>;

#[derive(Debug)]
pub struct RRuleError {
    message: Vec<&'static str>,
    cause: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}
impl RRuleError {
    #[inline]
    pub fn new() -> Self {
        Self {
            message: Vec::new(),
            cause: None,
        }
    }

    #[inline]
    pub fn context(&self) -> Vec<&'static str> {
        self.message.clone()
    }

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

impl Default for RRuleError {
    #[inline]
    fn default() -> Self {
        Self::new()
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
    fn from_input(input: &&[u8]) -> Self {
        Self::new()
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn into_inner(self) -> Result<Self::Inner, Self> {
        Ok(self)
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
struct RRule {
    freq: Frequency,
    count: Option<u32>,
    until: Option<Tagged>,
    interval: Option<usize>,
    by_second: Vec<u8>,
    by_minute: Vec<u8>,
    by_hour: Vec<u8>,
    by_day: Vec<SomeWeekdays>,
    by_month_day: Vec<i8>,
    by_year_day: Vec<i16>,
    by_week_no: Vec<i8>,
    by_month: Vec<u8>,
    by_set_pos: Vec<i16>,
    wk_st: Option<Weekday>,
}
type SomeWeekdays = (Option<NonZeroI8>, Weekday);

// We derive Default only because that makes is easier to handle the `freq` field,
// which unlike the others is not optional.
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Frequency {
    Secondly,
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    #[default]
    Yearly,
}

//==============================================================================
fn frequency(input: &mut &[u8]) -> ModalResult<Frequency> {
    use Frequency::*;
    cut_err(alt((
        Caseless(B("Secondly")).value(Secondly),
        Caseless(B("Minutely")).value(Minutely),
        Caseless(B("Hourly")).value(Hourly),
        Caseless(B("Daily")).value(Daily),
        Caseless(B("Weekly")).value(Weekly),
        Caseless(B("Monthly")).value(Monthly),
        Caseless(B("Yearly")).value(Yearly),
    )))
    .parse_next(input)
}

#[derive(Debug, Clone, PartialEq)]
enum Tagged {
    CivilDate(Date),
    CivilDateTime(DateTime),
    ZonedDate(Zoned),
    ZonedDateTime(Zoned),
}

fn weekday(input: &mut &[u8]) -> ModalResult<Weekday> {
    use Weekday::*;
    cut_err(alt((
        Caseless(B("SU")).value(Sunday),
        Caseless(B("MO")).value(Monday),
        Caseless(B("TU")).value(Tuesday),
        Caseless(B("WE")).value(Wednesday),
        Caseless(B("TH")).value(Thursday),
        Caseless(B("FR")).value(Friday),
        Caseless(B("SA")).value(Saturday),
    )))
    .parse_next(input)
}

#[derive(Clone)]
struct U8list {
    tag: &'static str,
    range: RangeInclusive<u8>,
}
impl U8list {
    const fn new(tag: &'static str, range: RangeInclusive<u8>) -> Self {
        U8list { tag, range }
    }
}
impl Parser<&[u8], Vec<u8>, ErrMode<RRuleError>> for U8list {
    fn parse_next(&mut self, input: &mut &[u8]) -> ResultRRule<Vec<u8>> {
        let item = dec_uint::<&[u8], u8, ErrMode<RRuleError>>
            .context(self.tag)
            .verify(|n| self.range.contains(n));
        separated(1.., cut_err(item), b',').parse_next(input)
    }
}

//==============================================================================
macro_rules! too_many {
    ($name:ident) => {
        paste! {
            concat!("RRule can have at most one ", stringify!([<$name:upper>]))
        }
    };
}
macro_rules! constants_for {
    ($name:ident) => {
        paste! {
        // We can't use, say, "FREQ".as_bytes() in pattern matching, but we can create
        // an equivalent constant and use that.
            const [<$name:upper _as_bytes>]: &[u8] = stringify!([<$name:upper>]).as_bytes();
        }
    };
}
macro_rules! u8_tag {
    ($name:ident, $min:literal, $max:literal) => {
        paste! {
            concat!(stringify!($name:upper), " takes a list of numbers from ", $min, " to ", $max)
        }
    };
}
macro_rules! u8_list {
    ($name:ident, $min:literal, $max:literal) => {
        U8list::new(u8_tag!($name, $min, $max), $min..=$max)
    };
}
//==============================================================================
const FREQ_needs_Frequency: &str = "FREQ takes a frequency, from SECONDLY to YEARLY";
const Expected_equal_sign: &str = "Expected a component name followed by an equal sign (=)";
const Bad_usize: &str = "Expected an unsigned integer";
const Unknown_component: &str = "Unrecognized RRule component";
const FREQ_required: &str = "RRule must have a FREQ component";

const FREQ_as_bytes: &[u8] = "FREQ".as_bytes();
const Too_many_FREQs: &str = "RRule must have exactly one FREQ component; found multiple";
constants_for!(WkSt);
constants_for!(Count);
constants_for!(Interval);
constants_for!(BySecond);

type ResultRRule<T> = ModalResult<T>;
fn parse_rrule(input: &mut &[u8]) -> ResultRRule<RRule> {
    macro_rules! fail {
        ($why:expr) => {
            return fail.context($why).parse_next(input)
        };
    }
    macro_rules! get_single {
        ($name:ident, $place:expr, $parser:expr, $too_many:expr) => {
            if $place.is_none() {
                $place = Some($parser.parse_next(input)?)
            } else {
                fail!($too_many)
            }
        };
        ($name:ident, $place:expr, $parser:expr) => {
            get_single!($name, $place, $parser, too_many!($name))
        };
    }
    macro_rules! get_u8_list {
        ($name:ident, $place:expr, $min:literal, $max:literal) => {
            if $place.is_empty() {
                $place = U8list::new(u8_tag!($name, $min, $max), $min..=$max).parse_next(input)?;
            } else {
                fail!(too_many!($name))
            }
        };
    }

    let mut freq = None;
    let mut rrule = RRule::default();
    let mut name: Vec<u8>;
    // Every RRule line must end in CRLF, so we use that to trigger end-of-parse
    while let Err(_) = crlf::<&[u8], RRuleError>.parse_next(input) {
        // Extract the component name into 'name' and resume parsing after the equal sign
        let Some(eq) = memchr(b'=', input) else {
            fail!(Expected_equal_sign);
        };
        name = input[0..eq].to_vec();
        name.make_ascii_uppercase();
        *input = &input[eq + 1..];

        match &name[..] {
            FREQ_as_bytes => get_single!(Freq, freq, frequency, Too_many_FREQs),
            COUNT_as_bytes => get_single!(Count, rrule.count, dec_uint.context(Bad_usize)),
            INTERVAL_as_bytes => get_single!(Interval, rrule.interval, dec_uint.context(Bad_usize)),
            BYSECOND_as_bytes => get_u8_list!(BySecond, rrule.by_second, 0, 60),

            WKST_as_bytes => get_single!(WkSt, rrule.wk_st, weekday),
            _ => fail!(Unknown_component),
        }
        // Components are separated by semicolons
        if input.len() > 0 && input[0] == b';' {
            *input = &input[1..];
        }
    }

    match freq {
        None => fail!(FREQ_required),
        Some(f) => rrule.freq = f,
    }

    Ok(rrule)
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};

    macro_rules! options {
        ($freq:ident $(,$field:ident : $value:expr)*) => {
            RRule{
                freq: Frequency::$freq
                $(,$field: Some($value))*
                , ..Default::default()
            }
        };
    }
    macro_rules! vecs {
        ($freq:ident $(,$field:ident : $value:expr)*) => {
            RRule{
                freq: Frequency::$freq
                $(,$field: $value)*
                , ..Default::default()
            }
        };
    }

    #[test]
    fn test_parse_rrule_ok() {
        let ok_cases = [
            ("FREQ=SECONDLY\r\n", options!(Secondly)),
            ("count=0;FREQ=SECONDLY\r\n", options!(Secondly, count: 0)),
            (
                "INTERVAL=0;FREQ=SECONDLY\r\n",
                options!(Secondly, interval: 0),
            ),
            (
                "count=0;FREQ=SECONDLY;WkSt=WE\r\n",
                options!(Secondly, count: 0, wk_st: Weekday::Wednesday),
            ),
            (
                "BYSECOND=0,60,9;FREQ=hourly\r\n",
                vecs!(Hourly, by_second: vec![0,60,9]),
            ),
        ];
        for case in ok_cases {
            assert_eq!(
                parse_rrule.parse_peek(B(&case.0)).unwrap(),
                (B(""), case.clone().1),
                "Case: {}",
                case.0
            );
        }
    }
    #[test]
    fn test_parse_rrule_errors() {
        let error_cases = [
            ("\r\n", FREQ_required),
            ("", Expected_equal_sign),
            ("Freq=Yearly", Expected_equal_sign),
            ("Foo=bar", Unknown_component),
            ("Freq=Yearly;FREQ=Monthly\r\n", Too_many_FREQs),
            ("Freq=Yearly;WksT=MO;wkst=SU\r\n", too_many!(WkSt)),
            ("Freq=Yearly;Count=0;COUNT=4\r\n", too_many!(Count)),
            ("Freq=Yearly;Count=-1\r\n", Bad_usize),
            ("Freq=Yearly;Interval=0;INTERVAL=4\r\n", too_many!(Interval)),
            ("Freq=Yearly;Interval=-1\r\n", Bad_usize),
        ];
        for case in error_cases {
            let Err(err) = parse_rrule.parse_peek(B(&case.0)) else {
                panic!("No error for {case:?}")
            };
            let err = err.into_inner().unwrap();
            let context = err.context();
            assert_eq!(
                err.context(),
                vec![case.1],
                "Unexpected error for {case:?}:\n{err:?}\n"
            );
        }
    }
}
