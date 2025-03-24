use std::num::NonZeroI8;

use crate::Weekday;
use bstr::B;

use winnow::ascii::{Caseless, Int, dec_int, dec_uint};
use winnow::combinator::{alt, cut_err, opt, separated};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

#[allow(clippy::missing_panics_doc)]
pub fn parse(input: &mut &[u8]) {
    alt((
        freq,
        count,
        interval,
        by_second,
        by_minute,
        by_hour,
        by_month_day,
        by_year_day,
        wk_st,
    ))
    .parse_next(input)
    .unwrap();
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
type SomeWeekdays = (Option<NonZeroI8>, Weekday);

pub struct Rule {
    freq: Frequency,
    count: Option<usize>,
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

#[test]
fn check_rule_part_errors() {
    check_error(freq, "FREQ=weird", [FREQ_LABEL, FREQ_EXPECTED]);
    check_error(count, "COUNT=nonnumeric", [COUNT_LABEL, COUNT_EXPECTED]);
    check_error(interval, "interval=x", [INTERVAL_LABEL, INTERVAL_EXPECTED]);
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
    let day_context = [BY_DAY_LABEL, BY_DAY_EXPECTED];
    for day_list in ["xx", "12", "-54SU", "54SU", "-54X", "54X"] {
        let input = format!("ByDay={day_list}");
        check_error(by_day, &input, day_context.clone());
    }
    let month_context = [BY_MONTH_DAY_LABEL, BY_MONTH_DAY_EXPECTED];
    for month_day_list in ["xx", "2,XX", "2,-32,4,24", "2,32,-4,24"] {
        let input = format!("ByMONTHDAY={month_day_list}");
        check_error(by_month_day, &input, month_context.clone());
    }
    let year_day_context = [BY_YEAR_DAY_LABEL, BY_YEAR_DAY_EXPECTED];
    for year_day_list in ["xx", "2,XX", "2,-367,4,24", "2,367,-4,24"] {
        let input = format!("ByYEARDAY={year_day_list}");
        check_error(by_year_day, &input, year_day_context.clone());
    }
    let week_no_context = [BY_WEEK_NO_LABEL, BY_WEEK_NO_EXPECTED];
    for week_no_list in ["xx", "2,XX", "2,-54,4,24", "2,54,-4,24"] {
        let input = format!("ByWEEKNO={week_no_list}");
        check_error(by_week_no, &input, week_no_context.clone());
    }
    for months_list in ["xx", "2,XX", "2,3,4,13", "2,0,10"] {
        let input = format!("ByMONTH={months_list}");
        check_error(by_month, &input, [BY_MONTH_LABEL, BY_MONTH_EXPECTED]);
    }
    let set_pos_context = [BY_SET_POS_LABEL, BY_SET_POS_EXPECTED];
    for set_pos_list in ["xx", "2,XX", "2,-367,4,24", "2,367,-4,24"] {
        let input = format!("BySETPOS={set_pos_list}");
        check_error(by_set_pos, &input, set_pos_context.clone());
    }
    check_error(wk_st, "WkSt=xx", [WEEKDAY_LABEL, WEEKDAY_EXPECTED]);
}
#[cfg(test)]
#[allow(clippy::needless_pass_by_value)]
fn check_error<const N: usize>(
    mut parser: impl FnMut(&mut &[u8]) -> ModalResult<RulePart>,
    input: &str,
    expected_context: [StrContext; N],
) {
    use bstr::ByteSlice;
    let equal_sign = input.find('=').expect("Can't find an equal sign (=)");
    let result = parser.parse(B(input));
    assert!(result.is_err(), "Result isn't an error: {result:#?}");
    let err = result.unwrap_err();
    assert!(
        err.offset() > equal_sign,
        "In '{input}', error should happen after the equals sign, but {} <= {equal_sign}",
        err.offset()
    );
    assert_eq!(err.input().as_bstr(), input, "for input {input}");
    assert_eq!(
        err.inner().context().cloned().collect::<Vec<_>>(),
        expected_context,
        "for input {input}"
    );
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

// Parse ByDay
const BY_DAY_LABEL: StrContext = label("day of the week with optional week number");
const BY_DAY_EXPECTED: StrContext = expected(
    "a weekday abbreviation (SU, MO, TU, ...), optionally preceded by a nonzero number between -53 and 53 (-1 is the last week of the month or year",
);
fn by_day(input: &mut &[u8]) -> ModalResult<RulePart> {
    let num = ClampedSigned::<i8> {
        range: -53..=53,
        label: BY_DAY_LABEL,
        expected: BY_DAY_EXPECTED,
    }
    .context(BY_DAY_LABEL)
    .context(BY_DAY_EXPECTED);
    Caseless("BYDAY=").parse_next(input)?;
    let maybe_with_week = (
        opt(num),
        weekday.context(BY_DAY_LABEL).context(BY_DAY_EXPECTED),
    )
        .map(|n_w| (NonZeroI8::new(n_w.0.unwrap_or(0i8)), n_w.1));
    let by_day_list = separated(1.., cut_err(maybe_with_week), b',').parse_next(input)?;
    Ok(RulePart::ByDay(by_day_list))
}
#[test]
fn test_by_day() {
    let z = NonZeroI8::new;
    use Weekday::*;
    assert_eq!(
        by_day.parse_peek(B("byDAY=su;")),
        Ok((B(";"), RulePart::ByDay(vec![(z(0), Sunday)]))),
    );
    let input = B("byday=-53su,-2mo,3tu,53we,+53th;");
    let expected = vec![
        (z(-53), Sunday),
        (z(-2), Monday),
        (z(3), Tuesday),
        (z(53), Wednesday),
        (z(53), Thursday),
    ];
    assert_eq!(
        by_day.parse_peek(input),
        Ok((B(";"), RulePart::ByDay(expected)))
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

// Parse ByYearDay
//
const BY_YEAR_DAY_LABEL: StrContext = label("a list of days");
const BY_YEAR_DAY_EXPECTED: StrContext =
    expected("nonzero numbers between -366 and 366 (-1 is the last day of the year)");
fn by_year_day(input: &mut &[u8]) -> ModalResult<RulePart> {
    let num = ClampedSigned::<i16> {
        range: -366..=366,
        label: BY_YEAR_DAY_LABEL,
        expected: BY_YEAR_DAY_EXPECTED,
    };
    Caseless("BYYEARDAY=").parse_next(input)?;
    let year_day_list = separated(1.., cut_err(num), b',').parse_next(input)?;
    Ok(RulePart::ByYearDay(year_day_list))
}
#[test]
fn test_by_year_day() {
    assert_eq!(
        by_year_day.parse_peek(B("byYEARDAY=-12;")),
        Ok((B(";"), RulePart::ByYearDay(vec![-12i16]))),
    );
    assert_eq!(
        by_year_day.parse_peek(B("byYearDay=-366,+2,3,31,+366,366;")),
        Ok((
            B(";"),
            RulePart::ByYearDay(vec![-366i16, 2i16, 3i16, 31i16, 366i16, 366i16])
        )),
    );
}

// Parse ByWeekNo
//
const BY_WEEK_NO_LABEL: StrContext = label("a list of weeks");
const BY_WEEK_NO_EXPECTED: StrContext =
    expected("nonzero numbers between -53 and 53 (-1 is the last week of the year)");
fn by_week_no(input: &mut &[u8]) -> ModalResult<RulePart> {
    let num = ClampedSigned::<i8> {
        range: -53..=53,
        label: BY_WEEK_NO_LABEL,
        expected: BY_WEEK_NO_EXPECTED,
    };
    Caseless("BYWEEKNO=").parse_next(input)?;
    let week_no_list = separated(1.., cut_err(num), b',').parse_next(input)?;
    Ok(RulePart::ByWeekNo(week_no_list))
}
#[test]
fn test_by_week_no() {
    assert_eq!(
        by_week_no.parse_peek(B("byWEEKNO=-12;")),
        Ok((B(";"), RulePart::ByWeekNo(vec![-12i8]))),
    );
    assert_eq!(
        by_week_no.parse_peek(B("byWeekNo=-53,+2,3,53,+53;")),
        Ok((
            B(";"),
            RulePart::ByWeekNo(vec![-53i8, 2i8, 3i8, 53i8, 53i8])
        )),
    );
}
// Parse ByMonth
//
const BY_MONTH_LABEL: StrContext = label("a list of month numbers");
const BY_MONTH_EXPECTED: StrContext = expected("numbers between 1 and 12");
fn by_month(input: &mut &[u8]) -> ModalResult<RulePart> {
    let num = Clamped {
        range: 1..=12,
        label: BY_MONTH_LABEL,
        expected: BY_MONTH_EXPECTED,
    };
    Caseless("BYMONTH=").parse_next(input)?;
    let month_list = separated(1.., cut_err(num), b',').parse_next(input)?;
    Ok(RulePart::ByMonth(month_list))
}
#[test]
fn test_by_month() {
    assert_eq!(
        by_month.parse_peek(B("byMONTH=12;")),
        Ok((B(";"), RulePart::ByMonth(vec![12u8]))),
    );
    assert_eq!(
        by_month.parse_peek(B("bymonth=1,2,3,12;")),
        Ok((B(";"), RulePart::ByMonth(vec![1u8, 2u8, 3u8, 12u8]))),
    );
}

// Parse BySetPos
//
const BY_SET_POS_LABEL: StrContext = label("a list of day positions");
const BY_SET_POS_EXPECTED: StrContext = expected(
    "nonzero numbers between -366 and 366 (-1 is the last day created by the other BYxxx rules)",
);
fn by_set_pos(input: &mut &[u8]) -> ModalResult<RulePart> {
    let num = ClampedSigned::<i16> {
        range: -366..=366,
        label: BY_SET_POS_LABEL,
        expected: BY_SET_POS_EXPECTED,
    };
    Caseless("BYSETPOS=").parse_next(input)?;
    let year_day_list = separated(1.., cut_err(num), b',').parse_next(input)?;
    Ok(RulePart::BySetPos(year_day_list))
}
#[test]
fn test_by_set_pos() {
    assert_eq!(
        by_set_pos.parse_peek(B("bySetPos=-12;")),
        Ok((B(";"), RulePart::BySetPos(vec![-12i16]))),
    );
    assert_eq!(
        by_set_pos.parse_peek(B("bySetPos=-366,+2,3,31,+366,366;")),
        Ok((
            B(";"),
            RulePart::BySetPos(vec![-366i16, 2i16, 3i16, 31i16, 366i16, 366i16])
        )),
    );
}

// Parse WkSt and Weekday
//
fn wk_st(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("WKST=").parse_next(input)?;
    let result = weekday
        .context(WEEKDAY_LABEL)
        .context(WEEKDAY_EXPECTED)
        .parse_next(input);
    Ok(RulePart::WkSt(result?))
}
#[test]
fn test_wk_st() {
    assert_eq!(
        wk_st.parse_peek(B("WkST=SU,")),
        Ok((B(","), RulePart::WkSt(Weekday::Sunday)))
    );
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
const WEEKDAY_LABEL: StrContext = label("weekday");
const WEEKDAY_EXPECTED: StrContext = expected("Weekday abbreviation: SU, MO, TU, WE, TH, FR, SA");
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
