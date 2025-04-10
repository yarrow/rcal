use crate::Weekday;
use crate::error::{Error, ModalResult};

use bstr::B;
use jiff::civil::{Date, DateTime};
use jiff::{Timestamp, tz::TimeZone};
use memchr::memchr;
use paste::paste;
use std::num::{NonZero, NonZeroI8};
use std::ops::RangeInclusive;

use winnow::ascii::{Caseless, Int, crlf, dec_int, dec_uint, digit1};
use winnow::combinator::{alt, cut_err, fail, opt, separated};
use winnow::error::{ErrMode, ParseError};
use winnow::{self, Parser};

// Data Types
//==============================================================================

#[derive(Default, Debug, Clone, PartialEq)]
pub struct RRule {
    freq: Frequency,
    count: Option<u32>,
    until: Option<When>,
    interval: Option<u32>,
    by_second: Vec<u8>,
    by_minute: Vec<u8>,
    by_hour: Vec<u8>,
    by_day: Vec<WeekdaySpec>,
    by_month_day: Vec<i8>,
    by_year_day: Vec<i16>,
    by_week_no: Vec<i8>,
    by_month: Vec<u8>,
    by_set_pos: Vec<i16>,
    wk_st: Option<Weekday>,
}

// We derive Default only because that makes it easier to handle the `freq` field,
// which unlike the others is not optional.
//
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

#[derive(Debug, Clone, PartialEq)]
enum When {
    Date(Date),
    DateTime(DateTime),
    Timestamp(Timestamp),
}

type WeekdaySpec = (Option<NonZeroI8>, Weekday);

// Error message constants.
// We allow non-uppercase because LONG_STRINGS_OF_UPPERCASE_ARE_HARDER_TO_READ
//==============================================================================
//
#[allow(non_upper_case_globals)]
mod msg {

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

    // ByDay_range is the only range that's not macro-created simultaneously with
    // the error message referring to it. We create it here so human eyes can
    // check that the bounds are the same in both places.
    pub(super) const ByDay_range: super::RangeInclusive<i8> = -53..=53;
    pub(super) const Expected_ByDay_offset: &str =
        "The number part of a BYDAY list item must be nonzero and between -53 and 53";
    pub(super) const Did_you_mean_semicolon: &str = "Did you mean to use a semicolon here?";
    pub(super) const UNTIL_expects: &str = "UNTIL expects yyyymmdd[Thhmmss[Z]]: a date, optionally \
        followed by T and a time, and an optional Z to indicate UTC";
    pub(super) const Not_a_time: &str =
        "This doesn't seem to be a legal date, date-time, or timestamp";
}

// Error message macros
//==============================================================================

macro_rules! too_many {
    ($name:ident) => {
        paste! {
            concat!("RRule can have at most one ", stringify!([<$name:upper>]))
        }
    };
}
macro_rules! index_msg {
    ($name:ident, $min:literal, $max:literal) => {
        paste! {
            concat!(stringify!([<$name:upper>]), " takes a list of numbers from ", $min, " to ", $max)
        }
    };
}
// We use `stringify!($min)` below to work around a rust analyszer bug:
// Wrong unexpected token diagnostic when passing negative numbers to concat! #19417
// (https://github.com/rust-lang/rust-analyzer/issues/19417)
macro_rules! offset_msg {
    ($name:ident, $min:literal, $max:literal) => {
        paste! {
            concat!(stringify!([<$name:upper>]), " takes a list of nonzero numbers from ", stringify!($min), " to ", $max)
        }
    };
}
// Parsers
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
        fail.context(msg::FREQ_needs_Frequency),
    )))
    .parse_next(input)
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
        fail.context(msg::Expected_day_abbreviation),
    )))
    .parse_next(input)
}

fn until(input: &mut &[u8]) -> ModalResult<When> {
    fn wrap<T: Default>(r: Result<T, jiff::Error>) -> ModalResult<T> {
        r.map_err(|e| Error::cut(msg::Not_a_time, Some(Box::new(e))))
    }
    let text = (digit1, opt((b'T', digit1, opt(b'Z'))))
        .take()
        .context(msg::UNTIL_expects)
        .parse_next(input)?;
    match text.len() {
        8 => Ok(When::Date(wrap(Date::strptime("%Y%m%d", text))?)),
        15 => Ok(When::DateTime(wrap(DateTime::strptime("%Y%m%dT%H%M%S", text))?)),
        16 => match DateTime::strptime("%Y%m%dT%H%M%S", &text[0..15]) {
            Ok(dt) => Ok(When::Timestamp(wrap(TimeZone::UTC.to_timestamp(dt))?)),
            Err(e) => Err(Error::cut(msg::Not_a_time, Some(Box::new(e)))),
        },
        _ => Err(Error::cut(msg::UNTIL_expects, None)),
    }
}

