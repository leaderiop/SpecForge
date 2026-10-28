//! Merging entries into a JSON config file without rewriting what is not ours (ADR 0292). The file is edited as text:
//! an entry is inserted after its last sibling and removed with the separator beside it, so a file with a user's own
//! keys, layout and order comes back byte for byte when install's entries are taken out again. A document is not
//! parsed into a value and written back, which would reflow it.
//!
//! An entry is ours when it is an object whose `command` is `specforge` or starts `specforge hook `, or which holds
//! a `hooks` array with such an object (a matcher group). Nothing else is ever removed or replaced.

use serde_json::Value;
use specforge_ops::host::{Merged, MergedEntry, NotRendered};

/// How deep an entry of ours may stand: `hooks.PreToolUse[]` is the deepest.
const DEPTH: usize = 3;

/// A parsed span of the document.
#[derive(Debug, Clone)]
struct Node {
    start: usize,
    end: usize,
    kind: Kind,
}

#[derive(Debug, Clone)]
enum Kind {
    Object(Vec<Member>),
    Array(Vec<Node>),
    Scalar,
}

#[derive(Debug, Clone)]
struct Member {
    key: String,
    /// Where the key's quote starts.
    key_start: usize,
    value: Node,
}

struct Parser<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.at)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.at += 1;
        }
    }

    fn fail<T>(&self, why: &str) -> Result<T, String> {
        Err(format!("{why} at byte {}", self.at))
    }

    fn string(&mut self) -> Result<String, String> {
        let start = self.at;
        self.at += 1;
        let bytes = self.text.as_bytes();
        while let Some(&b) = bytes.get(self.at) {
            match b {
                b'\\' => self.at += 2,
                b'"' => {
                    self.at += 1;
                    return serde_json::from_str(&self.text[start..self.at])
                        .map_err(|e| format!("a string is not valid: {e}"));
                }
                _ => self.at += 1,
            }
        }
        self.fail("a string is not closed")
    }

    fn value(&mut self, depth: usize) -> Result<Node, String> {
        if depth > 64 {
            return self.fail("the document is nested too deep");
        }
        self.skip();
        let start = self.at;
        let Some(&b) = self.text.as_bytes().get(self.at) else {
            return self.fail("a value is missing");
        };
        let kind = match b {
            b'{' => {
                self.at += 1;
                let mut members = Vec::new();
                loop {
                    self.skip();
                    match self.text.as_bytes().get(self.at) {
                        Some(b'}') => {
                            self.at += 1;
                            break;
                        }
                        Some(b'"') => {
                            let key_start = self.at;
                            let key = self.string()?;
                            self.skip();
                            if self.text.as_bytes().get(self.at) != Some(&b':') {
                                return self.fail("a ':' is missing");
                            }
                            self.at += 1;
                            let value = self.value(depth + 1)?;
                            members.push(Member {
                                key,
                                key_start,
                                value,
                            });
                            self.skip();
                            match self.text.as_bytes().get(self.at) {
                                Some(b',') => self.at += 1,
                                Some(b'}') => {}
                                _ => return self.fail("a ',' or '}' is missing"),
                            }
                        }
                        _ => return self.fail("a key or '}' is missing"),
                    }
                }
                Kind::Object(members)
            }
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                loop {
                    self.skip();
                    if self.text.as_bytes().get(self.at) == Some(&b']') {
                        self.at += 1;
                        break;
                    }
                    items.push(self.value(depth + 1)?);
                    self.skip();
                    match self.text.as_bytes().get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b']') => {}
                        _ => return self.fail("a ',' or ']' is missing"),
                    }
                }
                Kind::Array(items)
            }
            b'"' => {
                self.string()?;
                Kind::Scalar
            }
            _ => {
                while self
                    .text
                    .as_bytes()
                    .get(self.at)
                    .is_some_and(|b| !b.is_ascii_whitespace() && !b",]}".contains(b))
                {
                    self.at += 1;
                }
                if self.at == start {
                    return self.fail("a value is missing");
                }
                Kind::Scalar
            }
        };
        Ok(Node {
            start,
            end: self.at,
            kind,
        })
    }
}

fn parse(text: &str) -> Result<Node, String> {
    let mut parser = Parser { text, at: 0 };
    let node = parser.value(0)?;
    parser.skip();
    if parser.at != text.len() {
        return parser.fail("text follows the document");
    }
    match node.kind {
        Kind::Object(_) => Ok(node),
        _ => Err("the document is not an object".to_string()),
    }
}

