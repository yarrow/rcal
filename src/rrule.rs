use std::str::FromStr;

use crate::Weekday;
use bstr::{B, BStr, ByteSlice};

use winnow::ascii::{Caseless, Int, Uint, dec_int, dec_uint};
use winnow::combinator::{alt, cut_err, opt, separated};
use winnow::error::{
    ContextError, EmptyError, ErrMode, Needed, ParserError, StrContext, StrContextValue,
};
use winnow::token::literal;
use winnow::{ModalResult, Parser, Result};
use winnow::{Str, prelude::*};

#[derive(Debug, PartialEq, Eq, Clone)]
enum RulePart {
    Freq(Frequency),
    Count(usize),
    Interval(usize),
    BySecond(Vec<u8>),
    ByMinute(Vec<u8>),
    ByHour(Vec<u8>),
    ByDay((i8, Weekday)),
    ByMonthDay(Vec<i8>),
    WkSt(Weekday),
}

#[test]
fn check_rule_part_errors() {
    check_error(freq, "FREQ=weird", [FREQ_LABEL, FREQ_EXPECTED]);
    check_error(count, "COUNT=nonnumeric", [COUNT_LABEL, COUNT_EXPECTED]);
    check_error(interval, "interval=x", [INTERVAL_LABEL, INTERVAL_EXPECTED]);
    check_error(wk_st, "WkSt=xx", [WEEKDAY_LABEL, WEEKDAY_EXPECTED]);
    for seconds_list in ["xx", "2,XX", "2,3,4,61"] {
        let input = format!("BySeConD={seconds_list}");
        check_error(by_second, &input, [BY_SECOND_LABEL, BY_SECOND_EXPECTED]);
    }
    for minutes_list in ["xx", "2,XX", "2,3,4,60"] {
        let input = format!("ByMINUTE={minutes_list}");
        check_error(by_minute, &input, [BY_MINUTE_LABEL, BY_MINUTE_EXPECTED]);
    }
    for hours_list in ["xx", "2,XX", "2,3,4,24"] {
        let input = format!("ByHOUR={hours_list}");
        check_error(by_hour, &input, [BY_HOUR_LABEL, BY_HOUR_EXPECTED]);
    }
    for month_day_list in ["xx", "2,XX", "2,-32,4,24", "2,32,-4,24"] {
        let input = format!("ByMONTHDAY={month_day_list}");
        check_error(
            by_month_day,
            &input,
            [BY_MONTH_DAY_LABEL, BY_MONTH_DAY_EXPECTED],
        );
    }
}
#[cfg(test)]
#[allow(clippy::needless_pass_by_value)]
fn check_error<const N: usize>(
    mut parser: impl FnMut(&mut &[u8]) -> ModalResult<RulePart>,
    input: &str,
    context: [StrContext; N],
) {
    let equal_sign = input.find('=').expect("Can't find an equal sign (=)");
    let result = parser.parse(B(input));
    assert!(result.is_err(), "Result isn't an error: {result:#?}");
    let err = result.unwrap_err();
    assert!(
        err.offset() > equal_sign,
        "In '{input}', error should happen after the equals sign, but {} <= {equal_sign}",
        err.offset()
    );
    assert_eq!(err.input().as_bstr(), input);
    assert_eq!(err.inner().context().cloned().collect::<Vec<_>>(), context);
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Frequency {
    Secondly,
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

const fn label(lbl: &'static str) -> StrContext {
    StrContext::Label(lbl)
}
const fn expected(description: &'static str) -> StrContext {
    StrContext::Expected(StrContextValue::Description(description))
}
// Parse Freq and Frequency
//
fn freq(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("FREQ=").parse_next(input)?;
    Ok(RulePart::Freq(frequency(input)?))
}
#[test]
fn test_freq() {
    assert_eq!(
        freq.parse_peek(B("FREQ=YeaRly,")),
        Ok((B(","), RulePart::Freq(Frequency::Yearly)))
    );
}

const FREQ_LABEL: StrContext = label("frequency");
const FREQ_EXPECTED: StrContext = expected("Frequency: DAILY, WEEKLY, MONTHLY, etc");
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
    .context(FREQ_LABEL)
    .context(FREQ_EXPECTED)
    .parse_next(input)
}
#[test]
fn test_frequency() {
    use Frequency::*;
    let frequencies = [Secondly, Minutely, Hourly, Daily, Weekly, Monthly, Yearly];
    for frq in frequencies {
        assert_eq!(
            frequency.parse_peek(B(&format!("{frq:?},"))),
            Ok((B(","), frq))
        );
    }
}

// Parse Count
//
const COUNT_LABEL: StrContext = label("occurrences");
const COUNT_EXPECTED: StrContext = expected("Number of occurrences of the repeating event");
fn count(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("COUNT=").parse_next(input)?;
    let n = dec_uint
        .context(COUNT_LABEL)
        .context(COUNT_EXPECTED)
        .parse_next(input)?;
    Ok(RulePart::Count(n))
}
#[test]
fn test_count() {
    assert_eq!(
        count.parse_peek(B("count=42,")),
        Ok((B(","), RulePart::Count(42)))
    );
}

// Parse Interval
//
const INTERVAL_LABEL: StrContext = label("interval");
const INTERVAL_EXPECTED: StrContext = expected("How often the repeating event occurs");
fn interval(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("INTERVAL=").parse_next(input)?;
    let n = dec_uint
        .context(INTERVAL_LABEL)
        .context(INTERVAL_EXPECTED)
        .parse_next(input)?;
    Ok(RulePart::Interval(n))
}
#[test]
fn test_interval() {
    assert_eq!(
        interval.parse_peek(B("interval=42,")),
        Ok((B(","), RulePart::Interval(42)))
    );
}

// Clamped — for a list of seconds, minutes, or hours
#[derive(Debug, Clone)]
struct Clamped {
    range: std::ops::RangeInclusive<u8>,
    label: StrContext,
    expected: StrContext,
}
impl Parser<&[u8], u8, ErrMode<ContextError>> for Clamped {
    fn parse_next(&mut self, input: &mut &[u8]) -> ModalResult<u8> {
        dec_uint
            .verify(|n: &u8| self.range.contains(n))
            .context(self.label.clone())
            .context(self.expected.clone())
            .parse_next(input)
    }
}

// ClampedSigned — for a list of weeks (in year), days in month, or days in year
#[derive(Debug, Clone)]
struct ClampedSigned<N: Int + PartialOrd + Default> {
    range: std::ops::RangeInclusive<N>,
    label: StrContext,
    expected: StrContext,
}
impl<N: Int + PartialOrd + Default> Parser<&[u8], N, ErrMode<ContextError>> for ClampedSigned<N> {
    fn parse_next(&mut self, input: &mut &[u8]) -> ModalResult<N> {
        let zero = N::default();
        dec_int
            .verify(|n: &N| *n != zero && self.range.contains(n))
            .context(self.label.clone())
            .context(self.expected.clone())
            .parse_next(input)
    }
}

// Parse BySecond
//
const BY_SECOND_LABEL: StrContext = label("a list of seconds");
const BY_SECOND_EXPECTED: StrContext =
    expected("numbers between 0 and 60 (60 is for leap seconds only)");
fn by_second(input: &mut &[u8]) -> ModalResult<RulePart> {
    let num = Clamped {
        range: 0..=60,
        label: BY_SECOND_LABEL,
        expected: BY_SECOND_EXPECTED,
    };
    Caseless("BYSECOND=").parse_next(input)?;
    let second_list = separated(1.., cut_err(num), b',').parse_next(input)?;
    Ok(RulePart::BySecond(second_list))
}
#[test]
fn test_by_second() {
    assert_eq!(
        by_second.parse_peek(B("bySecond=42;")),
        Ok((B(";"), RulePart::BySecond(vec![42u8]))),
    );
    assert_eq!(
        by_second.parse_peek(B("bySecond=0,1,2,3,60;")),
        Ok((B(";"), RulePart::BySecond(vec![0u8, 1u8, 2u8, 3u8, 60u8]))),
    );
}

// Parse ByMinute
//
const BY_MINUTE_LABEL: StrContext = label("a list of minutes");
const BY_MINUTE_EXPECTED: StrContext = expected("numbers between 0 and 59");
fn by_minute(input: &mut &[u8]) -> ModalResult<RulePart> {
    let num = Clamped {
        range: 0..=59,
        label: BY_MINUTE_LABEL,
        expected: BY_MINUTE_EXPECTED,
    };
    Caseless("BYMINUTE=").parse_next(input)?;
    let minute_list = separated(1.., cut_err(num), b',').parse_next(input)?;
    Ok(RulePart::ByMinute(minute_list))
}
#[test]
fn test_by_minute() {
    assert_eq!(
        by_minute.parse_peek(B("byMinute=42;")),
        Ok((B(";"), RulePart::ByMinute(vec![42u8]))),
    );
    assert_eq!(
        by_minute.parse_peek(B("byMinute=0,1,2,3,59;")),
        Ok((B(";"), RulePart::ByMinute(vec![0u8, 1u8, 2u8, 3u8, 59u8]))),
    );
}

// Parse ByHour
//
const BY_HOUR_LABEL: StrContext = label("a list of hours");
const BY_HOUR_EXPECTED: StrContext = expected("numbers between 0 and 23");
fn by_hour(input: &mut &[u8]) -> ModalResult<RulePart> {
    let num = Clamped {
        range: 0..=23,
        label: BY_HOUR_LABEL,
        expected: BY_HOUR_EXPECTED,
    };
    Caseless("BYHOUR=").parse_next(input)?;
    let hour_list = separated(1.., cut_err(num), b',').parse_next(input)?;
    Ok(RulePart::ByHour(hour_list))
}
#[test]
fn test_by_hour() {
    assert_eq!(
        by_hour.parse_peek(B("byHOUR=12;")),
        Ok((B(";"), RulePart::ByHour(vec![12u8]))),
    );
    assert_eq!(
        by_hour.parse_peek(B("byHour=0,1,2,3,23;")),
        Ok((B(";"), RulePart::ByHour(vec![0u8, 1u8, 2u8, 3u8, 23u8]))),
    );
}

// Parse ByMonthDay
//
const BY_MONTH_DAY_LABEL: StrContext = label("a list of days");
const BY_MONTH_DAY_EXPECTED: StrContext =
    expected("nonzero numbers between -31 and 31 (-1 is the last day of the month)");
fn by_month_day(input: &mut &[u8]) -> ModalResult<RulePart> {
    let num = ClampedSigned::<i8> {
        range: -31..=31,
        label: BY_MONTH_DAY_LABEL,
        expected: BY_MONTH_DAY_EXPECTED,
    };
    Caseless("BYMONTHDAY=").parse_next(input)?;
    let month_day_list = separated(1.., cut_err(num), b',').parse_next(input)?;
    Ok(RulePart::ByMonthDay(month_day_list))
}
#[test]
fn test_by_month_day() {
    assert_eq!(
        by_month_day.parse_peek(B("byMONTHDAY=-12;")),
        Ok((B(";"), RulePart::ByMonthDay(vec![-12i8]))),
    );
    assert_eq!(
        by_month_day.parse_peek(B("byMonthDay=-31,+2,3,31,+31;")),
        Ok((
            B(";"),
            RulePart::ByMonthDay(vec![-31i8, 2i8, 3i8, 31i8, 31i8])
        )),
    );
}

// Parse WkSt and Weekday
//
fn wk_st(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("WKST=").parse_next(input)?;
    Ok(RulePart::WkSt(weekday(input)?))
}
#[test]
fn test_wk_st() {
    assert_eq!(
        wk_st.parse_peek(B("WkST=SU,")),
        Ok((B(","), RulePart::WkSt(Weekday::Sunday)))
    );
}

const WEEKDAY_LABEL: StrContext = label("weekday");
const WEEKDAY_EXPECTED: StrContext = expected("Weekday abbreviation: SU, MO, TU, WE, TH, FR, SA");
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
    .context(WEEKDAY_LABEL)
    .context(WEEKDAY_EXPECTED)
    .parse_next(input)
}
#[test]
fn test_weekday() {
    use Weekday::*;
    assert_eq!(weekday.parse_peek(B("Su,,,")), Ok((B(",,,"), Sunday)));
    let rest = [
        (B("mo,,,"), (B(",,,"), Monday)),
        (B("tUggg"), (B("ggg"), Tuesday)),
        (B("WE,,,"), (B(",,,"), Wednesday)),
        (B("Th,,,"), (B(",,,"), Thursday)),
        (B("fr,,,"), (B(",,,"), Friday)),
        (B("Sa,,,"), (B(",,,"), Saturday)),
    ];
    for d in rest {
        assert_eq!(weekday.parse_peek(d.0), Ok(d.1));
    }
}