// Certain RRule components (BySecond, ByMinute, ByHour, and ByMonth) take as
// values a list of indices of the relevant period: `ByMonth=1,9`, for instance,
// refers to January and September. We call a list of such indices an IndexList
//
struct IndexList {
    msg: &'static str,
    range: RangeInclusive<u8>,
}
impl IndexList {
    const fn new(msg: &'static str, range: RangeInclusive<u8>) -> Self {
        IndexList { msg, range }
    }
}
impl Parser<&[u8], Vec<u8>, ErrMode<Error>> for IndexList {
    fn parse_next(&mut self, input: &mut &[u8]) -> ModalResult<Vec<u8>> {
        let item = dec_uint::<&[u8], u8, ErrMode<Error>>
            .context(self.msg)
            .verify(|n| self.range.contains(n));
        match separated(1.., cut_err(item), b',').parse_next(input) {
            Ok(value) => Ok(value),
            Err(_) => cut_err(fail.context(self.msg)).parse_next(input),
        }
    }
}

// Other components (ByMonthDay, ByYearDay, ByWeekNo, and BySetPost) take as
// values a list of offsets into some other period. `ByMonthDay=9,-1`, for
// instance, means the ninth and the last day of the month(s) it applies to.
// We call a list of such offsets an OffsetList.
//
#[derive(Clone)]
struct OffsetList<N: Int + PartialOrd + Default> {
    msg: &'static str,
    range: RangeInclusive<N>,
}
impl<N: Int + PartialOrd + Default> OffsetList<N> {
    const fn new(msg: &'static str, range: RangeInclusive<N>) -> Self {
        Self { msg, range }
    }
}
impl<N: Int + PartialOrd + Default> Parser<&[u8], Vec<N>, ErrMode<Error>> for OffsetList<N> {
    fn parse_next(&mut self, input: &mut &[u8]) -> ModalResult<Vec<N>> {
        let zero = N::default();
        let item = dec_int::<&[u8], N, ErrMode<Error>>
            .context(self.msg)
            .verify(|n: &N| *n != zero && self.range.contains(n));
        match separated(1.., cut_err(item), b',').parse_next(input) {
            Ok(value) => Ok(value),
            Err(_) => cut_err(fail.context(self.msg)).parse_next(input),
        }
    }
}

// The ByDay component takes either an unadorned day abbreviation (ByDay=TU
// means every Tuesday in the relevant time period), or an offset followed by
// a day abbreviation (1TU means the first Tuesday in the relevant period, and
// -1TU means the last Tuesday in the relevant period.)
fn weekday_list(input: &mut &[u8]) -> ModalResult<Vec<WeekdaySpec>> {
    separated(1.., cut_err(weekday_spec), b',').parse_next(input)
}
fn weekday_spec(input: &mut &[u8]) -> ModalResult<WeekdaySpec> {
    let offset = match input.first() {
        Some(ch) if (*ch == b'+') || *ch == b'-' || ch.is_ascii_digit() => NonZero::new(
            dec_int
                .verify(|n| *n != 0i8 && msg::ByDay_range.contains(n))
                .context(msg::Expected_ByDay_offset)
                .parse_next(input)?,
        ),
        _ => None,
    };
    let day_of_week = weekday.parse_next(input)?;
    Ok((offset, day_of_week))
}