fn value_of(text: &str, node: &Node) -> Value {
    serde_json::from_str(&text[node.start..node.end]).unwrap_or(Value::Null)
}

/// Whether `value` is an entry install owns.
pub fn is_ours(value: &Value) -> bool {
    let command = value.get("command").and_then(Value::as_str);
    if command.is_some_and(|c| c == "specforge" || c.starts_with("specforge hook ")) {
        return true;
    }
    value
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| hooks.iter().any(is_ours))
}

/// A piece of text to cut out.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Cut {
    start: usize,
    end: usize,
}

/// The cuts that take the entries of ours out of `node`, and whether that leaves `node` with nothing (it then goes
/// whole with its parent's cut, not by its own).
fn strip(text: &str, node: &Node, depth: usize) -> (Vec<Cut>, bool) {
    let children: Vec<(&Node, usize)> = match &node.kind {
        Kind::Object(members) => members.iter().map(|m| (&m.value, m.key_start)).collect(),
        Kind::Array(items) => items.iter().map(|n| (n, n.start)).collect(),
        Kind::Scalar => return (Vec::new(), false),
    };
    let mut cuts = Vec::new();
    let mut removed = vec![false; children.len()];
    for (n, (child, _)) in children.iter().enumerate() {
        if is_ours(&value_of(text, child)) {
            removed[n] = true;
        } else if depth < DEPTH {
            let (inner, emptied) = strip(text, child, depth + 1);
            if emptied {
                removed[n] = true;
            } else {
                cuts.extend(inner);
            }
        }
    }
    if !children.is_empty() && removed.iter().all(|r| *r) {
        return (
            vec![Cut {
                start: node.start + 1,
                end: node.end - 1,
            }],
            true,
        );
    }
    let mut n = 0;
    while n < children.len() {
        if !removed[n] {
            n += 1;
            continue;
        }
        let mut last = n;
        while last + 1 < children.len() && removed[last + 1] {
            last += 1;
        }
        // The run takes the separator before it; at the front, the one after it.
        cuts.push(if n > 0 {
            Cut {
                start: children[n - 1].0.end,
                end: children[last].0.end,
            }
        } else {
            Cut {
                start: children[n].1,
                end: children[last + 1].1,
            }
        });
        n = last + 1;
    }
    (cuts, false)
}

/// Where the entries of ours stand: the keys down to their container, the member name (none: an array element) and the
/// value.
fn found(text: &str, node: &Node, at: &mut Vec<String>, depth: usize, out: &mut Vec<Found>) {
    let mut visit = |name: Option<&str>, child: &Node, at: &mut Vec<String>| {
        let value = value_of(text, child);
        if is_ours(&value) {
            out.push((at.clone(), name.map(str::to_string), value));
        } else if depth < DEPTH {
            if let Some(name) = name {
                at.push(name.to_string());
                found(text, child, at, depth + 1, out);
                at.pop();
            } else {
                found(text, child, at, depth + 1, out);
            }
        }
    };
    match &node.kind {
        Kind::Object(members) => {
            for m in members {
                visit(Some(&m.key), &m.value, at);
            }
        }
        Kind::Array(items) => {
            for item in items {
                visit(None, item, at);
            }
        }
        Kind::Scalar => {}
    }
}

type Found = (Vec<String>, Option<String>, Value);

fn apply(text: &str, mut cuts: Vec<Cut>) -> String {
    cuts.sort_by_key(|c| std::cmp::Reverse(c.start));
    let mut out = text.to_string();
    for cut in cuts {
        out.replace_range(cut.start..cut.end, "");
    }
    out
}

/// The indentation of the line `at` stands on, if `at` is the first thing on it.
fn line_indent(text: &str, at: usize) -> Option<&str> {
    let line_start = text[..at].rfind('\n').map_or(0, |n| n + 1);
    let indent = &text[line_start..at];
    (line_start > 0 || at == 0)
        .then_some(indent)
        .filter(|i| i.chars().all(|c| c == ' ' || c == '\t'))
}

/// The indentation unit of a document: its first indented line's, else two spaces.
fn unit_of(text: &str) -> String {
    text.lines()
        .find_map(|l| {
            let indent: String = l.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
            (!indent.is_empty() && l.trim() != "").then_some(indent)
        })
        .unwrap_or_else(|| "  ".to_string())
}

