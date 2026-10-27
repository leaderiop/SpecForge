//! Where a JSON key is written (ADR 0288): the span an E086, E087 or W166 about `specforge.json` points at, and
//! the path under a cursor. `serde_json` keeps no positions, so this scans the text once and records, for every
//! object member and array item, where its key and value are written.
//!
//! It agrees with `serde_json` on what a key is: escapes are decoded, and of a repeated key in one object only the
//! last is live. A text that is not JSON is indexed as far as it is well formed.

use std::ops::Range;

use crate::{SourceSpan, Sym};

/// One object member or array item: its path from the root (an array item's segment is its index), where its key
/// is written (none for an array item or the root) and where its value is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub path: Vec<String>,
    pub key: Option<Range<usize>>,
    pub value: Range<usize>,
    /// An earlier member of a repeated key: `serde_json` keeps the last.
    superseded: bool,
}

/// Every member and item of a JSON text, parents before their children, in the order written.
#[derive(Debug, Clone, Default)]
pub struct JsonIndex {
    entries: Vec<Located>,
}

/// Index `text`.
pub fn index(text: &str) -> JsonIndex {
    let mut scan = Scan {
        text: text.as_bytes(),
        at: 0,
        entries: Vec::new(),
    };
    scan.skip_space();
    let start = scan.at;
    scan.entries.push(Located {
        path: Vec::new(),
        key: None,
        value: start..start,
        superseded: false,
    });
    let _ = scan.value(&mut Vec::new(), 0);
    // A value that stops short (a half-typed one) still covers what was read of it.
    scan.entries[0].value.end = scan.at.max(start);
    JsonIndex {
        entries: scan.entries,
    }
}

/// The live member at `path` in `text`; `None` when it is not written.
pub fn locate(text: &str, path: &[&str]) -> Option<Located> {
    index(text).locate(path).cloned()
}

/// The deepest member whose key or value covers byte `offset` of `text`.
pub fn path_at(text: &str, offset: usize) -> Option<Located> {
    index(text).path_at(offset).cloned()
}

impl JsonIndex {
    /// The live member at `path`.
    pub fn locate(&self, path: &[&str]) -> Option<&Located> {
        self.entries
            .iter()
            .rev()
            .find(|e| !e.superseded && e.path.iter().map(String::as_str).eq(path.iter().copied()))
    }

    /// The live members directly under `path`, in the order written.
    pub fn children<'a>(&'a self, path: &[&str]) -> Vec<&'a Located> {
        self.entries
            .iter()
            .filter(|e| {
                !e.superseded
                    && e.path.len() == path.len() + 1
                    && e.path.iter().zip(path).all(|(a, b)| a == b)
            })
            .collect()
    }

    /// The deepest live member whose key or value covers `offset`. A member's key counts before its value.
    pub fn path_at(&self, offset: usize) -> Option<&Located> {
        self.entries
            .iter()
            .filter(|e| !e.superseded)
            .filter(|e| {
                e.key
                    .as_ref()
                    .is_some_and(|k| k.start <= offset && offset <= k.end)
                    || (e.value.start <= offset && offset <= e.value.end)
            })
            .max_by_key(|e| e.path.len())
    }
}

/// The span of `range` in `text`, in `file`: 1-based lines and columns, columns in characters.
pub fn span(text: &str, file: &str, range: Range<usize>) -> SourceSpan {
    let position = |offset: usize| {
        let before = &text[..offset.min(text.len())];
        let line = before.matches('\n').count() + 1;
        let start = before.rfind('\n').map_or(0, |i| i + 1);
        (line, before[start..].chars().count() + 1)
    };
    let (start_line, start_col) = position(range.start);
    let (end_line, end_col) = position(range.end);
    SourceSpan {
        file: Sym::new(file),
        start_line,
        start_col,
        end_line,
        end_col,
    }
}

struct Scan<'t> {
    text: &'t [u8],
    at: usize,
    entries: Vec<Located>,
}

