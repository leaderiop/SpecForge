//! A file's frontmatter (ADR 0391 D9, 13-RESEARCH Pitfall 3): split off by hand (a first line `---` up to the next line
//! that is exactly `---`), since CommonMark reads it as a thematic break and a setext heading, and read as the small
//! YAML subset GSD writes. No YAML crate (ADR 0391 Rejected): a bounded, line-driven reader that refuses what it does
//! not read, at its line.
//!
//! Read: plain, single- and double-quoted scalars; flow lists and flow maps of scalars; block lists, nested maps and
//! lists of maps; `|` and `>` block scalars with `-` / `+` chomping; `#` comments; empty values. A scalar is never
//! typed: `01` stays the text `01`, `true` the text `true`, for the reader to interpret.
//!
//! Refused, mapping nothing of the frontmatter: an anchor, an alias, a tag, a merge key, a second document, a tab in an
//! indentation, nesting deeper than [`MAX_DEPTH`], a line longer than [`MAX_LINE`], a key given twice.
//!
//! Lines are 1-based over the whole file, frontmatter lines included.

use std::collections::BTreeSet;

/// The deepest nesting read: the frontmatter's own map is depth 1.
pub const MAX_DEPTH: usize = 8;

/// The longest frontmatter line read, in bytes.
pub const MAX_LINE: usize = 64 * 1024;

/// A value and the 1-based line it starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located<T> {
    pub line: usize,
    pub value: T,
}

/// A frontmatter value: text, a list or a map (keys in the order written).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Str(String),
    List(Vec<Located<Value>>),
    Map(Vec<(String, Located<Value>)>),
}

impl Value {
    /// The value of `key`, when this is a map holding it.
    pub fn get(&self, key: &str) -> Option<&Located<Value>> {
        match self {
            Value::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The text, when this is a scalar.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(text) => Some(text),
            _ => None,
        }
    }
}

/// What a reader refused, and the 1-based line it refused at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub line: usize,
    pub reason: String,
}

fn refused<T>(line: usize, reason: impl Into<String>) -> Result<T, Refused> {
    Err(Refused {
        line,
        reason: reason.into(),
    })
}

/// A frontmatter: its lines, without the two `---` marks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Front<'a> {
    pub lines: Vec<&'a str>,
    /// The 1-based line of `lines[0]` in the file.
    pub line: usize,
}

/// The frontmatter of `text`, if its first line is exactly `---`, and the 0-based index of the body's first line. With
/// no closing `---` the frontmatter is refused at line 1 and the whole file is the body.
pub fn split(text: &str) -> (Result<Option<Front<'_>>, Refused>, usize) {
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    if lines.first() != Some(&"---") {
        return (Ok(None), 0);
    }
    match lines.iter().skip(1).position(|l| *l == "---") {
        Some(n) => (
            Ok(Some(Front {
                lines: lines[1..=n].to_vec(),
                line: 2,
            })),
            n + 2,
        ),
        None => (
            refused(1, "frontmatter not closed: no line `---` ends it"),
            0,
        ),
    }
}

/// `text` with its first `start` lines emptied: the body, at its own line numbers, for the Markdown readers.
pub fn body(text: &str, start: usize) -> String {
    text.split('\n')
        .enumerate()
        .map(|(n, line)| if n < start { "" } else { line })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The frontmatter's map, or the first thing it refuses.
pub fn read(front: &Front<'_>) -> Result<Located<Value>, Refused> {
    let mut lines = Vec::with_capacity(front.lines.len());
    for (n, raw) in front.lines.iter().enumerate() {
        let no = front.line + n;
        if raw.len() > MAX_LINE {
            return refused(no, format!("a line longer than {MAX_LINE} bytes"));
        }
        let content = raw.trim_start_matches(' ');
        if content.starts_with('\t') {
            return refused(no, "a tab indents this line");
        }
        let content = content.trim_end();
        if content == "..." || content.starts_with("--- ") || content.starts_with('%') {
            return refused(no, "a second YAML document or a directive");
        }
        lines.push(Line {
            no,
            indent: raw.len() - raw.trim_start_matches(' ').len(),
            content,
        });
    }
    let mut parser = Parser { lines, at: 0 };
    parser.skip();
    let Some(first) = parser.peek() else {
        return Ok(Located {
            line: front.line,
            value: Value::Map(Vec::new()),
        });
    };
    if first.content.starts_with('-')
        && (first.content.len() == 1 || first.content[1..].starts_with(' '))
    {
        return refused(first.no, "a frontmatter is a map, not a list");
    }
    let map = parser.map(first.indent, 1)?;
    match parser.peek() {
        None => Ok(map),
        Some(rest) => refused(rest.no, "a line outside the frontmatter's map"),
    }
}

#[derive(Debug, Clone, Copy)]
struct Line<'a> {
    no: usize,
    indent: usize,
    /// The line without its indentation and trailing white space.
    content: &'a str,
}