/// `parse_rrule` parses a recurrence rule.  Here `input` must be a single line
/// ending with `\cr\lf`. We use `&[u8]` on the assumption that the line has been
/// unfolded using `&[u8]` rather than `str`. This is the easiest way to follow
/// RFC 5545's advice: "It is possible for very simple implementations to
/// generate improperly folded lines in the middle of a UTF-8 multi-octet
/// sequence.  For this reason, implementations need to unfold lines in such
/// a way to properly restore the original sequence.""
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
                    IndexList::new(index_msg!($name, $min, $max), $min..=$max).parse_next(input)?;
            } else {
                fail!(too_many!($name))
            }
        };
    }
    macro_rules! get_offset_list {
        ($name:ident<$N:ident>, $place:expr, $min:literal, $max:literal) => {
            if $place.is_empty() {
                $place = OffsetList::<$N>::new(offset_msg!($name, $min, $max), $min..=$max)
                    .parse_next(input)?;
            } else {
                fail!(too_many!($name))
            }
        };
    }
    macro_rules! get_weekday_list {
        ($place:expr) => {
            if $place.is_empty() {
                $place = weekday_list.parse_next(input)?;
            } else {
                fail!(too_many!(ByDay))
            }
        };
    }

    let mut freq = None;
    let mut rrule = RRule::default();
    let mut name: Vec<u8>;
    // Every RRule line must end in CRLF, so we use that to trigger end-of-parse
    while crlf::<&[u8], Error>.parse_next(input).is_err() {
        // Extract the component name into 'name' and resume parsing after the equal sign
        let Some(eq) = memchr(b'=', input) else {
            fail!(msg::Expected_equal_sign);
        };
        name = input[0..eq].to_vec();
        name.make_ascii_uppercase();
        let old_input = *input;
        *input = &input[eq + 1..];

        const FREQ: &[u8] = "FREQ".as_bytes();
        const COUNT: &[u8] = "COUNT".as_bytes();
        const UNTIL: &[u8] = "UNTIL".as_bytes();
        const INTERVAL: &[u8] = "INTERVAL".as_bytes();
        const BYSECOND: &[u8] = "BYSECOND".as_bytes();
        const BYMINUTE: &[u8] = "BYMINUTE".as_bytes();
        const BYHOUR: &[u8] = "BYHOUR".as_bytes();
        const BYDAY: &[u8] = "BYDAY".as_bytes();
        const BYMONTH: &[u8] = "BYMONTH".as_bytes();
        const BYMONTHDAY: &[u8] = "BYMONTHDAY".as_bytes();
        const BYYEARDAY: &[u8] = "BYYEARDAY".as_bytes();
        const BYWEEKNO: &[u8] = "BYWEEKNO".as_bytes();
        const BYSETPOS: &[u8] = "BYSETPOS".as_bytes();
        const WKST: &[u8] = "WKST".as_bytes();
        match &name[..] {
            FREQ => get_single!(Freq, freq, frequency, msg::Too_many_FREQs),
            COUNT => get_single!(Count, rrule.count, dec_uint.context(msg::Bad_usize)),
            UNTIL => get_single!(Until, rrule.until, until),
            INTERVAL => get_single!(Interval, rrule.interval, dec_uint.context(msg::Bad_usize)),
            BYSECOND => get_index_list!(BySecond, rrule.by_second, 0, 60),
            BYMINUTE => get_index_list!(ByMinute, rrule.by_minute, 0, 59),
            BYHOUR => get_index_list!(ByHour, rrule.by_hour, 0, 23),
            BYDAY => get_weekday_list!(rrule.by_day),
            BYMONTHDAY => get_offset_list!(ByMonthDay<i8>, rrule.by_month_day, -31, 31),
            BYMONTH => get_index_list!(ByMonth, rrule.by_month, 1, 12),
            BYYEARDAY => get_offset_list!(ByYearDay<i16>, rrule.by_year_day, -366, 366),
            BYWEEKNO => get_offset_list!(ByWeekNo<i8>, rrule.by_week_no, -53, 53),
            BYSETPOS => get_offset_list!(BySetPos<i16>, rrule.by_set_pos, -366, 366),
            WKST => get_single!(WkSt, rrule.wk_st, weekday),
            _ => fail!(if name[0] == b',' {
                *input = old_input;
                msg::Did_you_mean_semicolon
            } else {
                msg::Unknown_component
            }),
        }
        // Components are separated by semicolons
        if input.first() == Some(&b';') {
            *input = &input[1..];
        }
    }

    match freq {
        None => fail!(msg::FREQ_required),
        Some(f) => rrule.freq = f,
    }

    Ok(rrule)
}

