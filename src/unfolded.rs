#[derive(Debug, PartialEq)]
pub struct Line<'a> {
    number: usize,
    line: &'a [u8],
}
pub type Unfolded<'a> = Vec<Line<'a>>;

pub fn unfold(text: &mut [u8]) -> Unfolded {
    vec![Line { number: 1, line: text }]
}

#[cfg(test)]
mod test {
    use super::*;
    use bstr::B;

    #[test]
    fn already_unfolded() {
        let mut text = B("foo\r\n").to_owned();
        let orig = text.clone();
        let result = unfold(&mut text);
        assert_eq!(result, vec![Line { number: 1, line: &orig }]);
    }
}