/// `value` written with `unit` indentation, its continuation lines starting at `base`.
fn pretty(value: &Value, unit: &str, base: &str) -> String {
    let mut out = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(unit.as_bytes());
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, formatter);
    serde::Serialize::serialize(value, &mut serializer).expect("a value serializes");
    String::from_utf8(out)
        .expect("JSON is UTF-8")
        .replace('\n', &format!("\n{base}"))
}

/// The container at `keys` below `node`, and the keys that are missing below it (the first of them is not a member
/// of the container).
fn descend<'a>(node: &'a Node, text: &str, keys: &'a [String]) -> (&'a Node, &'a [String]) {
    let mut at = node;
    for (n, key) in keys.iter().enumerate() {
        let Kind::Object(members) = &at.kind else {
            return (at, &keys[n..]);
        };
        match members.iter().find(|m| m.key == *key) {
            Some(m) if matches!(m.value.kind, Kind::Object(_) | Kind::Array(_)) => at = &m.value,
            _ => return (at, &keys[n..]),
        }
    }
    let _ = text;
    (at, &[])
}

/// `item` (a member's text or an element's) added to the container `at`, at nesting `depth`.
fn add(text: &str, at: &Node, depth: usize, item: &dyn Fn(&str, &str) -> String) -> String {
    let (unit, children): (String, Vec<usize>) = match &at.kind {
        Kind::Object(members) => (unit_of(text), members.iter().map(|m| m.value.end).collect()),
        Kind::Array(items) => (unit_of(text), items.iter().map(|n| n.end).collect()),
        Kind::Scalar => return text.to_string(),
    };
    let multiline = text[at.start..at.end].contains('\n')
        || (children.is_empty() && text.contains('\n') && at.start == 0);
    let mut out = text.to_string();
    if let Some(last) = children.last() {
        if multiline {
            let first_child = match &at.kind {
                Kind::Object(m) => m[0].key_start,
                Kind::Array(i) => i[0].start,
                Kind::Scalar => 0,
            };
            let indent = line_indent(text, first_child)
                .map_or_else(|| unit.repeat(depth + 1), str::to_string);
            let closing = unit.repeat(depth);
            let _ = closing;
            out.insert_str(*last, &format!(",\n{indent}{}", item(&unit, &indent)));
        } else {
            out.insert_str(*last, &format!(", {}", item(&unit, "")));
        }
    } else if multiline || (text.contains('\n') && depth == 0) {
        let indent = unit.repeat(depth + 1);
        let closing = unit.repeat(depth);
        out.replace_range(
            at.start + 1..at.end - 1,
            &format!("\n{indent}{}\n{closing}", item(&unit, &indent)),
        );
    } else {
        out.replace_range(at.start + 1..at.end - 1, &item(&unit, ""));
    }
    out
}

/// Insert `entry` into the document `text`: the container is created if it is missing.
fn insert(text: &str, entry: &MergedEntry, wrap_multiline: bool) -> Result<String, String> {
    let root = parse(text)?;
    let (container, missing) = descend(&root, text, &entry.keys);
    let depth = entry.keys.len() - missing.len();
    let mut value = entry.value.clone();
    // The missing levels, innermost first, wrapped around the entry.
    if let Some(name) = &entry.name {
        if let (true, Kind::Object(members)) = (missing.is_empty(), &container.kind)
            && members.iter().any(|m| m.key == *name)
        {
            return Ok(text.to_string());
        }
        let mut wrapped = serde_json::Map::new();
        wrapped.insert(name.clone(), value);
        value = Value::Object(wrapped);
    } else if missing.is_empty() {
        let line = |unit: &str, base: &str| {
            if wrap_multiline && text.contains('\n') {
                pretty(&entry.value, unit, base)
            } else {
                entry.value.to_string()
            }
        };
        return Ok(add(text, container, depth, &line));
    } else {
        value = Value::Array(vec![value]);
    }
    // `value` is now what stands at `missing[0]`'s level: the entry's member, or the array holding the entry.
    let mut nested = value;
    let mut path = missing.iter().skip(1).rev();
    let leaf_is_member = entry.name.is_some() && missing.is_empty();
    if leaf_is_member {
        // The container exists and holds the member itself.
        let Value::Object(map) = nested else {
            return Err("an entry is not a member".to_string());
        };
        let (name, member) = map.into_iter().next().ok_or("an entry has no name")?;
        let line = move |unit: &str, base: &str| {
            let text = if wrap_multiline {
                pretty(&member, unit, base)
            } else {
                member.to_string()
            };
            format!("{}: {text}", Value::String(name.clone()))
        };
        return Ok(add(text, container, depth, &line));
    }
    for key in path.by_ref() {
        let mut map = serde_json::Map::new();
        map.insert(key.clone(), nested);
        nested = Value::Object(map);
    }
    let first = missing.first().ok_or("an entry has no place")?.clone();
    let line = move |unit: &str, base: &str| {
        let text = if wrap_multiline {
            pretty(&nested, unit, base)
        } else {
            nested.to_string()
        };
        format!("{}: {text}", Value::String(first.clone()))
    };
    Ok(add(text, container, depth, &line))
}