struct Parser<'a> {
    lines: Vec<Line<'a>>,
    at: usize,
}

fn item(content: &str) -> Option<&str> {
    match content.strip_prefix('-') {
        Some("") => Some(""),
        Some(rest) if rest.starts_with(' ') => Some(rest.trim_start()),
        _ => None,
    }
}

impl<'a> Parser<'a> {
    /// Moves past blank and comment lines.
    fn skip(&mut self) {
        while self
            .lines
            .get(self.at)
            .is_some_and(|l| l.content.is_empty() || l.content.starts_with('#'))
        {
            self.at += 1;
        }
    }

    fn peek(&mut self) -> Option<Line<'a>> {
        self.skip();
        self.lines.get(self.at).copied()
    }

    fn deep(line: usize, depth: usize) -> Result<(), Refused> {
        if depth > MAX_DEPTH {
            return refused(line, format!("nesting deeper than {MAX_DEPTH}"));
        }
        Ok(())
    }

    /// The map whose keys stand at `indent`.
    fn map(&mut self, indent: usize, depth: usize) -> Result<Located<Value>, Refused> {
        let start = self.peek().map_or(0, |l| l.no);
        Self::deep(start, depth)?;
        let mut entries: Vec<(String, Located<Value>)> = Vec::new();
        // The keys seen, so a key given twice is found in linear time over a frontmatter of any size.
        let mut seen: BTreeSet<String> = BTreeSet::new();
        while let Some(line) = self.peek() {
            if line.indent < indent || (line.indent == indent && item(line.content).is_some()) {
                break;
            }
            if line.indent > indent {
                return refused(line.no, "an indentation no key opens");
            }
            let Some((key, rest)) = key(line.content) else {
                return refused(line.no, "not a `key: value` line");
            };
            if key == "<<" {
                return refused(line.no, "a merge key `<<`");
            }
            if !seen.insert(key.clone()) {
                return refused(line.no, format!("the key `{key}` is given twice"));
            }
            self.at += 1;
            let value = self.value(rest, line.no, indent, depth, true)?;
            entries.push((key, value));
        }
        Ok(Located {
            line: start,
            value: Value::Map(entries),
        })
    }

    /// The list whose `- ` marks stand at `indent`.
    fn list(&mut self, indent: usize, depth: usize) -> Result<Located<Value>, Refused> {
        let start = self.peek().map_or(0, |l| l.no);
        Self::deep(start, depth)?;
        let mut items = Vec::new();
        while let Some(line) = self.peek() {
            if line.indent < indent {
                break;
            }
            if line.indent > indent {
                return refused(line.no, "an indentation no list item opens");
            }
            let Some(rest) = item(line.content) else {
                break;
            };
            if !rest.is_empty() && !rest.starts_with(['"', '\'', '[', '{']) && key(rest).is_some() {
                // `- key: value`: a map whose keys stand at the column of `key`.
                let column = indent + line.content.len() - rest.len();
                self.lines[self.at] = Line {
                    no: line.no,
                    indent: column,
                    content: rest,
                };
                items.push(self.map(column, depth + 1)?);
            } else {
                self.at += 1;
                items.push(self.value(rest, line.no, indent, depth, false)?);
            }
        }
        Ok(Located {
            line: start,
            value: Value::List(items),
        })
    }

    /// The value written `rest` after a key or a list mark at `indent`, on line `no`, inside a collection of `depth`.
    fn value(
        &mut self,
        rest: &'a str,
        no: usize,
        indent: usize,
        depth: usize,
        in_map: bool,
    ) -> Result<Located<Value>, Refused> {
        if rest.is_empty() || rest.starts_with('#') {
            return match self.peek() {
                Some(next) if next.indent > indent => {
                    if item(next.content).is_some() {
                        self.list(next.indent, depth + 1)
                    } else {
                        self.map(next.indent, depth + 1)
                    }
                }
                // `key:` then `- a` at the key's own column: YAML's compact list.
                Some(next) if in_map && next.indent == indent && item(next.content).is_some() => {
                    self.list(indent, depth + 1)
                }
                _ => Ok(Located {
                    line: no,
                    value: Value::Str(String::new()),
                }),
            };
        }
        if let Some(header) = rest.strip_prefix('|') {
            return self.block(header, no, indent, false);
        }
        if let Some(header) = rest.strip_prefix('>') {
            return self.block(header, no, indent, true);
        }
        Ok(Located {
            line: no,
            value: inline(rest, no, depth)?,
        })
    }

    /// A `|` or `>` block scalar: the lines indented past `indent` after its header.
    fn block(
        &mut self,
        header: &str,
        no: usize,
        indent: usize,
        folded: bool,
    ) -> Result<Located<Value>, Refused> {
        let header = header.trim();
        let (chomp, rest) = match header.chars().next() {
            Some(c @ ('-' | '+')) => (c, header[1..].trim_start()),
            _ => (' ', header),
        };
        if !(rest.is_empty() || rest.starts_with('#')) {
            return refused(no, "a block scalar header this reader does not read");
        }
        let mut lines: Vec<String> = Vec::new();
        let mut column: Option<usize> = None;
        while let Some(line) = self.lines.get(self.at) {
            if !line.content.is_empty() && line.indent <= indent {
                break;
            }
            if line.content.is_empty() {
                lines.push(String::new());
            } else {
                let c = *column.get_or_insert(line.indent);
                if line.indent < c {
                    return refused(line.no, "a block scalar line less indented than its first");
                }
                lines.push(format!("{}{}", " ".repeat(line.indent - c), line.content));
            }
            self.at += 1;
        }
        let kept = lines
            .iter()
            .rposition(|l| !l.is_empty())
            .map_or(0, |i| i + 1);
        let trailing = lines.len() - kept;
        lines.truncate(kept);
        let mut text = if folded {
            fold(&lines)
        } else {
            lines.join("\n")
        };
        match chomp {
            '-' => {}
            '+' => text.push_str(&"\n".repeat(trailing + 1)),
            _ if !text.is_empty() => text.push('\n'),
            _ => {}
        }
        Ok(Located {
            line: no,
            value: Value::Str(text),
        })
    }
}

