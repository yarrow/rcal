use jiff::{
    Zoned,
    civil::{Date, DateTime},
};

use std::num::NonZeroI8;
use std::ops::RangeInclusive;

use paste::paste;

use crate::Weekday;
use bstr::B;
use memchr::memchr;

use winnow::ascii::{Caseless, Int, crlf, dec_int, dec_uint};
use winnow::combinator::{alt, cut_err, fail, separated};
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
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self { message: Vec::new(), cause: None }
    }

    #[must_use]
    #[inline]
    pub fn context(&self) -> Vec<&'static str> {
        self.message.clone()
    }

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
    fn from_input(_input: &&[u8]) -> Self {
        Self::new()
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn into_inner(self) -> Result<Self::Inner, Self> {
        Ok(self)
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct RRule {
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
        fail.context(tag::FREQ_needs_Frequency),
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
        fail.context(tag::Expected_day_abbreviation),
    )))
    .parse_next(input)
}

// Certain RRule components (BySecond, ByMinute, ByHour, and ByMonth) take as
// values a list of indices of the relevant period: `ByMonth=1,9`, for instance,
// refers to January and September. We call a list of such indices an IndexList
//
// Other components (ByMonthDay, ByYearDay, ByWeekNo, and BySetPost) take as
// values a list of offsets into some other period. `ByMonthDay=9,-1`, for
// instance, means the ninth and the last day of the month(s) it applies to.
// We call a list of such offsets an OffsetList.

struct IndexList {
    tag: &'static str,
    range: RangeInclusive<u8>,
}
impl IndexList {
    const fn new(tag: &'static str, range: RangeInclusive<u8>) -> Self {
        IndexList { tag, range }
    }
}
impl Parser<&[u8], Vec<u8>, ErrMode<RRuleError>> for IndexList {
    fn parse_next(&mut self, input: &mut &[u8]) -> ModalResult<Vec<u8>> {
        let item = dec_uint::<&[u8], u8, ErrMode<RRuleError>>
            .context(self.tag)
            .verify(|n| self.range.contains(n));
        match separated(1.., cut_err(item), b',').parse_next(input) {
            Ok(value) => Ok(value),
            Err(_) => cut_err(fail.context(self.tag)).parse_next(input),
        }
    }
}