#[cfg(test)]
mod test {
    use super::*;
    use bstr::{BString, ByteSlice};
    use jiff::civil;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_rrule_ok() {
        use Weekday::*;
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
                rrule!(Secondly, count: Some(0), wk_st: Some(Wednesday)),
            ),
            ("BYSECOND=0,60,9;FREQ=hourly\r\n", rrule!(Hourly, by_second: vec![0,60,9])),
            ("BYMINUTE=0,59,9;FREQ=hourly\r\n", rrule!(Hourly, by_minute: vec![0,59,9])),
            ("BYHOUR=0,23,9;FREQ=yearly\r\n", rrule!(Yearly, by_hour: vec![0,23,9])),
            (
                "BYDAY=MO,23Tu,-9sa;FREQ=yearly\r\n",
                rrule!(Yearly, by_day: vec![
                    (None, Monday),
                    (NonZeroI8::new(23), Tuesday),
                    (NonZeroI8::new(-9), Saturday)
                ]),
            ),
            ("BYMONTH=1,12,9;FREQ=yearly\r\n", rrule!(Yearly, by_month: vec![1,12,9])),
            ("BYMONTHDay=-31,31,9;FREQ=yearly\r\n", rrule!(Yearly, by_month_day: vec![-31,31,9])),
            ("BYweekNO=-53,53,9;FREQ=yearly\r\n", rrule!(Yearly, by_week_no: vec![-53,53,9])),
            ("BYYearDAY=-366,366,9;FREQ=yearly\r\n", rrule!(Yearly, by_year_day: vec![-366,366,9])),
            ("BYsetPOS=-366,366,9;FREQ=yearly\r\n", rrule!(Yearly, by_set_pos: vec![-366,366,9])),
            (
                "FREQ=Monthly;until=20000101\r\n",
                rrule!(Monthly, until: Some(When::Date(civil::date(2000,1,1)))),
            ),
            (
                "FREQ=Monthly;until=20000101T020304\r\n",
                rrule!(Monthly, until: Some(When::DateTime(civil::datetime(2000,1,1,2,3,4,0)))),
            ),
            (
                "FREQ=Monthly;until=20000101T020304Z\r\n",
                rrule!(Monthly, until: Some(When::Timestamp(TimeZone::UTC.to_timestamp(civil::datetime(2000,1,1,2,3,4,0)).unwrap()))),
            ),
        ];
        for case in ok_cases {
            let result = parse_rrule.parse_peek(B(&case.0));
            if result.is_ok() {
                assert_eq!(result.unwrap(), (B(""), case.clone().1), "Case: {}", case.0);
            } else {
                let input = case.0.as_bytes();
                match parse_rrule.parse(input) {
                    Ok(_) => panic!("Error on parse_peek but not parse!? Case:_{}", case.0),
                    Err(err) => {
                        let input = err.input().as_bstr();
                        let offset = err.offset();
                        let err = err.into_inner();
                        let to_pointer = " ".repeat("input: ".len() + offset);
                        panic!("input: {input}\n{to_pointer}^\noffset: {offset}\n{err:#?}");
                    }
                }
            }
        }
    }
    #[test]
    fn test_parse_rrule_errors() {
        let error_cases = [
            ("\r\n", msg::FREQ_required),
            ("FREQ=Monthly,count=42\r\n", msg::Did_you_mean_semicolon),
            ("", msg::Expected_equal_sign),
            ("Freq=Yearly", msg::Expected_equal_sign),
            ("Foo=bar", msg::Unknown_component),
            ("Freq=Yearly;FREQ=Monthly\r\n", msg::Too_many_FREQs),
            ("Freq=Neverly\r\n", msg::FREQ_needs_Frequency),
            ("Freq=Yearly;WksT=MO;wkst=SU\r\n", too_many!(WkSt)),
            ("Freq=Yearly;Count=0;COUNT=4\r\n", too_many!(Count)),
            ("Freq=Yearly;Count=-1\r\n", msg::Bad_usize),
            ("Freq=Yearly;Interval=0;INTERVAL=4\r\n", too_many!(Interval)),
            ("Freq=Yearly;Interval=-1\r\n", msg::Bad_usize),
            ("Freq=Yearly;WKST=XX\r\n", msg::Expected_day_abbreviation),
            ("Freq=Yearly;BySecond=0,60,61\r\n", index_msg!(BySecond, 0, 60)),
            ("Freq=Yearly;BySecond=0,60,-1\r\n", index_msg!(BySecond, 0, 60)),
            ("Freq=Yearly;ByMinute=0,59,60\r\n", index_msg!(ByMinute, 0, 59)),
            ("Freq=Yearly;ByMinute=0,59,-1\r\n", index_msg!(ByMinute, 0, 59)),
            ("Freq=Yearly;ByHour=0,23,24\r\n", index_msg!(ByHour, 0, 23)),
            ("Freq=Yearly;ByHour=0,23,-1\r\n", index_msg!(ByHour, 0, 23)),
            ("Freq=Yearly;bYdAY=Mo,XY,Fr\r\n", msg::Expected_day_abbreviation),
            ("Freq=Yearly;bYdAY=Mo,0Tu\r\n", msg::Expected_ByDay_offset),
            ("Freq=Yearly;ByMonth=1,12,13\r\n", index_msg!(ByMonth, 1, 12)),
            ("Freq=Yearly;ByMonth=1,12,-1\r\n", index_msg!(ByMonth, 1, 12)),
            ("Freq=Yearly;ByMonthDay=1,12,0\r\n", offset_msg!(ByMonthDay, -31, 31)),
            ("Freq=Yearly;ByMonthDay=1,12,-32\r\n", offset_msg!(ByMonthDay, -31, 31)),
            ("Freq=Yearly;ByMonthDay=1,12,32\r\n", offset_msg!(ByMonthDay, -31, 31)),
            ("Freq=Yearly;ByWeekNo=1,12,0\r\n", offset_msg!(ByWeekNo, -53, 53)),
            ("Freq=Yearly;ByWeekNo=1,12,-54\r\n", offset_msg!(ByWeekNo, -53, 53)),
            ("Freq=Yearly;ByWeekNo=1,12,54\r\n", offset_msg!(ByWeekNo, -53, 53)),
            ("Freq=Yearly;ByYearDay=1,12,0\r\n", offset_msg!(ByYearDay, -366, 366)),
            ("Freq=Yearly;ByYearDay=1,12,-367\r\n", offset_msg!(ByYearDay, -366, 366)),
            ("Freq=Yearly;ByYearDay=1,12,367\r\n", offset_msg!(ByYearDay, -366, 366)),
            ("Freq=Yearly;BySetPos=1,12,0\r\n", offset_msg!(BySetPos, -366, 366)),
            ("Freq=Yearly;BySetPos=1,12,-367\r\n", offset_msg!(BySetPos, -366, 366)),
            ("Freq=Yearly;BySetPos=1,12,367\r\n", offset_msg!(BySetPos, -366, 366)),
            ("Freq=Yearly;UNTIL=gagaga\r\n", msg::UNTIL_expects),
            ("Freq=Yearly;UNTIL=1234567\r\n", msg::UNTIL_expects),
            ("Freq=Yearly;UNTIL=123456789\r\n", msg::UNTIL_expects),
            ("Freq=Yearly;UNTIL=20251301\r\n", msg::Not_a_time),
        ];
        for case in error_cases {
            let Err(err) = parse_rrule.parse_peek(B(&case.0)) else {
                panic!("No error for {case:?}")
            };
            let err = err.into_inner().unwrap();
            assert_eq!(err.context(), vec![case.1], "Unexpected error for {case:?}:\n{err:?}\n");
        }
    }

    fn error_info<T: std::fmt::Debug>(
        err: Result<T, ParseError<&[u8], Error>>,
    ) -> (usize, Vec<&'static str>) {
        let err = err.unwrap_err();
        let offset = err.offset();
        (offset, err.into_inner().context())
    }
    #[test]
    fn test_offsets() {
        let context = vec![index_msg!(BySecond, 0, 60)];

        let input = "Freq=Yearly;BySecond=0,60,-1\r\n".as_bytes();
        assert_eq!(error_info(parse_rrule.parse(input)), (26, context.clone()),);

        let input = "Freq=Yearly;BySecond=0,61,60\r\n".as_bytes();
        assert_eq!(error_info(parse_rrule.parse(input)), (23, context.clone()),);
    }

    #[test]
    fn test_weekday_spec_errors() {
        for good in ["", "42", "-53", "53"] {
            let base = BString::from(format!("{good}SX"));
            let input = &base;
            assert_eq!(
                error_info(weekday_spec.parse(input)),
                (good.len(), vec![msg::Expected_day_abbreviation])
            );
        }
        for bad in ["0", "54", "-54"] {
            let base = BString::from(format!("{bad}SU"));
            let input = &base;
            assert_eq!(
                error_info(weekday_spec.parse(input)),
                (0, vec![msg::Expected_ByDay_offset])
            );
        }
    }
}
