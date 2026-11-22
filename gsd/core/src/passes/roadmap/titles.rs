//! W109: a phase's title does not begin with a phase number. A phase is labelled `Phase {its id's last segment}:
//! {title}` (ADR 0387), so a title that carries a number of its own prints a second one, and a renumber (`phase
//! --remove` renames the later ids, never their titles) makes the two disagree. Every phase is read, a `999.x` backlog
//! phase too: it prints the same label.

use specforge_extension_sdk::prelude::*;

use super::of_kind;

pub fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "phase")
        .filter_map(|phase| {
            let title = phase.title.as_deref()?;
            let (number, rest) = numbered(title)?;
            let own = phase.id.rsplit('/').next().unwrap_or_default();
            let suggestion = if rest.is_empty() {
                format!("title it by what it delivers: its label reads 'Phase {own}: <title>'")
            } else {
                format!("retitle it '{rest}': its label then reads 'Phase {own}: {rest}'")
            };
            Some(
                PassDiagnostic::new(
                    "W109",
                    PassSeverity::Warning,
                    format!(
                        "phase '{}' is titled '{title}', which begins with the phase number '{number}': its label \
                         reads its number from its id",
                        phase.id
                    ),
                )
                .with_suggestion(suggestion)
                .with_entity(&phase.id),
            )
        })
        .collect()
}

/// The phase number `title` begins with and what follows it, separators dropped, when it begins with one: `Phase`
/// (any case) and a number in GSD's grammar (`\d+[A-Z]?(\.\d+)*`), then no letter or digit; or a bare number of
/// digits and dots, then `:` or `.` before a space or the end, `)`, or a dash between spaces (`03: Core`, `3. Core`,
/// `2) Core`, `04 - Core`). A number inside the title (`HTTP 2 support`, `2FA login`, `v2.0 migration`) is none.
pub(super) fn numbered(title: &str) -> Option<(&str, &str)> {
    let text = title.trim_start();
    let end = match text.get(..5) {
        Some(word) if word.eq_ignore_ascii_case("phase") => worded(&text[5..]).map(|n| 5 + n)?,
        _ => bare(text)?,
    };
    let rest = text[end..].trim_start_matches(|c: char| {
        c.is_whitespace() || matches!(c, ':' | '.' | ')' | '-' | '–' | '—')
    });
    Some((text[..end].trim_end(), rest.trim_end()))
}

/// The length of `rest` up to the end of the number after `Phase`: spaces, then a number in GSD's grammar, not
/// followed by a letter or a digit.
fn worded(rest: &str) -> Option<usize> {
    let spaces = rest.len() - rest.trim_start().len();
    let number = grammar(&rest[spaces..], true)?;
    let end = spaces + number;
    match rest[end..].chars().next() {
        Some(c) if c.is_alphanumeric() => None,
        _ => Some(end),
    }
}

/// The length of a bare number at the start of `text` when a label's separator follows it.
fn bare(text: &str) -> Option<usize> {
    let end = grammar(text, false)?;
    let after = &text[end..];
    let spaced = |tail: &str| tail.is_empty() || tail.starts_with(char::is_whitespace);
    let separated = match after.chars().next()? {
        ':' | '.' => spaced(&after[1..]),
        ')' => true,
        c if c.is_whitespace() => {
            let dash = after.trim_start();
            dash.starts_with(['-', '–', '—']) && spaced(&dash[dash.chars().next()?.len_utf8()..])
        }
        _ => false,
    };
    separated.then_some(end)
}

/// The length of the number at the start of `text`: digits, an optional capital letter when `lettered`, then any
/// `.digits`.
fn grammar(text: &str, lettered: bool) -> Option<usize> {
    let digits = |s: &str| s.len() - s.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let mut end = digits(text);
    if end == 0 {
        return None;
    }
    if lettered && text[end..].starts_with(|c: char| c.is_ascii_uppercase()) {
        end += 1;
    }
    while let Some(tail) = text[end..].strip_prefix('.') {
        match digits(tail) {
            0 => break,
            n => end += 1 + n,
        }
    }
    Some(end)
}
