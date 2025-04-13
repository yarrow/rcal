use bstr::BString;
use bstr::io::{BufReadExt, ByteLines};
use std::io;
use std::iter::{Fuse, FusedIterator};

/// An iterator over the unfolded content lines of an iCal calendar file, annotated with the
/// starting line number (in the original, folded source) where the unfolded line starts.
#[derive(Debug)]
pub struct Unfolded<B: io::BufRead> {
    source_lines: Fuse<ByteLines<B>>, // We may call `source_lines.next()` after it's returned `None`
    lines_read: usize,                // How many source lines have we read?
    start_of_content_line: usize,     // Which source line is the start of the current content line
    waiting: Option<BString>,         // When we read a source line that *doesn't* start with a
                                      // space or tab, we return the content line we were working
                                      // on, and store the source line in `waiting` to start the
                                      // next content line.
}

impl<B: io::BufRead> Unfolded<B> {
    /// Returns an iterator that returns `(n, line)` pairs where `n` is the starting line number
    /// (in the original, folded source) and `line` is the unfolded iCal content line starting at
    /// `n`. The line in each pair is a`bstr::BString`. From the `bstr` documentation:
    ///
    /// > Byte strings are just like standard Unicode strings with one very important
    /// > difference: byte strings are only *conventionally* UTF-8 while Rust’s standard
    /// > Unicode strings are *guaranteed* to be valid UTF-8.
    ///
    /// We must treat the folded source file lines as byte strings, since RFC 5545 warns:
    ///
    /// > Note: It is possible for very simple implementations to generate
    /// > improperly folded lines in the middle of a UTF-8 multi-octet
    /// > sequence.  For this reason, implementations need to unfold lines
    /// > in such a way to properly restore the original sequence.
    ///
    /// We keep the byte string representation unless and until we need to print or return a string
    /// — at which point we use the `bstr` crate's `to_string` to create a valid UTF-8 by
    /// substituting the Unicode replacement codepoint (�) for invalid UTF-8 bytes.
    ///
    /// We allow either `\r\n` or just `\n` as line endings in the source file.
    ///
    pub fn lines(reader: B) -> Self {
        Self {
            source_lines: reader.byte_lines().fuse(),
            lines_read: 0,
            start_of_content_line: 0,
            waiting: None,
        }
    }
}

impl<B: io::BufRead> FusedIterator for Unfolded<B> {}
impl<B: io::BufRead> Iterator for Unfolded<B> {
    type Item = Result<(usize, BString), io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        // Here we prime the pump, setting `content_line` to `self.waiting` if available,
        // or reading a fresh `content_line` if not.
        let mut content_line = match self.waiting.take() {
            Some(start_of_line) => start_of_line,
            None => match self.source_lines.next() {
                None => return None,
                Some(Err(e)) => return Some(Err(e)),
                Some(Ok(start_of_line)) => {
                    self.lines_read += 1;
                    start_of_line.into()
                }
            },
        };
        self.start_of_content_line = self.lines_read;

        // At this point we know:
        // * `content_line` is the source line with number `self.lines_read`
        // * `self.start_of_content_line == self.lines_read`
        //
        // The following loop adds source lines that start with a space or tab
        // to `content_line`, returning `content_line` when that's no longer
        // possible.
        loop {
            self.lines_read += 1;
            match self.source_lines.next() {
                None => return Some(Ok((self.start_of_content_line, content_line))),
                Some(Err(e)) => return Some(Err(e)),
                Some(Ok(next_part)) => match next_part.first() {
                    Some(b' ' | b'\t') => content_line.extend(&next_part[1..]),
                    _ => {
                        self.waiting = Some(next_part.into());
                        return Some(Ok((self.start_of_content_line, content_line)));
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bstr::B;
    //use pretty_assertions::assert_eq;

    #[test]
    fn empty_string() {
        let input = B("");
        let cursor = io::Cursor::new(input);
        let result: Vec<_> = Unfolded::lines(cursor).map(Result::unwrap).collect();
        assert_eq!(result, vec![]);
    }
    #[test]
    fn already_unfolded() {
        let input = B("foo\r\nbar\r\n");
        let cursor = io::Cursor::new(input);
        let result: Vec<_> = Unfolded::lines(cursor).map(Result::unwrap).collect();
        let expected = vec![(1, B("foo").into()), (2, B("bar").into())];
        assert_eq!(result, expected);
    }
    #[test]
    fn folded() {
        let input = B("OF\r\n F\r\nb\r\n\tar\r\n");
        let cursor = io::Cursor::new(input);
        let result: Vec<_> = Unfolded::lines(cursor).map(Result::unwrap).collect();
        let expected: Vec<_> = vec![(1, B("OFF").into()), (3, B("bar").into())];
        assert_eq!(result, expected);
    }
}
