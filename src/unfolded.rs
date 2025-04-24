use memchr::memchr;
use std::io::{self, ErrorKind};
use thiserror::Error;

/// Reads content lines into `buf`, unfolding long lines as described in
/// [RFC 5545 Section 3.1](https://datatracker.ietf.org/doc/html/rfc5545#section-3.1), except that
/// we accept either CRLF (`b"\r\n"`) or a bare `b'\n'` as a line ending. In either case, when the
/// `b'\n'` is followed by a space (`b' '`) or a tab (`b'\t'`), the line ending and the space or tab
/// are dropped.
///
/// We don't return the line ending.
pub fn read_content_line_u8<R: io::BufRead + ?Sized>(
    r: &mut R,
    buf: &mut Vec<u8>,
) -> Result<usize, io::Error> {
    // Adapted from the rust standard library's `read_until` in `io/mod.rs`
    macro_rules! fill_buf_to {
        ($a:ident) => {
            let $a = match r.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
        };
    }
    let mut lines_read = 0;
    let mut nonline_read = 0;
    loop {
        let (mut saw_newline, consumed) = {
            fill_buf_to!(available);
            //if available.len() == 0 { return Ok(lines_read)}
            match memchr(b'\n', available) {
                Some(newline) => {
                    lines_read += 1;
                    buf.extend_from_slice(&available[..newline]);
                    if buf.last() == Some(&b'\r') {
                        buf.pop();
                    }
                    (true, newline + 1)
                }
                None => {
                    if !available.is_empty() {
                        nonline_read = 1;
                    }
                    buf.extend_from_slice(available);
                    (false, available.len())
                }
            }
        };
        r.consume(consumed);
        if saw_newline {
            fill_buf_to!(available);
            if !available.is_empty() && (available[0] == b'\t' || available[0] == b' ') {
                r.consume(1);
                saw_newline = false;
            }
        }
        if saw_newline {
            return Ok(lines_read);
        } else if consumed == 0 {
            return Ok(lines_read + nonline_read);
            // return Ok(if lines_read == 0 { 0 } else { lines_read + 1 });
        }
    }
}

#[derive(Error, Debug)]
pub enum CalendarError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
}
#[derive(Debug)]
pub struct ContentLines<R> {
    lines_read: usize,
    r: R,
}
pub trait BufReadContent: io::BufRead {
    fn content_lines(self) -> ContentLines<Self>
    where
        Self: Sized,
    {
        ContentLines { lines_read: 1, r: self }
    }
}
impl<R: io::BufRead> BufReadContent for R {}

impl<R: io::BufRead> Iterator for ContentLines<R> {
    type Item = Result<(usize, String), CalendarError>;

