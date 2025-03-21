use std::f32::consts::FRAC_1_PI;

use crate::Weekday;
use bstr::{B, BStr};

use winnow::ascii::{Caseless, digit1};
use winnow::combinator::{alt, cut_err};
use winnow::error::{ContextError, ErrMode, Needed, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::literal;
use winnow::{ModalResult, Parser};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum RulePart {
    Freq(Frequency),
    Count(usize),
    WkSt(Weekday),
}

fn freq(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("FREQ=").parse_next(input)?;
    Ok(RulePart::Freq(frequency(input)?))
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

const FREQUENCY_LABEL: StrContext = StrContext::Label("frequency");
const FREQUENCY_EXPECTED: StrContext = StrContext::Expected(StrContextValue::Description(
    "Frequency: DAILY, WEEKLY, MONTHLY, etc",
));
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
    .context(FREQUENCY_LABEL)
    .context(FREQUENCY_EXPECTED)
    .parse_next(input)
}

const COUNT_LABEL: StrContext = StrContext::Label("occurences");
const COUNT_EXPECTED: StrContext = StrContext::Expected(StrContextValue::Description(
    "Number of occurrences of the repeating event",
));
fn count(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("COUNT=").parse_next(input)?;
    let n = digit1
        .parse_to()
        .context(COUNT_LABEL)
        .context(COUNT_EXPECTED)
        .parse_next(input)?;
    Ok(RulePart::Count(n))
}
fn wk_st(input: &mut &[u8]) -> ModalResult<RulePart> {
    Caseless("WKST=").parse_next(input)?;
    Ok(RulePart::WkSt(weekday(input)?))
}

const WEEKDAY_LABEL: StrContext = StrContext::Label("weekday");
const WEEKDAY_EXPECTED: StrContext = StrContext::Expected(StrContextValue::Description(
    "Weekday abbreviation: SU, MO, TU, WE, TH, FR, SA",
));
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
    fn freq_error() {
        let input = B("FREQ=Zekondly");
        let err = freq.parse(input).unwrap_err();
        assert_eq!(err.offset(), 5);
        assert_eq!(B(err.input()), input);
        check_context(err, vec![FREQUENCY_LABEL, FREQUENCY_EXPECTED])
    }

    fn check_context(err: ParseError<&[u8], ContextError>, contexts: Vec<StrContext>) {
        assert_eq!(err.inner().context().cloned().collect::<Vec<_>>(), contexts);
    }

    #[test]
    fn test_count() {
        assert_eq!(
            count.parse_peek(B("count=42,")),
            Ok((B(","), RulePart::Count(42)))
        );
    }

    #[test]
    fn count_error() {
        check_context(
            count.parse(B("count=-1")).unwrap_err(),
            vec![COUNT_LABEL, COUNT_EXPECTED],
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

    #[test]
    fn wk_st_error() {
        check_context(
            wk_st.parse(B("wkst=XX")).unwrap_err(),
            vec![WEEKDAY_LABEL, WEEKDAY_EXPECTED],
        )
    }
}
