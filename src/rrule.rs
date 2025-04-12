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
        "Expected a rule part name followed by an equal sign (=)";
    pub(super) const Bad_usize: &str = "Expected an unsigned integer";
    pub(super) const Unknown_rule_part: &str = "Unrecognized RRule rule part";
    pub(super) const FREQ_required: &str = "RRule must have a FREQ rule part";
    pub(super) const Too_many_FREQs: &str =
        "RRule must have exactly one FREQ rule part; found multiple";

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
    (Freq) => {
        msg::Too_many_FREQs
    };
    ($name:ident) => {
        paste! {
            concat!("RRule can have at most one ", stringify!([<$name:upper>]), " rule part")
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

/// Storage for an `RRule`
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

// Frequency =====================================================================
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
// Parse a `Frequency`
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

// `Weekday` and `WeekdaySpec` =============================
// Parse a Weekday
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
// WeekdaySpec
type WeekdaySpec = (Option<NonZeroI8>, Weekday);
// The ByDay rule part takes either an unadorned day abbreviation (ByDay=TU
// means every Tuesday in the relevant time period), or an offset followed by
// a day abbreviation (1TU means the first Tuesday in the relevant period, and
// -1TU means the last Tuesday in the relevant period.)
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
fn weekday_list(input: &mut &[u8]) -> ModalResult<Vec<WeekdaySpec>> {
    separated(1.., cut_err(weekday_spec), b',').parse_next(input)
}

// When =============================================================
#[derive(Debug, Clone, PartialEq)]
enum When {
    Date(Date),
    DateTime(DateTime),
    Timestamp(Timestamp),
}
fn when(input: &mut &[u8]) -> ModalResult<When> {
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

// Certain RRule rule parts (BySecond, ByMinute, ByHour, and ByMonth) take as
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

// Other rule parts (ByMonthDay, ByYearDay, ByWeekNo, and BySetPost) take as
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

macro_rules! list {
    ($name:ident<$N:ident>, $min:literal, $max:literal) => {
        OffsetList::<$N>::new(offset_msg!($name, $min, $max), $min..=$max)
    };
    ($name:ident, $min:literal, $max:literal) => {
        IndexList::new(index_msg!($name, $min, $max), $min..=$max)
    };
}
///
/// `parse_rrule` parses a recurrence rule.  Here `input` must be a single line
/// ending with `\cr\lf`. We use `&[u8]` on the assumption that the line has been
/// unfolded using `&[u8]` rather than `str`. This is the easiest way to follow
/// RFC 5545's advice: "It is possible for very simple implementations to
/// generate improperly folded lines in the middle of a UTF-8 multi-octet
/// sequence.  For this reason, implementations need to unfold lines in such
/// a way to properly restore the original sequence.""
pub fn parse_rrule(input: &mut &[u8]) -> ModalResult<RRule> {
    let mut rrule = RRule::default();

    macro_rules! fail {
        ($why:expr) => {
            return fail.context($why).parse_next(input)
        };
    }
    macro_rules! get_vec {
        ($field:ident, $parser:expr) => {
            if rrule.$field.is_empty() {
                rrule.$field = $parser.parse_next(input)?;
            } else {
                paste! { fail!(too_many!([<$field:camel>])) }
            }
        };
    }
    macro_rules! get_option {
        ($field:ident, $parser:expr) => {
            match rrule.$field {
                None => rrule.$field = Some($parser.parse_next(input)?),
                Some(_) => paste! { fail!(too_many!([<$field:camel>])) },
            }
        };
    }

    let mut freq = None;
    let mut name: Vec<u8>;
    // Every RRule line must end in CRLF, so we use that to trigger end-of-parse
    while crlf::<&[u8], Error>.parse_next(input).is_err() {
        // Extract the rule part name into 'name' and resume parsing after the equal sign
        let Some(eq) = memchr(b'=', input) else {
            fail!(msg::Expected_equal_sign);
        };
        name = input[0..eq].to_vec();
        name.make_ascii_uppercase();
        let old_input = *input;
        *input = &input[eq + 1..];

        match &name[..] {
            FREQ => match freq {
                None => freq = Some(frequency.parse_next(input)?),
                Some(_) => fail!(msg::Too_many_FREQs),
            },
            COUNT => get_option!(count, dec_uint.context(msg::Bad_usize)),
            UNTIL => get_option!(until, when),
            INTERVAL => get_option!(interval, dec_uint.context(msg::Bad_usize)),
            BYSECOND => get_vec!(by_second, list!(BySecond, 0, 60)),
            BYMINUTE => get_vec!(by_minute, list!(ByMinute, 0, 59)),
            BYHOUR => get_vec!(by_hour, list!(ByHour, 0, 23)),
            BYDAY => get_vec!(by_day, weekday_list),
            BYMONTHDAY => get_vec!(by_month_day, list!(ByMonthDay<i8>, -31, 31)),
            BYMONTH => get_vec!(by_month, list!(ByMonth, 1, 12)),
            BYYEARDAY => get_vec!(by_year_day, list!(ByYearDay<i16>, -366, 366)),
            BYWEEKNO => get_vec!(by_week_no, list!(ByWeekNo<i8>, -53, 53)),
            BYSETPOS => get_vec!(by_set_pos, list!(BySetPos<i16>, -366, 366)),
            WKST => get_option!(wk_st, weekday),
            _ => {
                if name[0] == b',' {
                    *input = old_input;
                    fail!(msg::Did_you_mean_semicolon);
                }
                fail!(msg::Unknown_rule_part);
            }
        }
        // Rule parts are separated by semicolons
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

// We need these `const` definitations because we can't use `"X"`.as_bytes() in a pattern
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
            ("Foo=bar", msg::Unknown_rule_part),
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
