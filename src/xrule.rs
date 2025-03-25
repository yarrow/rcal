#![allow(non_upper_case_globals)]

use std::num::NonZeroI8;
use std::ops::RangeInclusive;
use std::str::FromStr;

use paste::paste;

use crate::Weekday;
use bstr::{B, ByteSlice};
use memchr::{Memchr3, memchr};

use winnow::ascii::{Caseless, Int, dec_int, dec_uint};
use winnow::combinator::{alt, cut_err, fail, opt, separated};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

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

//==============================================================================
macro_rules! u8_rule {
    ($name: ident, $min:literal, $max:literal) => {
        paste! {
            const [<$name _u8>]: &[u8] = stringify!([<$name:upper>]).as_bytes();
            const [<$name List>]: U8list = U8list::new(
                concat!(stringify!($name), " takes a list of numbers from ", $min, " to ", $max),
                $min ..= $max
                );
        }
    };
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
fn one_part(input: &mut &[u8]) -> ModalResult<RulePart> {
    u8_rule!(BySecond, 0, 60);
    u8_rule!(ByMinute, 0, 59);
    u8_rule!(ByHour, 0, 59);
    u8_rule!(ByMonth, 1, 12);

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
        BySecond_u8 => BySecond(BySecondList.clone().parse_next(input)?),
        ByMinute_u8 => ByMinute(ByMinuteList.clone().parse_next(input)?),
        ByHour_u8 => ByHour(ByHourList.clone().parse_next(input)?),
        ByMonth_u8 => ByMonth(ByMonthList.clone().parse_next(input)?),
        _ => fail.parse_next(input)?,
    })
}

//==============================================================================
#[cfg(test)]
#[allow(non_snake_case)]
mod test {
    use std::f32::consts::E;

    use super::*;
    use RulePart::*;

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
        check_ok("byHour=0,1,2,3,59", ByHour(vec![0u8, 1u8, 2u8, 3u8, 59u8]));
        check_err("ByHour", ["x1", "-1", "60"]);
    }
    #[test]
    fn test_ByMonth() {
        check_ok("byMonth=1,2,3,12", ByMonth(vec![1u8, 2u8, 3u8, 12u8]));
        check_err("ByMonth", ["x1", "0", "13"]);
    }
}
