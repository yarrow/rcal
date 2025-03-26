#![allow(non_upper_case_globals)]

use jiff::{civil::{Date, DateTime}, Zoned};

use std::num::NonZeroI8;
use std::ops::RangeInclusive;
use std::str::FromStr;

use paste::paste;

use crate::Weekday;
use bstr::{B, ByteSlice};
use memchr::{Memchr3, memchr};

use winnow::ascii::{Caseless, Int, dec_int, dec_uint, crlf};
use winnow::combinator::{alt, cut_err, fail, opt, separated};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

#[derive(Debug, PartialEq)]
enum Tagged {
    CivilDate(Date),
    CivilDateTime(DateTime),
    ZonedDate(Zoned),
    ZonedDateTime(Zoned),
}
#[derive(Debug, PartialEq, Eq, Clone)]
enum RulePart {
    Freq(Frequency),
    Count(usize),
    Interval(usize),
    BySecond(Vec<u8>),
    ByMinute(Vec<u8>),
    ByHour(Vec<u8>),
    ByDay(Vec<SomeWeekdays>),
    ByMonthDay(Vec<i8>),
    ByYearDay(Vec<i16>),
    ByWeekNo(Vec<i8>),
    ByMonth(Vec<u8>),
    BySetPos(Vec<i16>),
    WkSt(Weekday),
}