#[derive(Clone)]
struct OffsetList<N: Int + PartialOrd + Default> {
    tag: &'static str,
    range: RangeInclusive<N>,
}
impl<N: Int + PartialOrd + Default> OffsetList<N> {
    const fn new(tag: &'static str, range: RangeInclusive<N>) -> Self {
        Self { tag, range }
    }
}
impl<N: Int + PartialOrd + Default> Parser<&[u8], Vec<N>, ErrMode<RRuleError>> for OffsetList<N> {
    fn parse_next(&mut self, input: &mut &[u8]) -> ModalResult<Vec<N>> {
        let zero = N::default();
        let item = dec_int::<&[u8], N, ErrMode<RRuleError>>
            .context(self.tag)
            .verify(|n: &N| *n != zero && self.range.contains(n));
        match separated(1.., cut_err(item), b',').parse_next(input) {
            Ok(value) => Ok(value),
            Err(_) => cut_err(fail.context(self.tag)).parse_next(input),
        }
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
macro_rules! index_tag {
    ($name:ident, $min:literal, $max:literal) => {
        paste! {
            concat!(stringify!([<$name:upper>]), " takes a list of numbers from ", $min, " to ", $max)
        }
    };
}
macro_rules! offset_tag {
    ($name:ident, $min:literal, $max:literal) => {
        paste! {
            concat!(stringify!([<$name:upper>]), " takes a list of nonzero numbers from ", $min, " to ", $max)
        }
    };
}
//==============================================================================
#[allow(non_upper_case_globals)]
mod tag {

    pub(super) const Expected_day_abbreviation: &str =
        "Expected a day-of-week abbreviation: SU, MO, TU, WE, TH, FR, or SA";
    pub(super) const FREQ_needs_Frequency: &str = "FREQ takes a frequency, from SECONDLY to YEARLY";
    pub(super) const Expected_equal_sign: &str =
        "Expected a component name followed by an equal sign (=)";
    pub(super) const Bad_usize: &str = "Expected an unsigned integer";
    pub(super) const Unknown_component: &str = "Unrecognized RRule component";
    pub(super) const FREQ_required: &str = "RRule must have a FREQ component";
    pub(super) const Too_many_FREQs: &str =
        "RRule must have exactly one FREQ component; found multiple";
}
pub fn parse_rrule(input: &mut &[u8]) -> ModalResult<RRule> {
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
    macro_rules! get_index_list {
        ($name:ident, $place:expr, $min:literal, $max:literal) => {
            if $place.is_empty() {
                $place =
                    IndexList::new(index_tag!($name, $min, $max), $min..=$max).parse_next(input)?;
            } else {
                fail!(too_many!($name))
            }
        };
    }

    macro_rules! get_offset_list {
        ($name:ident<$N:ident>, $place:expr, $min:literal, $max:literal) => {
            if $place.is_empty() {
                $place = OffsetList::<$N>::new(index_tag!($name, $min, $max), $min..=$max)
                    .parse_next(input)?;
            } else {
                fail!(too_many!($name))
            }
        };
    }

    let mut freq = None;
    let mut rrule = RRule::default();
    let mut name: Vec<u8>;
    // Every RRule line must end in CRLF, so we use that to trigger end-of-parse
    while crlf::<&[u8], RRuleError>.parse_next(input).is_err() {
        // Extract the component name into 'name' and resume parsing after the equal sign
        let Some(eq) = memchr(b'=', input) else {
            fail!(tag::Expected_equal_sign);
        };
        name = input[0..eq].to_vec();
        name.make_ascii_uppercase();
        *input = &input[eq + 1..];

        const FREQ: &[u8] = "FREQ".as_bytes();
        const COUNT: &[u8] = "COUNT".as_bytes();
        const INTERVAL: &[u8] = "INTERVAL".as_bytes();
        const BYSECOND: &[u8] = "BYSECOND".as_bytes();
        const BYMINUTE: &[u8] = "BYMINUTE".as_bytes();
        const BYHOUR: &[u8] = "BYHOUR".as_bytes();
        const BYMONTH: &[u8] = "BYMONTH".as_bytes();
        const BYMONTHDAY: &[u8] = "BYMONTHDAY".as_bytes();
        const WKST: &[u8] = "WKST".as_bytes();
        match &name[..] {
            FREQ => get_single!(Freq, freq, frequency, tag::Too_many_FREQs),
            COUNT => get_single!(Count, rrule.count, dec_uint.context(tag::Bad_usize)),
            INTERVAL => get_single!(Interval, rrule.interval, dec_uint.context(tag::Bad_usize)),
            BYSECOND => get_index_list!(BySecond, rrule.by_second, 0, 60),
            BYMINUTE => get_index_list!(ByMinute, rrule.by_minute, 0, 59),
            BYHOUR => get_index_list!(ByHour, rrule.by_hour, 0, 23),
            BYMONTHDAY => get_offset_list!(ByMonthDay<i8>, rrule.by_month_day, -31, 31),
            BYMONTH => get_index_list!(ByMonth, rrule.by_month, 1, 12),
            WKST => get_single!(WkSt, rrule.wk_st, weekday),
            _ => fail!(tag::Unknown_component),
        }
        // Components are separated by semicolons
        if input.first() == Some(&b';') {
            *input = &input[1..];
        }
    }

    match freq {
        None => fail!(tag::FREQ_required),
        Some(f) => rrule.freq = f,
    }

    Ok(rrule)
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn test_parse_rrule_ok() {
        macro_rules! rrule {
            ($freq:ident $(,$field:ident : $value:expr)*) => {
                RRule{
                    freq: Frequency::$freq
                    $(,$field: $value)*
                    , ..Default::default()
                }
            };
        }
        let ok_cases = [
            ("FREQ=SECONDLY\r\n", rrule!(Secondly)),
            ("count=0;FREQ=SECONDLY\r\n", rrule!(Secondly, count: Some(0))),
            ("INTERVAL=0;FREQ=SECONDLY\r\n", rrule!(Secondly, interval: Some(0))),
            (
                "count=0;FREQ=SECONDLY;WkSt=WE\r\n",
                rrule!(Secondly, count: Some(0), wk_st: Some(Weekday::Wednesday)),
            ),
            ("BYSECOND=0,60,9;FREQ=hourly\r\n", rrule!(Hourly, by_second: vec![0,60,9])),
            ("BYMINUTE=0,59,9;FREQ=hourly\r\n", rrule!(Hourly, by_minute: vec![0,59,9])),
            ("BYHOUR=0,23,9;FREQ=yearly\r\n", rrule!(Yearly, by_hour: vec![0,23,9])),
            ("BYMONTH=1,12,9;FREQ=yearly\r\n", rrule!(Yearly, by_month: vec![1,12,9])),
            ("BYMONTHDay=-31,31,9;FREQ=yearly\r\n", rrule!(Yearly, by_month_day: vec![-31,31,9])),
        ];
        for case in ok_cases {
            let result = parse_rrule.parse_peek(B(&case.0));
            assert!(result.is_ok(), "Error for {}: {result:#?}", case.0);
            assert_eq!(result.unwrap(), (B(""), case.clone().1), "Case: {}", case.0);
        }
    }
    #[test]
    fn test_parse_rrule_errors() {
        let error_cases = [
            ("\r\n", tag::FREQ_required),
            ("", tag::Expected_equal_sign),
            ("Freq=Yearly", tag::Expected_equal_sign),
            ("Foo=bar", tag::Unknown_component),
            ("Freq=Yearly;FREQ=Monthly\r\n", tag::Too_many_FREQs),
            ("Freq=Neverly\r\n", tag::FREQ_needs_Frequency),
            ("Freq=Yearly;WksT=MO;wkst=SU\r\n", too_many!(WkSt)),
            ("Freq=Yearly;Count=0;COUNT=4\r\n", too_many!(Count)),
            ("Freq=Yearly;Count=-1\r\n", tag::Bad_usize),
            ("Freq=Yearly;Interval=0;INTERVAL=4\r\n", too_many!(Interval)),
            ("Freq=Yearly;Interval=-1\r\n", tag::Bad_usize),
            ("Freq=Yearly;WKST=XX\r\n", tag::Expected_day_abbreviation),
            ("Freq=Yearly;BySecond=0,60,61\r\n", index_tag!(BySecond, 0, 60)),
            ("Freq=Yearly;BySecond=0,60,-1\r\n", index_tag!(BySecond, 0, 60)),
            ("Freq=Yearly;ByMinute=0,59,60\r\n", index_tag!(ByMinute, 0, 59)),
            ("Freq=Yearly;ByMinute=0,59,-1\r\n", index_tag!(ByMinute, 0, 59)),
            ("Freq=Yearly;ByHour=0,23,24\r\n", index_tag!(ByHour, 0, 23)),
            ("Freq=Yearly;ByHour=0,23,-1\r\n", index_tag!(ByHour, 0, 23)),
            ("Freq=Yearly;ByMonth=1,12,13\r\n", index_tag!(ByMonth, 1, 12)),
            ("Freq=Yearly;ByMonth=1,12,-1\r\n", index_tag!(ByMonth, 1, 12)),
            ("Freq=Yearly;ByMonthDay=1,12,0\r\n", index_tag!(ByMonthDay, -31, 31)),
            ("Freq=Yearly;ByMonthDay=1,12,-32\r\n", index_tag!(ByMonthDay, -31, 31)),
            ("Freq=Yearly;ByMonthDay=1,12,32\r\n", index_tag!(ByMonthDay, -31, 31)),
        ];
        for case in error_cases {
            let Err(err) = parse_rrule.parse_peek(B(&case.0)) else {
                panic!("No error for {case:?}")
            };
            let err = err.into_inner().unwrap();
            let context = err.context();
            assert_eq!(err.context(), vec![case.1], "Unexpected error for {case:?}:\n{err:?}\n");
        }
    }
    #[test]
    fn test_offsets() {
        let mut input = "Freq=Yearly;BySecond=0,60,-1\r\n".as_bytes();
        //                                         ^ Offset 26
        let err = parse_rrule.parse(&mut input).unwrap_err();
        assert_eq!(err.offset(), 26);
        let err = err.into_inner();
        assert_eq!(err.context(), vec![index_tag!(BySecond, 0, 60),]);

        let mut input = "Freq=Yearly;BySecond=0,61,60\r\n".as_bytes();
        //                                      ^ Offset 23
        let err = parse_rrule.parse(&mut input).unwrap_err();
        assert_eq!(err.offset(), 23);
        let err = err.into_inner();
        assert_eq!(err.context(), vec![index_tag!(BySecond, 0, 60),]);
    }
}
