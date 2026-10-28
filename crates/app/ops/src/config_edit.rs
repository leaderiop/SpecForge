//! Splicing one member of `specforge.json` as text (ADR 0296): replace a member's value, insert a member after its
//! last sibling, create the objects above it, or remove it with its comma, leaving every other byte of the file where
//! the author put it. `serde_json` would re-print the whole file; the project's `specforge.json` is a reviewed file.
//!
//! Every edit is proved before it is returned: the new text must parse and equal, as a JSON value, the old document
//! with exactly that path set or removed. An edit that would not is [`Err`], and the caller writes nothing.

use serde::Serialize;
use serde_json::Value;
use specforge_common::json_span;

/// `text` with the member at `path` (from the root) set to `value`; created, with the objects above it, when it is
/// not written. A `value` equal to the written one leaves the text as it is.
pub(crate) fn set_member(text: &str, path: &[&str], value: &Value) -> Result<String, String> {
    let old = parse(text)?;
    if lookup(&old, path) == Some(value) {
        return Ok(text.to_string());
    }
    let index = json_span::index(text);
    let style = Style::of(text);
    let spliced = if let Some(member) = index.locate(path) {
        let depth = depth_of(text, member.key.as_ref().map_or(0, |k| k.start));
        replace(text, member.value.clone(), &style.render(value, &depth))
    } else {
        // The deepest object that is written: the new member, and the objects it needs, go at its end.
        let at = (0..path.len())
            .rev()
            .find(|&n| {
                index
                    .locate(&path[..n])
                    .is_some_and(|m| text[m.value.clone()].starts_with('{'))
            })
            .ok_or("specforge.json is not a JSON object")?;
        let parent = index
            .locate(&path[..at])
            .ok_or("specforge.json is not a JSON object")?;
        let parent_depth = depth_of(text, parent.key.as_ref().map_or(0, |k| k.start));
        let child_depth = format!("{parent_depth}{}", style.unit);
        let entry = nested(&path[at..], value, &style, &child_depth);
        let siblings = index.children(&path[..at]);
        match siblings.last() {
            Some(last) => {
                let separator = if text[parent.value.clone()].contains('\n') {
                    format!(",{}{child_depth}", style.newline)
                } else {
                    ", ".to_string()
                };
                insert(text, last.value.end, &format!("{separator}{entry}"))
            }
            None => replace(
                text,
                parent.value.clone(),
                &format!(
                    "{{{nl}{child_depth}{entry}{nl}{parent_depth}}}",
                    nl = style.newline
                ),
            ),
        }
    };
    let mut expected = old;
    put(&mut expected, path, Some(value.clone()));
    prove(&spliced, &expected)?;
    Ok(spliced)
}

/// `text` without the member at `path`, its comma gone with it; unchanged when the member is not written.
pub(crate) fn remove_member(text: &str, path: &[&str]) -> Result<String, String> {
    let old = parse(text)?;
    let index = json_span::index(text);
    let Some(member) = index.locate(path) else {
        return Ok(text.to_string());
    };
    let key = member.key.clone().ok_or("a member has a key")?;
    let siblings = index.children(&path[..path.len() - 1]);
    let at = siblings
        .iter()
        .position(|s| s.path == member.path)
        .ok_or("the member is not among its siblings")?;
    let removed = match (at.checked_sub(1).map(|i| siblings[i]), siblings.get(at + 1)) {
        // Up to the next key: its indentation is the removed member's own.
        (_, Some(next)) => {
            let next_key = next.key.clone().ok_or("a member has a key")?;
            key.start..next_key.start
        }
        // The last of several: from the end of the one before.
        (Some(before), None) => before.value.end..member.value.end,
        // The only one: an empty object.
        (None, None) => {
            let parent = index
                .locate(&path[..path.len() - 1])
                .ok_or("the member has a parent")?;
            // shortcut: an emptied parent object stays as `{}` rather than being removed up the path, when a tidy-up is asked for.
            let spliced = replace(text, parent.value.clone(), "{}");
            let mut expected = old;
            put(&mut expected, path, None);
            prove(&spliced, &expected)?;
            return Ok(spliced);
        }
    };
    let spliced = replace(text, removed, "");
    let mut expected = old;
    put(&mut expected, path, None);
    prove(&spliced, &expected)?;
    Ok(spliced)
}

/// The layout of a file: its newline and the unit it indents by.
struct Style {
    newline: &'static str,
    unit: String,
}

impl Style {
    fn of(text: &str) -> Self {
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let unit = text
            .lines()
            .map(|line| &line[..line.len() - line.trim_start().len()])
            .find(|indent| !indent.is_empty())
            .unwrap_or("  ")
            .to_string();
        Style { newline, unit }
    }

    /// `value` as the file would hold it at indentation `depth`: a scalar inline, a list or object pretty.
    fn render(&self, value: &Value, depth: &str) -> String {
        if !matches!(value, Value::Array(_) | Value::Object(_)) {
            return value.to_string();
        }
        let mut out = Vec::new();
        let format = serde_json::ser::PrettyFormatter::with_indent(self.unit.as_bytes());
        let mut serializer = serde_json::Serializer::with_formatter(&mut out, format);
        if value.serialize(&mut serializer).is_err() {
            return value.to_string();
        }
        String::from_utf8_lossy(&out)
            .lines()
            .collect::<Vec<_>>()
            .join(&format!("{}{depth}", self.newline))
    }
}

/// The indentation of the line `offset` is on.
fn depth_of(text: &str, offset: usize) -> String {
    let start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    text[start..offset]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect()
}

/// `"a": { "b": value }` for the path `["a", "b"]`, each object opened at `depth` and deeper.
fn nested(path: &[&str], value: &Value, style: &Style, depth: &str) -> String {
    let key = Value::String(path[0].to_string());
    if path.len() == 1 {
        return format!("{key}: {}", style.render(value, depth));
    }
    let inner = format!("{depth}{}", style.unit);
    format!(
        "{key}: {{{nl}{inner}{}{nl}{depth}}}",
        nested(&path[1..], value, style, &inner),
        nl = style.newline
    )
}

fn replace(text: &str, range: std::ops::Range<usize>, with: &str) -> String {
    format!("{}{with}{}", &text[..range.start], &text[range.end..])
}

fn insert(text: &str, at: usize, with: &str) -> String {
    replace(text, at..at, with)
}

fn parse(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|e| format!("specforge.json is not JSON: {e}"))
}

fn lookup<'a>(document: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(document, |at, key| at.get(*key))
}

/// `document` with `path` set to `value` (its objects created), or removed for `None`.
fn put(document: &mut Value, path: &[&str], value: Option<Value>) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut at = document;
    for key in parents {
        let Some(map) = at.as_object_mut() else {
            return;
        };
        at = map
            .entry((*key).to_string())
            .or_insert_with(|| Value::Object(Default::default()));
    }
    if let Some(map) = at.as_object_mut() {
        match value {
            Some(value) => {
                map.insert((*last).to_string(), value);
            }
            None => {
                map.remove(*last);
            }
        }
    }
}

/// The proof: `spliced` is JSON and is `expected`.
fn prove(spliced: &str, expected: &Value) -> Result<(), String> {
    match serde_json::from_str::<Value>(spliced) {
        Ok(ref value) if value == expected => Ok(()),
        Ok(_) => Err("the edit would change more than the one setting".to_string()),
        Err(e) => Err(format!("the edit would not leave JSON: {e}")),
    }
}

#[cfg(test)]
mod tests;
