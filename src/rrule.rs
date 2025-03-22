use std::f32::consts::FRAC_1_PI;

use crate::Weekday;
use bstr::{B, BStr};

use winnow::ascii::{Caseless, digit1};
use winnow::combinator::{alt, cut_err, separated};
use winnow::error::{
    ContextError, EmptyError, ErrMode, Needed, ParserError, StrContext, StrContextValue,
};
use winnow::prelude::*;
use winnow::token::literal;
use winnow::{ModalResult, Parser, Result};

#[derive(Debug, PartialEq, Eq, Clone)]
enum RulePart {
    Freq(Frequency),
    Count(usize),
    Interval(usize),
    WkSt(Weekday),
}

#[test]
fn check_rule_part_errors() {
    check_error(freq, "FREQ=weird", [FREQ_LABEL, FREQ_EXPECTED]);
    check_error(count, "COUNT=nonnumeric", [COUNT_LABEL, COUNT_EXPECTED]);
    check_error(interval, "interval=x", [INTERVAL_LABEL, INTERVAL_EXPECTED]);
    check_error(wk_st, "WkSt=xx", [WEEKDAY_LABEL, WEEKDAY_EXPECTED]);
}
#[cfg(test)]
fn check_error<const N: usize>(
    mut parser: impl FnMut(&mut &[u8]) -> ModalResult<RulePart>,
    input: &str,
    context: [StrContext; N],
) {
    let equal_sign = input.find('=').expect("Can't find an equal sign (=)");
    let input = B(input);
    let err = parser.parse(input).unwrap_err();
    assert!(
        err.offset() > equal_sign,
        "Error should happen after the equals sign"
    );
    assert_eq!(B(err.input()), input);
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

// Parse Count
//
const COUNT_LABEL: StrContext = label("occurrences");
const COUNT_EXPECTED: StrContext = expected("Number of occurrences of the repeating event");
fn count(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("COUNT=").parse_next(input)?;
    let n = digit1
        .parse_to()
        .context(COUNT_LABEL)
        .context(COUNT_EXPECTED)
        .parse_next(input)?;
    Ok(RulePart::Count(n))
}

// Parse Interval
//

const INTERVAL_LABEL: StrContext = label("interval");
const INTERVAL_EXPECTED: StrContext = expected("How often the repeating event occurs");
fn interval(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("INTERVAL=").parse_next(input)?;
    let n = digit1
        .parse_to()
        .context(INTERVAL_LABEL)
        .context(INTERVAL_EXPECTED)
        .parse_next(input)?;
    Ok(RulePart::Interval(n))
}

// Parse WkSt and Weekday
//
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

fn wk_st(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("WKST=").parse_next(input)?;
    Ok(RulePart::WkSt(weekday(input)?))
}

// Tests //////////////////////////////////////////////////////////
#[cfg(test)]
mod test {
    #![allow(clippy::pedantic)]
    use super::*;
    use winnow::error::ParseError;

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

    #[test]
    fn test_freq() {
        assert_eq!(
            freq.parse_peek(B("FREQ=YeaRly,")),
            Ok((B(","), RulePart::Freq(Frequency::Yearly)))
        );
    }

    #[test]
    fn test_count() {
        assert_eq!(
            count.parse_peek(B("count=42,")),
            Ok((B(","), RulePart::Count(42)))
        );
    }

    #[test]
    fn test_interval() {
        assert_eq!(
            interval.parse_peek(B("interval=42,")),
            Ok((B(","), RulePart::Interval(42)))
        );
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
    #[test]
    fn test_wk_st() {
        assert_eq!(
            wk_st.parse_peek(B("WkST=SU,")),
            Ok((B(","), RulePart::WkSt(Weekday::Sunday)))
        );
    }
}
