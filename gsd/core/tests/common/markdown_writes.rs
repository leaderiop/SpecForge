//! The write detectors (ADR 0332 D9, ADR 0343 D10): a line that tells an agent to write a GSD file, in kept and own text
//! alike. In prose that is a write verb and a GSD file name in one sentence; in a shell fence, an output redirect or
//! `tee` into one, the redirect's direction read by the SDK shell reader. No asset holds one.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

use std::collections::BTreeSet;
use std::ops::Range;

use specforge_extension_sdk::shell;

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

/// The words that tell an agent to generate: no asset holds one.
pub const GENERATE: &[&str] = &["generate", "generates", "generated", "generating"];

/// The words beyond [`VERBS`] that tell an agent to put an entry in a state file: recorded or noted there, a value
/// filled in.
const ENTRY_VERBS: &[&str] = &[
    "record",
    "records",
    "recorded",
    "recording",
    "noted",
    "fill",
    "fills",
    "filled",
];

/// The words beyond [`ENTRY_VERBS`] that edit a part of a file named just after them: "mark the frontmatter".
const MARK: &[&str] = &["mark", "marks", "marked"];

/// The words that name part of a state file: "Update Tests section", "Update frontmatter.updated".
const PARTS: &[&str] = &["section", "sections", "frontmatter"];

/// How many words may stand between a verb and the [`PARTS`] word it edits ("Append to Gaps section").
const PART_REACH: usize = 2;

/// What a sentence must hold: one of `verbs` with a GSD file name in the sentence, or one of `part_verbs` (not the
/// head of a compound: "append-only") with a [`PARTS`] word within [`PART_REACH`] words after it; and with
/// `redirects`, a shell line that writes a GSD file counts.
#[derive(Clone, Copy)]
struct Rule<'a> {
    verbs: &'a [&'a [&'a str]],
    part_verbs: &'a [&'a [&'a str]],
    redirects: bool,
}

/// The 0-based lines of `text` that tell an agent to write a GSD file, each once.
pub fn hits(text: &str) -> BTreeSet<usize> {
    hits_by(
        text,
        Rule {
            verbs: &[VERBS],
            part_verbs: &[],
            redirects: true,
        },
    )
}

/// The 0-based lines of `text` that tell an agent to edit a state file or part of one: a write or [`ENTRY_VERBS`]
/// word with a GSD file name in one sentence ("record overrides in MILESTONES.md"), or one of those or [`MARK`]
/// followed closely by a [`PARTS`] word ("Update Tests section"), in prose or a line of a fence that is not shell;
/// and a shell line that writes a GSD file.
pub fn section_edits(text: &str) -> BTreeSet<usize> {
    hits_by(
        text,
        Rule {
            verbs: &[VERBS, ENTRY_VERBS],
            part_verbs: &[VERBS, ENTRY_VERBS, MARK],
            redirects: true,
        },
    )
}

/// The 0-based lines of `text` that tell an agent to generate a GSD file: a [`GENERATE`] word and a GSD file name in
/// one sentence of prose, or of a line of a fence that is not shell.
pub fn generates(text: &str) -> BTreeSet<usize> {
    hits_by(
        text,
        Rule {
            verbs: &[GENERATE],
            part_verbs: &[],
            redirects: false,
        },
    )
}

/// The lines of `text` whose sentences hold what `rule` asks, and with its `redirects` a shell fence's line that
/// writes a GSD file.
fn hits_by(text: &str, rule: Rule) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for block in gsd_kit::markdown::inline_blocks(text) {
        let quoted: Vec<Range<usize>> = block
            .links
            .iter()
            .filter(|(_, dest)| dest.starts_with(gsd_kit::files::UPSTREAM_BLOB))
            .map(|(range, _)| range.clone())
            .collect();
        for sentence in sentences(&block.text) {
            if let Some(at) = writes_in(&block.text, sentence, &quoted, rule) {
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
                rule.redirects && shell_writes(line)
            } else {
                // a template or an example is prose, line by line
                sentences(line)
                    .into_iter()
                    .any(|s| writes_in(line, s, &[], rule).is_some())
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

/// The byte (in `text`) of what the sentence `range` edits when it also holds one of `rule`'s verbs that counts,
/// outside the `quoted` ranges: its first GSD file name, else a [`PARTS`] word close after a part verb.
fn writes_in(
    text: &str,
    range: Range<usize>,
    quoted: &[Range<usize>],
    rule: Rule,
) -> Option<usize> {
    let sentence = &text[range.clone()];
    let name = gsd_kit::files::planning_names(sentence)
        .into_iter()
        .map(|(r, _)| range.start + r.start..range.start + r.end)
        .find(|r| !quoted.iter().any(|q| q.start <= r.start && r.end <= q.end));
    if name.is_none() && rule.part_verbs.is_empty() {
        return None;
    }
    let words = words(sentence);
    let part_after = |k: usize| {
        words[k + 1..]
            .iter()
            .take(PART_REACH + 1)
            .find(|(_, w)| PARTS.contains(&w.to_ascii_lowercase().as_str()))
            .map(|(at, _)| range.start + at)
    };
    words.iter().enumerate().find_map(|(k, (at, word))| {
        let lower = word.to_ascii_lowercase();
        let of = |lists: &[&[&str]]| lists.iter().any(|v| v.contains(&lower.as_str()));
        let compound = sentence.as_bytes().get(at + word.len()) == Some(&b'-');
        let (verb, part_verb) = (of(rule.verbs), of(rule.part_verbs) && !compound);
        if !(verb || part_verb) || quoted.iter().any(|q| q.contains(&(range.start + at))) {
            return None;
        }
        let slash = |i: Option<usize>| i.and_then(|i| sentence.as_bytes().get(i)) == Some(&b'/');
        let (before, after) = (at.checked_sub(1), Some(at + word.len()));
        // 1. a slash alternative is a menu choice: `update/skip`, `Review/edit`
        if slash(before) || slash(after) {
            return None;
        }
        // 2. the agent offers the choice: the word before the verb, or before its alternative
        let offered = words[..k]
            .last()
            .is_some_and(|(_, w)| OFFER.contains(&w.to_ascii_lowercase().as_str()));
        // 3. the person is its subject
        let person = words[..k]
            .iter()
            .any(|(_, w)| PERSON.contains(&w.to_ascii_lowercase().as_str()));
        if offered || person {
            return None;
        }
        name.as_ref()
            .filter(|_| verb)
            .map(|n| n.start)
            .or_else(|| part_verb.then(|| part_after(k)).flatten())
    })
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