#[derive(Default, Debug, PartialEq)]
struct RRule {
    freq: Frequency,
    count: Option<u32>,
    until: Option<Tagged>,
    interval: Option<usize>,
    by_second: Option<Vec<u8>>,
    by_minute: Option<Vec<u8>>,
    by_hour: Option<Vec<u8>>,
    by_day: Option<Vec<SomeWeekdays>>,
    by_month_day: Option<Vec<i8>>,
    by_year_day: Option<Vec<i16>>,
    by_week_no: Option<Vec<i8>>,
    by_month: Option<Vec<u8>>,
    by_set_pos: Option<Vec<i16>>,
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

//==============================================================================
macro_rules! constants_for {
    ($name:ident, $context:expr) => {
        paste! {
            const [<$name _as_bytes>]: &[u8] = stringify!([<$name:upper>]).as_bytes();
            const [<$name Context>]: StrContext = StrContext::Label($context);
        }
    }
}
macro_rules! u8_rule {
    ($name: ident, $min:literal, $max:literal) => {
        num_list_rule!(U8list, $name, $min, $max, "numbers")
    };
}

macro_rules! i8_rule {
    ($name: ident, $min:literal, $max:literal) => {
        num_list_rule!(SignedList::<i8>, $name, $min, $max, "nonzero numbers")
    };
}

macro_rules! i16_rule {
    ($name:ident, $min:literal, $max:literal) => {
        num_list_rule!(SignedList::<i16>, $name, $min, $max, "nonzero numbers")
    };
}

macro_rules! num_list_rule {
    ($list:ty, $name:ident, $min:literal, $max:literal, $numbers:literal) => {
        paste! {
            const [<$name _as_bytes>]: &[u8] = stringify!([<$name:upper>]).as_bytes();
            const [<$name List>]: $list = $list::new(
                concat!(stringify!($name), " takes a list of ", $numbers, " from ", $min, " to ", $max),
                $min ..= $max
                );
        }
    };
}

#[derive(Clone)]
struct SignedList<N: Int + PartialOrd + Default> {
    tag: StrContext,
    range: RangeInclusive<N>,
}
impl<N: Int + PartialOrd + Default> SignedList<N> {
    const fn new(tag: &'static str, range: RangeInclusive<N>) -> Self {
        Self {
            tag: StrContext::Label(tag),
            range,
        }
    }
}
impl<N: Int + PartialOrd + Default> Parser<&[u8], Vec<N>, ErrMode<ContextError>> for SignedList<N> {
    fn parse_next(&mut self, input: &mut &[u8]) -> ModalResult<Vec<N>> {
        let zero = N::default();
        let item = dec_int::<&[u8], N, ErrMode<ContextError>>
            .context(self.tag.clone())
            .verify(|n: &N| *n != zero && self.range.contains(n));
        separated(1.., cut_err(item), b',').parse_next(input)
    }
}

#[derive(Clone)]
struct U8list {
    tag: StrContext,
    range: RangeInclusive<u8>,
}
impl U8list {
    const fn new(tag: &'static str, range: RangeInclusive<u8>) -> Self {
        U8list {
            tag: StrContext::Label(tag),
            range,
        }
    }
}
impl Parser<&[u8], Vec<u8>, ErrMode<ContextError>> for U8list {
    fn parse_next(&mut self, input: &mut &[u8]) -> ModalResult<Vec<u8>> {
        let item = dec_uint::<&[u8], u8, ErrMode<ContextError>>
            .context(self.tag.clone())
            .verify(|n| self.range.contains(n));
        separated(1.., cut_err(item), b',').parse_next(input)
    }
}

//==============================================================================
const FREQ_needs_Frequency: &str =  "FREQ takes a frequency, from SECONDLY to YEARLY";
const Expected_equal_sign: &str = "Expected an equal sign (=)";
const Too_many_FREQs: &str = "RRule must have exactly one FREQ component; found multiple";
const Unknown_component: &str = "Unrecognized RRule component";
const FREQ_required: &str = "RRule must have a FREQ component";
fn parse_rrule(input: &mut &[u8]) -> ModalResult<RRule> {
    macro_rules! fail {
        ($why:expr) => { return fail.context(StrContext::Label($why)).parse_next(input) }
    }
    let mut freq = None;
    let mut rrule = RRule::default();
    constants_for!(Freq, FREQ_needs_Frequency);

    loop {
        if crlf::<&[u8], ContextError>.parse_next(input).is_ok() { break}
        let Some(eq) = memchr(b'=', input) else {
            fail!(Expected_equal_sign);
        };
        let name = input[0..eq].as_bstr().to_ascii_uppercase();
        *input = if eq < input.len() {
            &input[eq + 1..]
        } else {
            todo!()
        };
        use RulePart::*;
        match &name[..] {
            Freq_as_bytes => if freq.is_some() {
                fail!(Too_many_FREQs);
            } else { freq = Some(frequency.parse_next(input)?) }
            _ => fail!(Unknown_component),
        }
    }

    match freq {
        None => fail!("RRule must have a FREQ component"),
        Some(f) => rrule.freq = f,
    }

    Ok(rrule)
}
#[test]
fn test_parse_rrule() {
    let secondly = RRule{freq: Frequency::Secondly, ..Default::default()};
    for pair in [("\r\n", FREQ_required)] {
        let Err(err)= parse_rrule.parse_peek(B(&pair.0)) else { panic!("No error for {pair:?}")};
        let err = err.into_inner().unwrap();
        let context = err.context().collect::<Vec<_>>();
        assert_eq!(context.len(), 1);
        assert_eq!(context[0].to_string(), format!("invalid {}", pair.1));
    }
}
//==============================================================================
fn one_part(input: &mut &[u8]) -> ModalResult<RulePart> {

    constants_for!(Freq, "Freq takes a frequency, from SECONDLY to YEARLY");
    u8_rule!(BySecond, 0, 60);
    u8_rule!(ByMinute, 0, 59);
    u8_rule!(ByHour, 0, 23);
    u8_rule!(ByMonth, 1, 12);
    i8_rule!(ByMonthDay, -31, 31);
    i8_rule!(ByWeekNo, -53, 53);
    i16_rule!(ByYearDay, -366, 366);
    i16_rule!(BySetPos, -366, 366);
    constants_for!(WkSt, "WkSt takes an abbreviation for the starting day of the week: SU, MO, etc");

    let Some(eq) = memchr(b'=', input) else {
        todo!()
    };
    let name = input[0..eq].as_bstr().to_ascii_uppercase();
    *input = if eq < input.len() {
        &input[eq + 1..]
    } else {
        todo!()
    };
    use RulePart::*;
    Ok(match &name[..] {
        Freq_as_bytes => Freq(frequency.context(FreqContext).parse_next(input)?),
        BySecond_as_bytes => BySecond(BySecondList.clone().parse_next(input)?),
        ByMinute_as_bytes => ByMinute(ByMinuteList.clone().parse_next(input)?),
        ByHour_as_bytes => ByHour(ByHourList.clone().parse_next(input)?),
        ByMonth_as_bytes => ByMonth(ByMonthList.clone().parse_next(input)?),
        ByMonthDay_as_bytes => ByMonthDay(ByMonthDayList.clone().parse_next(input)?),
        ByWeekNo_as_bytes => ByWeekNo(ByWeekNoList.clone().parse_next(input)?),
        ByYearDay_as_bytes => ByYearDay(ByYearDayList.clone().parse_next(input)?),
        BySetPos_as_bytes => BySetPos(BySetPosList.clone().parse_next(input)?),
        WkSt_as_bytes => WkSt(weekday.context(WkStContext).parse_next(input)?),
        _ => fail.context(StrContext::Label("expected an RRULE part")).parse_next(input)?,
    })
}

//==============================================================================
#[cfg(test)]
#[allow(non_snake_case)]
mod test {
    use std::f32::consts::E;

