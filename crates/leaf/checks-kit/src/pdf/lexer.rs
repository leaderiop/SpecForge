//! The values of a PDF: its objects and the lexer that reads them from served bytes.

use super::{MAX_DEPTH, Span, Stop, bad};

/// A value of the file.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Obj {
    Null,
    Bool,
    Int(i64),
    Real,
    Name(Vec<u8>),
    Str,
    Array(Vec<Obj>),
    Dict(Dict),
    Ref(u32),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Dict(Vec<(Vec<u8>, Obj)>);

impl Dict {
    pub(super) fn get(&self, key: &[u8]) -> Option<&Obj> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub(super) fn int(&self, key: &[u8]) -> Option<i64> {
        match self.get(key) {
            Some(Obj::Int(n)) => Some(*n),
            _ => None,
        }
    }

    pub(super) fn reference(&self, key: &[u8]) -> Option<u32> {
        match self.get(key) {
            Some(Obj::Ref(n)) => Some(*n),
            _ => None,
        }
    }

    pub(super) fn is(&self, key: &[u8], name: &[u8]) -> bool {
        matches!(self.get(key), Some(Obj::Name(n)) if n == name)
    }
}

/// Reads values from served bytes; running out of them is a need for `need`, unless they reach the file's end.
pub(super) struct Lexer<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) at: usize,
    pub(super) end: bool,
    pub(super) need: Span,
}

pub(super) fn is_white(byte: u8) -> bool {
    matches!(byte, 0 | b'\t' | b'\n' | 0x0c | b'\r' | b' ')
}

pub(super) fn is_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

impl<'a> Lexer<'a> {
    pub(super) fn short<T>(&self) -> Result<T, Stop> {
        if self.end {
            bad("it ends inside an object")
        } else {
            Err(Stop::Short(self.need))
        }
    }

    pub(super) fn peek(&self) -> Result<u8, Stop> {
        match self.bytes.get(self.at) {
            Some(byte) => Ok(*byte),
            None => self.short(),
        }
    }

