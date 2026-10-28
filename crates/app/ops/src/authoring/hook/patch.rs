//! A patch-text tool's input (Codex `apply_patch`, Cursor `ApplyPatch`): one string holding `*** Begin Patch`, and per
//! file an `*** Add File:`, `*** Update File:` (optionally `*** Move to:`) or `*** Delete File:` section. The spec guard
//! judges the files such a patch leaves, as it does for a Write or an Edit (review K9 M3).
//!
//! The tool shape is the one the hosts document; it is read from any string of the input that holds the markers, so a
//! host that names the field differently is still seen.

use std::collections::BTreeMap;

use serde_json::Value;

/// What a patch does to one file.
#[derive(Debug, PartialEq, Eq)]
pub enum Change {
    /// The file with these lines.
    Add(String),
    /// Hunks of (old text, new text), the file moved to `to` if it is.
    Update {
        hunks: Vec<(String, String)>,
        to: Option<String>,
    },
    Delete,
}

/// The patch text of a tool's input: the input itself or the first string field holding the markers.
pub fn text_of(input: &Value) -> Option<&str> {
    let holds = |s: &str| s.contains("*** Begin Patch") || s.lines().any(|l| section(l).is_some());
    match input {
        Value::String(s) if holds(s) => Some(s),
        Value::Object(map) => map.values().filter_map(Value::as_str).find(|s| holds(s)),
        _ => None,
    }
}

/// The header a line is: `(kind, path)`.
fn section(line: &str) -> Option<(&str, &str)> {
    ["Add File", "Update File", "Delete File", "Move to"]
        .into_iter()
        .find_map(|kind| {
            line.strip_prefix("*** ")?
                .strip_prefix(kind)?
                .strip_prefix(':')
                .map(|path| (kind, path.trim()))
        })
}

/// The files `patch` changes, in order.
pub fn parse(patch: &str) -> Vec<(String, Change)> {
    let mut out: Vec<(String, Change)> = Vec::new();
    let mut hunk: Option<(Vec<&str>, Vec<&str>)> = None;
    let flush = |out: &mut Vec<(String, Change)>, hunk: &mut Option<(Vec<&str>, Vec<&str>)>| {
        if let (Some((old, new)), Some((_, Change::Update { hunks, .. }))) =
            (hunk.take(), out.last_mut())
        {
            hunks.push((old.join("\n"), new.join("\n")));
        }
    };
    for line in patch.lines() {
        if let Some((kind, path)) = section(line) {
            flush(&mut out, &mut hunk);
            match kind {
                "Add File" => out.push((path.to_string(), Change::Add(String::new()))),
                "Update File" => out.push((
                    path.to_string(),
                    Change::Update {
                        hunks: Vec::new(),
                        to: None,
                    },
                )),
                "Delete File" => out.push((path.to_string(), Change::Delete)),
                _ => {
                    if let Some((_, Change::Update { to, .. })) = out.last_mut() {
                        *to = Some(path.to_string());
                    }
                }
            }
            continue;
        }
        if line.starts_with("*** ") {
            flush(&mut out, &mut hunk);
            continue;
        }
        match out.last_mut() {
            Some((_, Change::Add(text))) => {
                if let Some(added) = line.strip_prefix('+') {
                    text.push_str(added);
                    text.push('\n');
                }
            }
            Some((_, Change::Update { .. })) => {
                if line.starts_with("@@") {
                    flush(&mut out, &mut hunk);
                    hunk = Some((Vec::new(), Vec::new()));
                    continue;
                }
                let (old, new) = hunk.get_or_insert_with(Default::default);
                match line.split_at_checked(1) {
                    Some(("+", rest)) => new.push(rest),
                    Some(("-", rest)) => old.push(rest),
                    Some((" ", rest)) => {
                        old.push(rest);
                        new.push(rest);
                    }
                    _ if line.is_empty() => {
                        old.push("");
                        new.push("");
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    flush(&mut out, &mut hunk);
    out
}

/// The paths a patch names (the file a move leaves too).
pub fn paths(patch: &str) -> Vec<String> {
    patch
        .lines()
        .filter_map(section)
        .map(|(_, path)| path.to_string())
        .collect()
}

/// The text each file holds after `patch`, given what `read` finds on disk (none: not there). A file whose hunk does
/// not apply makes the whole patch the host's to refuse: none.
pub fn after(
    patch: &str,
    mut read: impl FnMut(&str) -> Option<String>,
    keep: impl Fn(&str) -> bool,
) -> Option<BTreeMap<String, String>> {
    let mut texts = BTreeMap::new();
    for (path, change) in parse(patch) {
        if !keep(&path) {
            continue;
        }
        match change {
            Change::Add(text) => {
                texts.insert(path, text);
            }
            Change::Delete => {
                texts.insert(path, String::new());
            }
            Change::Update { hunks, to } => {
                let mut text = read(&path)?;
                for (old, new) in &hunks {
                    let at = text.find(old.as_str())?;
                    text.replace_range(at..at + old.len(), new);
                }
                match to {
                    Some(to) => {
                        texts.insert(path, String::new());
                        texts.insert(to, text);
                    }
                    None => {
                        texts.insert(path, text);
                    }
                }
            }
        }
    }
    Some(texts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PATCH: &str = "*** Begin Patch\n*** Update File: a.spec\n@@\n keep\n-old\n+new\n*** Add File: b.spec\n+one\n+two\n*** Delete File: c.spec\n*** End Patch\n";

    #[test]
    fn a_patch_is_read_into_the_files_it_leaves() {
        assert_eq!(paths(PATCH), ["a.spec", "b.spec", "c.spec"]);
        let texts = after(
            PATCH,
            |p| (p == "a.spec").then(|| "x\nkeep\nold\ny\n".to_string()),
            |_| true,
        )
        .unwrap();
        assert_eq!(texts["a.spec"], "x\nkeep\nnew\ny\n");
        assert_eq!(texts["b.spec"], "one\ntwo\n");
        assert_eq!(texts["c.spec"], "");
    }

    #[test]
    fn a_hunk_that_does_not_apply_is_the_hosts_to_refuse() {
        assert!(after(PATCH, |_| Some("other\n".into()), |_| true).is_none());
    }

    #[test]
    fn the_patch_is_found_in_whatever_field_holds_it() {
        assert!(text_of(&json!({"input": PATCH})).is_some());
        assert!(text_of(&json!(PATCH)).is_some());
        assert!(text_of(&json!({"command": "ls"})).is_none());
    }
}
