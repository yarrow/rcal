#![allow(clippy::pedantic)]
use bstr::BString;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rcal::preparse;
use rcal::unfolded::BufReadContent;

fn discard(stuff: &preparse::Prop) -> u8 {
    if stuff.name.loc == 0 { black_box(b'0') } else { black_box(b'1') }
}
#[allow(unused_variables)]
fn bold_preparse_and_discard(lines: &[BString]) -> Vec<u8> {
    #[cfg(feature = "bold")]
    {
        lines.iter().map(|line| discard(&preparse::bold_preparse(line).unwrap())).collect()
    }
    #[cfg(not(feature = "bold"))]
    {
        unimplemented!("\n\nTry cargo bench --all-features\n")
    }
}
#[allow(unused_variables)]
fn cautious_preparse_and_discard(lines: &[BString]) -> Vec<u8> {
    #[cfg(feature = "cautious")]
    {
        lines.iter().map(|line| discard(&preparse::cautious_preparse(line).unwrap())).collect()
    }
    #[cfg(not(feature = "cautious"))]
    {
        unimplemented!("\n\nTry cargo bench --all-features\n")
    }
}
pub fn compare_preparsers(c: &mut Criterion) {
    let mut group = c.benchmark_group("Preparsers");
    group.sample_size(500);
    group.measurement_time(std::time::Duration::new(10, 0));
    let path = "/Users/yarrow/rust/rcal/notes/studio/ics/Yarrow_yarrow.angelweed@gmail.com.ics";
    let input = std::fs::read_to_string(path).unwrap();
    let iter = std::io::Cursor::new(input.as_bytes()).content_lines().map(Result::unwrap);
    let mut lines = Vec::new();
    for line in iter {
        lines.push(line.1);
    }
    group.bench_with_input(BenchmarkId::new("Plain", "Events-Calendar"), &lines, |b, lines| {
        b.iter(|| bold_preparse_and_discard(black_box(lines)))
    });
    group.bench_with_input(BenchmarkId::new("Regex", "Events-Calendar"), &lines, |b, lines| {
        b.iter(|| cautious_preparse_and_discard(black_box(lines)))
    });
}

criterion_group!(benches, compare_preparsers);
criterion_main!(benches);