    fn next(&mut self) -> Option<Result<(usize, String), CalendarError>> {
        let mut buf = vec![];
        match read_content_line_u8(&mut self.r, &mut buf) {
            Err(e) => Some(Err(e.into())),
            Ok(0) => None,
            Ok(n) => match String::from_utf8(buf) {
                Ok(s) => {
                    let start_of_content_line = self.lines_read;
                    self.lines_read += n;
                    Some(Ok((start_of_content_line, s)))
                }
                Err(e) => Some(Err(e.into())),
            },
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use bstr::ByteSlice;
    use pretty_assertions::assert_eq;

    fn content_lines(input: &str) -> Vec<(usize, String)> {
        let result: Vec<_> =
            io::Cursor::new(input.as_bytes()).content_lines().map(Result::unwrap).collect();
        result
    }
    #[test]
    fn empty() {
        assert!(content_lines("").is_empty());

        let input = b"";
        let mut cursor = io::Cursor::new(input);
        let mut buf = Vec::new();
        let lines = read_content_line_u8(&mut cursor, &mut buf).unwrap();
        assert_eq!(buf.as_bstr(), input.as_bstr());
        assert_eq!(lines, 0);
    }
    #[test]
    fn no_newline() {
        let input = "Without newline";
        assert_eq!(content_lines(input), vec![(1, input.to_string())]);

        let mut cursor = io::Cursor::new(input.as_bytes());
        let mut buf = Vec::new();
        let lines = read_content_line_u8(&mut cursor, &mut buf).unwrap();
        assert_eq!(buf.as_bstr(), input);
        assert_eq!(lines, 1);
    }
    #[test]
    fn one_newline() {
        let input = "One newline\r\n";
        let bare = "One newline";
        assert_eq!(content_lines(input), vec![(1, bare.to_string())]);

        let mut cursor = io::Cursor::new(input.as_bytes());
        let mut buf = Vec::new();
        let lines = read_content_line_u8(&mut cursor, &mut buf).unwrap();
        assert_eq!(buf.as_bstr(), bare);
        assert_eq!(lines, 1);
    }
    #[test]
    fn joined_line() {
        let input = "With newlin\r\n e and without";
        let joined = "With newline and without";
        assert_eq!(content_lines(input), vec![(1, joined.to_string())]);

        let mut cursor = io::Cursor::new(input.as_bytes());
        let mut buf = Vec::new();
        let lines = read_content_line_u8(&mut cursor, &mut buf).unwrap();
        assert_eq!(buf.as_bstr(), joined);
        assert_eq!(lines, 2);
    }
    #[test]
    fn joined_line_small_buffer() {
        let a = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".repeat(5000);
        let b = b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".repeat(5000);
        let c = b"cccccccccccccccccccccccccccccccc".repeat(5000);
        let mut input = a.clone();
        let mut joined = input.clone();
        input.extend_from_slice(b"\r\n ");
        input.extend_from_slice(&b);
        joined.extend_from_slice(&b);
        input.extend_from_slice(b"\n ");
        input.extend_from_slice(&c);
        joined.extend_from_slice(&c);
        let mut reader = io::BufReader::with_capacity(1, io::Cursor::new(&input));
        let mut buf = Vec::new();
        let mut lines = read_content_line_u8(&mut reader, &mut buf).unwrap();
        assert_eq!(buf.as_bstr(), joined.as_bstr());
        assert_eq!(lines, 3);

        input.extend(b"\r\n");
        reader = io::BufReader::with_capacity(1, io::Cursor::new(&input));
        buf = Vec::new();
        lines = read_content_line_u8(&mut reader, &mut buf).unwrap();
        assert_eq!(buf.as_bstr(), joined.as_bstr());
        assert_eq!(lines, 3);

        input.extend(b"\r\n");
        reader = io::BufReader::with_capacity(1, io::Cursor::new(&input));
        buf.clear();
        lines = read_content_line_u8(&mut reader, &mut buf).unwrap();
        assert_eq!(buf.as_bstr(), joined.as_bstr());
        assert_eq!(lines, 3);

        buf.clear();
        lines = read_content_line_u8(&mut reader, &mut buf).unwrap();
        assert_eq!(buf.as_bstr(), b"".as_bstr());
        assert_eq!(lines, 1);
    }
    #[test]
    fn two_content_lines() {
        let first = "With newline";
        let second = "and without";
        let mut input = [first, second].join("\n");
        assert_eq!(content_lines(&input), vec![(1, first.to_string()), (2, second.to_string())]);

        input.push_str("\r\n");
        assert_eq!(content_lines(&input), vec![(1, first.to_string()), (2, second.to_string())]);

        let mut reader = io::BufReader::new(io::Cursor::new(input.as_bytes()));
        let mut buf = Vec::new();
        let lines = read_content_line_u8(&mut reader, &mut buf).unwrap();
        assert_eq!(lines, 1);
        assert_eq!(buf.as_bstr(), first);
        buf.clear();
        let lines = read_content_line_u8(&mut reader, &mut buf).unwrap();
        assert_eq!(lines, 1);
        assert_eq!(buf.as_bstr(), second);
    }
}
