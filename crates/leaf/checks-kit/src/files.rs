//! The syntax of what a file check's value says beside its path: a line range and a digest pin. The path itself is the
//! host's to read: only it knows the `<root>/` prefix and where a path may lead.

use std::ops::Range;

use crate::Invalid;

/// `a.rs:10` or `a.rs:10-40`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRange<'a> {
    pub path: &'a str,
    /// The first line, from 1.
    pub start: u32,
    /// The last line; `start` for a single line.
    pub end: u32,
    /// The bytes of the numbers.
    pub at: Range<usize>,
}

impl<'a> LineRange<'a> {
    pub fn parse(text: &'a str) -> Result<LineRange<'a>, Invalid> {
        let Some((path, lines)) = text.rsplit_once(':') else {
            return Err(Invalid::new(
                "a line range ends in `:<line>` or `:<first>-<last>`",
            ));
        };
        let at = path.len() + 1..text.len();
        if path.is_empty() {
            return Err(Invalid::at("there is no path before the lines", 0..at.end));
        }
        let number = |written: &str| {
            (!written.is_empty() && written.bytes().all(|byte| byte.is_ascii_digit()))
                .then(|| written.parse::<u32>().ok())
                .flatten()
                .filter(|line| *line > 0)
                .ok_or_else(|| Invalid::at("a line is a number from 1", at.clone()))
        };
        let (start, end) = match lines.split_once('-') {
            Some((start, end)) => (number(start)?, number(end)?),
            None => {
                let line = number(lines)?;
                (line, line)
            }
        };
        if start > end {
            return Err(Invalid::at(format!("line {start} is after line {end}"), at));
        }
        Ok(LineRange {
            path,
            start,
            end,
            at,
        })
    }
}

/// How a text's lines end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnds {
    None,
    Lf,
    Crlf,
    Mixed,
}

/// What a file's bytes are, as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text {
    pub utf8: bool,
    pub bom: bool,
    pub line_ends: LineEnds,
}

impl Text {
    pub fn classify(bytes: &[u8]) -> Text {
        let (mut lf, mut crlf) = (false, false);
        let mut previous = 0;
        for byte in bytes {
            if *byte == b'\n' {
                if previous == b'\r' {
                    crlf = true;
                } else {
                    lf = true;
                }
            }
            previous = *byte;
        }
        Text {
            utf8: std::str::from_utf8(bytes).is_ok(),
            bom: bytes.starts_with(b"\xEF\xBB\xBF"),
            line_ends: match (lf, crlf) {
                (false, false) => LineEnds::None,
                (true, false) => LineEnds::Lf,
                (false, true) => LineEnds::Crlf,
                (true, true) => LineEnds::Mixed,
            },
        }
    }
}

/// `path@sha256:<64 hex>`: a pin written in the value, so it works in a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestPin<'a> {
    pub path: &'a str,
    /// Lower case.
    pub sha256: String,
    /// The bytes of the digest.
    pub at: Range<usize>,
}

impl<'a> DigestPin<'a> {
    pub fn parse(text: &'a str) -> Result<DigestPin<'a>, Invalid> {
        let Some((path, hex)) = text.rsplit_once("@sha256:") else {
            return Err(Invalid::new(
                "a digest pin ends in `@sha256:<64 hex digits>`",
            ));
        };
        let at = text.len() - hex.len()..text.len();
        if path.is_empty() {
            return Err(Invalid::at(
                "there is no path before the digest",
                0..at.start,
            ));
        }
        if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Invalid::at("a sha256 digest is 64 hex digits", at));
        }
        Ok(DigestPin {
            path,
            sha256: hex.to_ascii_lowercase(),
            at,
        })
    }
}

/// The globs a file check may ask the host to walk.
pub mod glob {
    use crate::Invalid;

    /// The longest pattern a value may hold.
    const MAX_LENGTH: usize = 1024;

    /// `pattern` is relative, `/`-separated and goes nowhere upward. What it matches is the host's to decide.
    pub fn validate(pattern: &str) -> Result<(), Invalid> {
        if pattern.is_empty() || pattern.len() > MAX_LENGTH {
            return Err(Invalid::new(format!("a glob has 1 to {MAX_LENGTH} bytes")));
        }
        if pattern.starts_with('/') {
            return Err(Invalid::at("a glob is relative", 0..1));
        }
        if let Some(at) = pattern.find('\\') {
            return Err(Invalid::at("a glob separates with `/`", at..at + 1));
        }
        let mut at = 0;
        for segment in pattern.split('/') {
            if segment == ".." {
                return Err(Invalid::at("a glob does not climb", at..at + 2));
            }
            at += segment.len() + 1;
        }
        Ok(())
    }
}