/// `>`'s folding: the lines of a paragraph joined by a space, each blank line a line break.
fn fold(lines: &[String]) -> String {
    let mut out = String::new();
    let mut blank = 0;
    for (n, line) in lines.iter().enumerate() {
        if line.is_empty() {
            blank += 1;
            continue;
        }
        if n > 0 {
            out.push_str(&if blank == 0 {
                " ".to_string()
            } else {
                "\n".repeat(blank)
            });
        }
        blank = 0;
        out.push_str(line);
    }
    out
}

/// The key of a `key: value` line, unquoted, and the text after it.
fn key(content: &str) -> Option<(String, &str)> {
    let (key, after) = match content.chars().next()? {
        q @ ('"' | '\'') => {
            let end = content[1..].find(q)? + 1;
            (content[1..end].to_string(), &content[end + 1..])
        }
        '[' | '{' | '#' | '&' | '*' | '!' | '?' => return None,
        _ => {
            let end = content
                .find(": ")
                .or_else(|| content.strip_suffix(':').map(str::len))?;
            (content[..end].trim_end().to_string(), &content[end..])
        }
    };
    let rest = after.strip_prefix(':')?;
    if !(rest.is_empty() || rest.starts_with(' ')) || key.is_empty() {
        return None;
    }
    Some((key, rest.trim()))
}