    use super::*;
    use RulePart::*;

    #[test]
    fn test_Freq() {
        use Frequency::*;
        for f in [Secondly, Minutely, Hourly, Daily, Weekly, Monthly, Yearly] {
            let input = format!("FrEQ={f:?}");
            assert_eq!(one_part.parse_peek(B(&input)), Ok((B(""), Freq(f))));
        }
        assert!(one_part.parse_peek(B("FREQ=Neverly")).is_err());
    }

    #[test]
    fn test_WkSt() {
        use Weekday::*;
        for d in [Sunday, Monday, Tuesday, Wednesday, Thursday, Friday, Saturday] {
            let input = &format!("WkSt={d:?}")[0..7];
            let (rest, result) = one_part.parse_peek(B(&input)).unwrap();
            assert_eq!(one_part.parse_peek(B(&input)), Ok((B(""), WkSt(d))));
        }
        assert!(one_part.parse_peek(B("WkSt=Noday")).is_err());
    }

    fn check_ok(input: &str, result: RulePart) {
        let input = format!("{input};");
        assert_eq!(
            one_part.parse_peek(B(&input)),
            Ok((B(";"), result)),
            "for input {input}"
        );
    }

    fn check_err<const N: usize>(name: &str, bad_values: [&str; N]) {
        for bad in bad_values {
            let input = format!("{name}={bad}");
            let equal_sign = name.len();
            let result = one_part.parse(B(&input));
            assert!(
                result.is_err(),
                "Result for input '{input}' isn't an error: {result:#?}"
            );
            let err = result.unwrap_err();
            assert!(
                err.offset() > equal_sign,
                "In '{input}', error should happen after the equals sign, but {} <= {equal_sign}",
                err.offset()
            );
            assert_eq!(err.input().as_bstr(), input, "for input {input}");
        }
    }
    #[test]
    fn test_BySecond() {
        check_ok("bySecond=0,2,3,60", BySecond(vec![0u8, 2u8, 3u8, 60u8]));
        check_err("BySecond", ["x1", "-1", "61"]);
    }
    #[test]
    fn test_ByMinute() {
        check_ok("byMinute=0,2,3,59", ByMinute(vec![0u8, 2u8, 3u8, 59u8]));
        check_err("ByMinute", ["x1", "-1", "60"]);
    }
    #[test]
    fn test_ByHour() {
        check_ok("byHour=0,1,2,3,23", ByHour(vec![0u8, 1u8, 2u8, 3u8, 23u8]));
        check_err("ByHour", ["x1", "-1", "24"]);
    }
    #[test]
    fn test_ByMonth() {
        check_ok("byMonth=1,2,3,12", ByMonth(vec![1u8, 2u8, 3u8, 12u8]));
        check_err("ByMonth", ["x1", "0", "13"]);
    }
    #[test]
    fn test_ByMonthDay() {
        check_ok(
            "byMonthDay=1,-1,31,-31",
            ByMonthDay(vec![1i8, -1i8, 31i8, -31i8]),
        );
        check_err("ByMonthDay", ["x1", "0", "32", "-32"]);
    }
    #[test]
    fn test_ByWeekNo() {
        check_ok(
            "byWeekNo=1,-1,53,-53",
            ByWeekNo(vec![1i8, -1i8, 53i8, -53i8]),
        );
        check_err("ByWeekNo", ["x1", "0", "54", "-54"]);
    }
    #[test]
    fn test_ByYearDay() {
        check_ok(
            "byYearDay=1,-1,366,-366",
            ByYearDay(vec![1i16, -1i16, 366i16, -366i16]),
        );
        check_err("ByYearDay", ["x1", "0", "367", "-367"]);
    }
    #[test]
    fn test_BySetPos() {
        check_ok(
            "bySetPos=1,-1,366,-366",
            BySetPos(vec![1i16, -1i16, 366i16, -366i16]),
        );
        check_err("BySetPos", ["x1", "0", "367", "-367"]);
    }
}
