//! The write ledger (ADR 0332 D9): a line that tells an agent to write a GSD file is a row of `markdown_writes.txt`,
//! `<asset>\t<trimmed line>`, in kept and own text alike; its row count is the port's write measure. In prose that is a
//! write verb and a GSD file name in one sentence; in a shell fence, an output redirect or `tee` into one, the
//! redirect's direction read by the SDK shell reader.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

use std::collections::BTreeSet;
use std::ops::Range;

use specforge_extension_sdk::shell;

/// The ledger's rows.
pub const MARKDOWN_WRITES: &str = include_str!("markdown_writes.txt");

/// The words that tell an agent to write.
const VERBS: &[&str] = &[
    "write",
    "writes",
    "wrote",
    "written",
    "writing",
    "create",
    "creates",
    "created",
    "creating",
    "update",
    "updates",
    "updated",
    "updating",
    "append",
    "appends",
    "appended",
    "appending",
    "overwrite",
    "overwrites",
    "overwrote",
    "overwritten",
    "overwriting",
    "save",
    "saves",
    "saved",
    "saving",
    "edit",
    "edits",
    "edited",
    "editing",
];

/// A verb after one of these offers the person a choice: the agent writes nothing.
const OFFER: &[&str] = &["offer", "offers", "offered", "offering"];

/// A verb after one of these in its sentence is the person's to do.
const PERSON: &[&str] = &["user", "users", "human", "person"];

/// The fence languages read as shell.
const SHELL: &[&str] = &["bash", "sh", "shell", "zsh"];

/// The 0-based lines of `text` that tell an agent to write a GSD file, each once.
pub fn hits(text: &str) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for block in gsd_kit::markdown::inline_blocks(text) {
        let quoted: Vec<Range<usize>> = block
            .links
            .iter()
            .filter(|(_, dest)| dest.starts_with(gsd_kit::files::UPSTREAM_BLOB))
            .map(|(range, _)| range.clone())
            .collect();
        for sentence in sentences(&block.text) {
            if let Some(at) = writes_in(&block.text, sentence, &quoted) {
                out.insert(block.lines[at]);
            }
        }
    }
    let lines: Vec<&str> = text.split('\n').collect();
    for fence in gsd_kit::markdown::fences(text) {
        let shell = SHELL.contains(&fence.language.as_str());
        for n in fence.code.clone() {
            let line = lines.get(n).copied().unwrap_or_default();
            let hit = if shell {
                shell_writes(line)
            } else {
                // a template or an example is prose, line by line
                sentences(line)
                    .into_iter()
                    .any(|s| writes_in(line, s, &[]).is_some())
            };
            if hit {
                out.insert(n);
            }
        }
    }
    out
}

/// Whether `text` tells an agent to write a GSD file anywhere.
pub fn writes(text: &str) -> bool {
    !hits(text).is_empty()
}

/// The sentences of `text`, as byte ranges: cut after a `.`, `!` or `?` that whitespace or the end follows.
fn sentences(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    for (i, b) in bytes.iter().enumerate() {
        let end = matches!(b, b'.' | b'!' | b'?')
            && bytes.get(i + 1).is_none_or(|n| n.is_ascii_whitespace());
        if end {
            out.push(start..i + 1);
            start = i + 1;
        }
    }
    if start < text.len() {
        out.push(start..text.len());
    }
    out
}

/// The byte (in `text`) of the first GSD file name of the sentence `range` when it also holds a write verb that
/// counts, outside the `quoted` ranges.
fn writes_in(text: &str, range: Range<usize>, quoted: &[Range<usize>]) -> Option<usize> {
    let sentence = &text[range.clone()];
    let name = gsd_kit::files::planning_names(sentence)
        .into_iter()
        .map(|(r, _)| range.start + r.start..range.start + r.end)
        .find(|r| !quoted.iter().any(|q| q.start <= r.start && r.end <= q.end))?;
    let words = words(sentence);
    let counts = words.iter().enumerate().any(|(k, (at, word))| {
        let lower = word.to_ascii_lowercase();
        if !VERBS.contains(&lower.as_str())
            || quoted.iter().any(|q| q.contains(&(range.start + at)))
        {
            return false;
        }
        let slash = |i: Option<usize>| i.and_then(|i| sentence.as_bytes().get(i)) == Some(&b'/');
        let (before, after) = (at.checked_sub(1), Some(at + word.len()));
        // 1. a slash alternative is a menu choice: `update/skip`, `Review/edit`
        if slash(before) || slash(after) {
            return false;
        }
        // 2. the agent offers the choice: the word before the verb, or before its alternative
        let offered = words[..k]
            .last()
            .is_some_and(|(_, w)| OFFER.contains(&w.to_ascii_lowercase().as_str()));
        // 3. the person is its subject
        let person = words[..k]
            .iter()
            .any(|(_, w)| PERSON.contains(&w.to_ascii_lowercase().as_str()));
        !offered && !person
    });
    counts.then_some(name.start)
}

/// The words of `sentence` with their byte offsets: runs of ASCII letters.
fn words(sentence: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in sentence.char_indices() {
        match (c.is_ascii_alphabetic(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, &sentence[s..i]));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, &sentence[s..]));
    }
    out
}

/// Whether the shell line `line` writes a GSD file: an output redirect into one, or `tee` with one among its words.
/// Input redirects, heredocs and descriptor duplications never do; the direction is the SDK reader's.
// shortcut: a line inside a multi-line quote does not parse alone and is skipped; read the whole fence once the SDK
// reader gives a segment its line
fn shell_writes(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') {
        return false;
    }
    let names = |word: &str| !gsd_kit::files::planning_names(word).is_empty();
    shell::segments(line).is_ok_and(|segments| {
        segments.iter().any(|segment| {
            segment
                .redirects
                .iter()
                .any(|r| r.op.writes() && names(&r.target))
                || (segment.words.first().is_some_and(|w| w == "tee")
                    && segment.words[1..].iter().any(|w| names(w)))
        })
    })
}

/// The ledger rows of `asset`'s `lines` (its body after the attribution header).
pub fn rows_of(asset: &str, lines: &[String]) -> Vec<Vec<String>> {
    hits(&lines.join("\n"))
        .into_iter()
        .map(|n| vec![asset.to_string(), lines[n].trim().to_string()])
        .collect()
}
