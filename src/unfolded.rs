pub(crate) struct Line<'a> {
    number: usize,
    line: &'a [u8],
}
pub(crate) type Unfolded<'a> = Vec<Line>;

fn unfold<'a>(text: &mut'a str) -> Unfolded<'a> {
    unimplemented!()
}