/// `existing` with the entries of ours replaced by `entries`, everything else untouched; see the module docs.
/// `scaffold` names the root keys that are not content: a file holding only them is as good as empty.
pub fn merge(
    existing: Option<&str>,
    entries: &[MergedEntry],
    scaffold: &[&str],
) -> Result<Merged, String> {
    let blank = existing.is_none_or(|t| t.trim().is_empty());
    let mut text = if blank {
        "{}\n".to_string()
    } else {
        existing.unwrap_or_default().to_string()
    };
    let root = parse(&text)?;
    let mut not_rendered = Vec::new();
    // A named entry whose name a user's own member holds is not rendered.
    let mut wanted: Vec<&MergedEntry> = Vec::new();
    for entry in entries {
        let (container, missing) = descend(&root, &text, &entry.keys);
        let occupied = entry
            .name
            .as_ref()
            .filter(|_| missing.is_empty())
            .and_then(|name| {
                let Kind::Object(members) = &container.kind else {
                    return None;
                };
                members.iter().find(|m| m.key == *name)
            });
        match occupied {
            Some(member) if is_ours(&value_of(&text, &member.value)) => wanted.push(entry),
            Some(_) if entry.name.as_deref().is_some_and(|n| scaffold.contains(&n)) => {}
            Some(member) => {
                let _ = member;
                not_rendered.push(NotRendered {
                    what: format!(
                        "{}.{}",
                        entry.keys.join("."),
                        entry.name.as_deref().unwrap_or_default()
                    )
                    .trim_start_matches('.')
                    .to_string(),
                    why: "the file already sets it".to_string(),
                });
            }
            None => wanted.push(entry),
        }
    }
    // Nothing to change: the entries of ours are exactly the wanted ones.
    let mut here: Vec<Found> = Vec::new();
    found(&text, &root, &mut Vec::new(), 0, &mut here);
    let key = |keys: &[String], name: &Option<String>, value: &Value| {
        format!("{keys:?} {name:?} {value}")
    };
    let mut have: Vec<String> = here.iter().map(|(k, n, v)| key(k, n, v)).collect();
    let mut want: Vec<String> = wanted
        .iter()
        .filter(|e| !e.name.as_deref().is_some_and(|n| scaffold.contains(&n)))
        .map(|e| key(&e.keys, &e.name, &e.value))
        .collect();
    have.sort();
    want.sort();
    let scaffold_present = wanted
        .iter()
        .filter(|e| e.name.as_deref().is_some_and(|n| scaffold.contains(&n)))
        .all(|e| {
            let (c, missing) = descend(&root, &text, &e.keys);
            missing.is_empty()
                && matches!(&c.kind, Kind::Object(m) if m.iter().any(|m| Some(&m.key) == e.name.as_ref()))
        });
    if have == want && scaffold_present && !blank {
        return Ok(Merged {
            text: Some(text),
            not_rendered,
        });
    }
    let (cuts, _) = strip(&text, &root, 0);
    text = apply(&text, cuts);
    let multiline = blank || text.trim().contains('\n');
    for entry in &wanted {
        text = insert(&text, entry, multiline)?;
    }
    let root = parse(&text)?;
    let Kind::Object(members) = &root.kind else {
        return Err("the document is not an object".to_string());
    };
    if members.iter().all(|m| scaffold.contains(&m.key.as_str())) && wanted.is_empty() {
        return Ok(Merged {
            text: None,
            not_rendered,
        });
    }
    Ok(Merged {
        text: Some(text),
        not_rendered,
    })
}

#[cfg(test)]
mod tests;