impl Scan<'_> {
    fn skip_space(&mut self) {
        while self.text.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> Option<()> {
        (self.text.get(self.at) == Some(&byte)).then(|| self.at += 1)
    }

    /// A value at `self.at`, its members recorded under `path`. `None`: not well formed.
    fn value(&mut self, path: &mut Vec<String>, depth: usize) -> Option<()> {
        self.skip_space();
        match self.text.get(self.at)? {
            b'{' if depth < 128 => self.object(path, depth),
            b'[' if depth < 128 => self.array(path, depth),
            b'"' => self.string().map(|_| ()),
            b'{' | b'[' => None,
            _ => {
                let start = self.at;
                while self
                    .text
                    .get(self.at)
                    .is_some_and(|b| !b.is_ascii_whitespace() && !b",]}:".contains(b))
                {
                    self.at += 1;
                }
                (self.at > start).then_some(())
            }
        }
    }

    fn object(&mut self, path: &mut Vec<String>, depth: usize) -> Option<()> {
        self.eat(b'{')?;
        let mut members: Vec<(String, usize)> = Vec::new();
        self.skip_space();
        if self.eat(b'}').is_some() {
            return Some(());
        }
        loop {
            self.skip_space();
            // A comma before the closing brace is not JSON, but it is what an editor holds while a member is
            // being typed: the object still ends there.
            if !members.is_empty() && self.eat(b'}').is_some() {
                return Some(());
            }
            let key_start = self.at;
            let key = self.string()?;
            let key_range = key_start..self.at;
            self.skip_space();
            self.eat(b':')?;
            // A value being typed starts at the colon, so a cursor in the blank after it is inside the value.
            let colon_end = self.at;
            self.skip_space();
            let entry = self.entries.len();
            path.push(key.clone());
            self.entries.push(Located {
                path: path.clone(),
                key: Some(key_range),
                value: self.at..self.at,
                superseded: false,
            });
            let ended = self.value(path, depth + 1);
            path.pop();
            if ended.is_some() {
                self.entries[entry].value.end = self.at;
            } else {
                // Half-typed: the value covers what was read of it, from the colon.
                self.entries[entry].value = colon_end..self.at.max(colon_end);
            }
            if let Some(earlier) = members.iter().rfind(|(k, _)| *k == key).map(|(_, i)| *i) {
                self.supersede(earlier, entry);
            }
            members.push((key, entry));
            ended?;
            self.skip_space();
            if self.eat(b',').is_some() {
                continue;
            }
            return self.eat(b'}');
        }
    }

    /// Mark the member at `earlier` and everything under it, up to `later`, superseded.
    fn supersede(&mut self, earlier: usize, later: usize) {
        let end = self.entries[earlier].value.end;
        for entry in &mut self.entries[earlier..later] {
            if entry.value.start <= end && (entry.value.end <= end) {
                entry.superseded = true;
            }
        }
    }

    fn array(&mut self, path: &mut Vec<String>, depth: usize) -> Option<()> {
        self.eat(b'[')?;
        self.skip_space();
        if self.eat(b']').is_some() {
            return Some(());
        }
        let mut index = 0usize;
        loop {
            self.skip_space();
            if index > 0 && self.eat(b']').is_some() {
                return Some(());
            }
            let entry = self.entries.len();
            path.push(index.to_string());
            self.entries.push(Located {
                path: path.clone(),
                key: None,
                value: self.at..self.at,
                superseded: false,
            });
            let ended = self.value(path, depth + 1);
            path.pop();
            self.entries[entry].value.end = self.at.max(self.entries[entry].value.start);
            ended?;
            index += 1;
            self.skip_space();
            if self.eat(b',').is_some() {
                continue;
            }
            return self.eat(b']');
        }
    }

    /// A string at `self.at`, decoded as `serde_json` decodes it.
    fn string(&mut self) -> Option<String> {
        self.eat(b'"')?;
        let mut out = Vec::new();
        loop {
            let byte = *self.text.get(self.at)?;
            self.at += 1;
            match byte {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let escape = *self.text.get(self.at)?;
                    self.at += 1;
                    let plain = match escape {
                        b'"' | b'\\' | b'/' => escape,
                        b'b' => 8,
                        b'f' => 12,
                        b'n' => b'\n',
                        b'r' => b'\r',
                        b't' => b'\t',
                        b'u' => {
                            let c = self.unicode()?;
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                            continue;
                        }
                        _ => return None,
                    };
                    out.push(plain);
                }
                _ => out.push(byte),
            }
        }
    }

    /// The `XXXX` of a `\u` escape (and the low surrogate that follows a high one).
    fn unicode(&mut self) -> Option<char> {
        let hex = |scan: &mut Self| -> Option<u32> {
            let digits = scan.text.get(scan.at..scan.at + 4)?;
            let value = u32::from_str_radix(std::str::from_utf8(digits).ok()?, 16).ok()?;
            scan.at += 4;
            Some(value)
        };
        let high = hex(self)?;
        if (0xD800..0xDC00).contains(&high) {
            self.eat(b'\\')?;
            self.eat(b'u')?;
            let low = hex(self)?;
            if !(0xDC00..0xE000).contains(&low) {
                return None;
            }
            return char::from_u32(0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00));
        }
        char::from_u32(high)
    }
}