/// A value written on its key's line: a scalar, a flow list or a flow map of scalars.
fn inline(text: &str, no: usize, depth: usize) -> Result<Value, Refused> {
    let text = text.trim();
    match text.chars().next() {
        Some('[') | Some('{') => {
            Parser::deep(no, depth + 1)?;
            let (open, close) = if text.starts_with('[') {
                ('[', ']')
            } else {
                ('{', '}')
            };
            let inner = uncomment(text)
                .strip_prefix(open)
                .and_then(|t| t.strip_suffix(close))
                .ok_or_else(|| Refused {
                    line: no,
                    reason: format!("a flow collection not closed by `{close}` on its line"),
                })?;
            let parts = commas(inner, no)?.into_iter().filter(|p| !p.is_empty());
            if open == '[' {
                return Ok(Value::List(
                    parts.map(|p| scalar(p, no)).collect::<Result<_, _>>()?,
                ));
            }
            let mut entries = Vec::new();
            for part in parts {
                let (k, v) = key(part).ok_or_else(|| Refused {
                    line: no,
                    reason: "a flow map item is not `key: value`".to_string(),
                })?;
                entries.push((k, scalar(v, no)?));
            }
            Ok(Value::Map(entries))
        }
        _ => Ok(scalar(text, no)?.value),
    }
}

/// `text` split at the commas outside quotes; a flow collection inside one is refused.
fn commas(text: &str, no: usize) -> Result<Vec<&str>, Refused> {
    let mut parts = Vec::new();
    let (mut start, mut quote) = (0, None::<char>);
    for (i, c) in text.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '[' | '{') => return refused(no, "a flow collection inside a flow collection"),
            (None, ',') => {
                parts.push(text[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    Ok(parts)
}

/// `text` without a trailing ` #` comment outside quotes.
fn uncomment(text: &str) -> &str {
    let mut quote = None::<char>;
    let mut prev = ' ';
    for (i, c) in text.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') if prev == ' ' || prev == '[' || prev == '{' || prev == ',' => {
                quote = Some(c);
            }
            (None, '#') if prev == ' ' || prev == '\t' => return text[..i].trim_end(),
            _ => {}
        }
        prev = c;
    }
    text.trim_end()
}

/// One scalar, unquoted.
fn scalar(text: &str, no: usize) -> Result<Located<Value>, Refused> {
    let text = text.trim();
    let value = match text.chars().next() {
        Some('&') => return refused(no, "an anchor `&`"),
        Some('*') => return refused(no, "an alias `*`"),
        Some('!') => return refused(no, "a tag `!`"),
        Some('[' | '{') => return refused(no, "a flow collection inside a flow collection"),
        Some(q @ ('"' | '\'')) => quoted(text, q, no)?,
        _ => uncomment(text).to_string(),
    };
    Ok(Located {
        line: no,
        value: Value::Str(value),
    })
}

/// A quoted scalar's text; it ends on its line, then only a comment may follow.
fn quoted(text: &str, q: char, no: usize) -> Result<String, Refused> {
    let mut out = String::new();
    let mut chars = text[1..].char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' if q == '"' => match chars.next() {
                Some((_, 'n')) => out.push('\n'),
                Some((_, 't')) => out.push('\t'),
                Some((_, e)) => out.push(e),
                None => break,
            },
            c if c == q => {
                if q == '\'' && text[1 + i + 1..].starts_with('\'') {
                    out.push('\'');
                    chars.next();
                    continue;
                }
                let after = text[1 + i + 1..].trim();
                if !(after.is_empty() || after.starts_with('#')) {
                    return refused(no, "text after a quoted value");
                }
                return Ok(out);
            }
            c => out.push(c),
        }
    }
    refused(no, "a quoted value runs past its line")
}

#[cfg(test)]
mod tests;