    /// Skip white space and comments.
    pub(super) fn white(&mut self) {
        while let Some(&byte) = self.bytes.get(self.at) {
            if is_white(byte) {
                self.at += 1;
            } else if byte == b'%' {
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| !matches!(b, b'\n' | b'\r'))
                {
                    self.at += 1;
                }
            } else {
                break;
            }
        }
    }

    /// A run of regular characters: a keyword, a number or a name's text.
    pub(super) fn word(&mut self) -> Result<&'a [u8], Stop> {
        let start = self.at;
        while let Some(&byte) = self.bytes.get(self.at) {
            if is_white(byte) || is_delimiter(byte) {
                return Ok(&self.bytes[start..self.at]);
            }
            self.at += 1;
        }
        // A word at the end of what was served may go on past it.
        if self.end {
            Ok(&self.bytes[start..])
        } else {
            Err(Stop::Short(self.need))
        }
    }

    /// Whether `keyword` comes next, after white space; it is taken when it does.
    pub(super) fn keyword(&mut self, keyword: &[u8]) -> Result<bool, Stop> {
        self.white();
        let start = self.at.min(self.bytes.len());
        let rest = &self.bytes[start..];
        if !rest.starts_with(keyword) {
            if rest.len() < keyword.len() && keyword.starts_with(rest) && !self.end {
                return Err(Stop::Short(self.need));
            }
            return Ok(false);
        }
        self.at += keyword.len();
        match self.bytes.get(self.at) {
            Some(&byte) if !is_white(byte) && !is_delimiter(byte) => {
                self.at = start;
                Ok(false)
            }
            None if !self.end => Err(Stop::Short(self.need)),
            _ => Ok(true),
        }
    }

    pub(super) fn int(&mut self) -> Result<i64, Stop> {
        self.white();
        let word = self.word()?;
        parse_int(word).map_or_else(|| bad("a number is expected"), Ok)
    }

    pub(super) fn value(&mut self, depth: usize) -> Result<Obj, Stop> {
        if depth > MAX_DEPTH {
            return bad(format!("it nests deeper than {MAX_DEPTH} levels"));
        }
        self.white();
        match self.peek()? {
            b'/' => {
                self.at += 1;
                Ok(Obj::Name(self.word()?.to_vec()))
            }
            b'(' => self.literal(),
            b'<' if self.bytes.get(self.at + 1) == Some(&b'<') => {
                self.at += 2;
                let mut dict = Dict::default();
                loop {
                    self.white();
                    if self.peek()? == b'>' {
                        if self.bytes.get(self.at + 1) == Some(&b'>') {
                            self.at += 2;
                            return Ok(Obj::Dict(dict));
                        }
                        return self.short();
                    }
                    let Obj::Name(key) = self.value(depth + 1)? else {
                        return bad("a dictionary key is not a name");
                    };
                    let value = self.value(depth + 1)?;
                    dict.0.push((key, value));
                }
            }
            b'<' => {
                match self.bytes[self.at..].iter().position(|b| *b == b'>') {
                    Some(end) => self.at += end + 1,
                    None => return self.short(),
                }
                Ok(Obj::Str)
            }
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                loop {
                    self.white();
                    if self.peek()? == b']' {
                        self.at += 1;
                        return Ok(Obj::Array(items));
                    }
                    items.push(self.value(depth + 1)?);
                }
            }
            _ => {
                let word = self.word()?;
                match word {
                    b"null" => Ok(Obj::Null),
                    b"true" | b"false" => Ok(Obj::Bool),
                    _ => match parse_int(word) {
                        Some(number) => self.after_int(number),
                        None if parse_real(word) => Ok(Obj::Real),
                        None => bad(format!("'{}' is no value", String::from_utf8_lossy(word))),
                    },
                }
            }
        }
    }

    /// `number`, or a reference `number generation R`.
    pub(super) fn after_int(&mut self, number: i64) -> Result<Obj, Stop> {
        let back = self.at;
        self.white();
        let generation = match self.bytes.get(self.at) {
            Some(byte) if byte.is_ascii_digit() => self.word().ok().and_then(parse_int),
            None if !self.end => return self.short(),
            _ => None,
        };
        if generation.is_some() && number >= 0 && self.keyword(b"R")? {
            let object = u32::try_from(number)
                .map_err(|_| Stop::Bad("an object number is too large".into()))?;
            return Ok(Obj::Ref(object));
        }
        self.at = back;
        Ok(Obj::Int(number))
    }

    /// A literal string: balanced parentheses, a backslash escaping the next byte.
    pub(super) fn literal(&mut self) -> Result<Obj, Stop> {
        let mut depth = 0usize;
        while let Some(&byte) = self.bytes.get(self.at) {
            self.at += 1;
            match byte {
                b'\\' => self.at += 1,
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(Obj::Str);
                    }
                }
                _ => {}
            }
        }
        self.short()
    }

    /// An indirect object `number generation obj`, its value, and its stream's bytes when it has one.
    pub(super) fn object(&mut self, number: u32) -> Result<(Obj, Option<&'a [u8]>), Stop> {
        let found = self.int()?;
        let _generation = self.int()?;
        if !self.keyword(b"obj")? {
            return bad(format!(
                "object {number} is not at the offset its cross-reference gives"
            ));
        }
        if found != i64::from(number) {
            return bad(format!(
                "object {number} is not at the offset its cross-reference gives"
            ));
        }
        let value = self.value(0)?;
        if !self.keyword(b"stream")? {
            return Ok((value, None));
        }
        // The data starts after the keyword's end of line.
        match self.bytes.get(self.at) {
            Some(b'\r') if self.bytes.get(self.at + 1) == Some(&b'\n') => self.at += 2,
            Some(b'\n' | b'\r') => self.at += 1,
            Some(_) => {}
            None => return self.short(),
        }
        let length = match &value {
            Obj::Dict(dict) => dict.int(b"Length"),
            _ => None,
        };
        let rest = &self.bytes[self.at..];
        let data = match length.and_then(|n| usize::try_from(n).ok()) {
            Some(length) if length <= rest.len() => &rest[..length],
            Some(_) => return self.short(),
            None => match find(rest, b"endstream") {
                Some(end) => trim_eol(&rest[..end]),
                None => return self.short(),
            },
        };
        Ok((value, Some(data)))
    }
}

pub(super) fn parse_int(word: &[u8]) -> Option<i64> {
    let text = std::str::from_utf8(word).ok()?;
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

pub(super) fn parse_real(word: &[u8]) -> bool {
    let text = std::str::from_utf8(word).unwrap_or("");
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    digits.bytes().filter(|b| *b == b'.').count() == 1
        && digits.len() > 1
        && digits.bytes().all(|b| b.is_ascii_digit() || b == b'.')
}

pub(super) fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

pub(super) fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

pub(super) fn trim_eol(bytes: &[u8]) -> &[u8] {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    bytes.strip_suffix(b"\r").unwrap_or(bytes)
}

impl Lexer<'_> {
    /// The integer that comes next, after white space, which stays to be read.
    pub(super) fn peek_int(&mut self) -> Option<i64> {
        self.white();
        let start = self.at;
        let number = self.word().ok().and_then(parse_int);
        self.at = start;
        number
    }
}
