//! Runs an asset's typed-edit fence as the asset holds it: the model's values filled in, under bash with only
//! `specforge` on the `PATH`, on a project whose check report is read before and after.

use super::Shell;
use crate::harness::Project;

/// What a filled fence did: its output, its exit code, and the codes `check` reports after it that it did not before
/// (each code counted, so a second `E006` is added even when one was there; the judged rules' codes left out, as
/// [`Project::codes`] leaves them).
pub(super) struct Applied {
    pub out: String,
    pub err: String,
    pub code: i32,
    pub added: Vec<String>,
}

/// `fence` with the model's values filled in. A key `NAME` gives the assignment `NAME="…"` (or `NAME='…'`) its
/// value; a key `{placeholder}` replaces that text wherever it stands. A key that fills nothing panics, and so does an
/// assignment that still holds a `{…}` placeholder afterwards: the test must fill every value the model writes.
pub(super) fn filled(fence: &str, values: &[(&str, &str)]) -> String {
    let mut text = fence.to_string();
    for (key, value) in values {
        if key.starts_with('{') {
            assert!(text.contains(key), "no {key} in the fence:\n{fence}");
            text = text.replace(key, value);
            continue;
        }
        let mut hit = false;
        let lines: Vec<String> = text
            .lines()
            .map(|line| {
                let Some(rest) = line.strip_prefix(&format!("{key}=")) else {
                    return line.to_string();
                };
                let quote = rest.chars().next().filter(|q| *q == '"' || *q == '\'');
                hit = quote.is_some();
                match quote {
                    Some(q) => format!("{key}={q}{value}{q}"),
                    None => line.to_string(),
                }
            })
            .collect();
        assert!(hit, "no quoted assignment {key}= in the fence:\n{fence}");
        text = lines.join("\n");
    }
    let placeholder = regex::Regex::new(r"^[A-Z][A-Z0-9_]*=.*[^$]\{[A-Za-z]").expect("a regex");
    for line in text.lines() {
        assert!(
            !placeholder.is_match(line),
            "a value the model writes is left unfilled: {line}"
        );
    }
    text
}

/// Runs `script` in `p`'s root under `sh`, with `p`'s check codes read before and after.
pub(super) fn apply(sh: &Shell, p: &Project, script: &str) -> Applied {
    let before = p.codes();
    let (out, err, code) = sh.run_in(p.root(), script);
    let mut left = before;
    let mut added = Vec::new();
    for found in p.codes() {
        match left.iter().position(|c| *c == found) {
            Some(at) => {
                left.remove(at);
            }
            None => added.push(found),
        }
    }
    Applied {
        out,
        err,
        code,
        added,
    }
}

/// The code of the first `bash` fence after the first line that starts (after its indent) with `heading`, as
/// gsd-kit's Markdown reader reads it.
pub(super) fn typed_fence(text: &str, heading: &str) -> String {
    let at = text
        .lines()
        .position(|l| l.trim_start().starts_with(heading))
        .unwrap_or_else(|| panic!("no line starts with {heading}"));
    let fence = gsd_kit::markdown::fences(text)
        .into_iter()
        .find(|f| f.language == "bash" && f.open > at)
        .unwrap_or_else(|| panic!("no bash fence after {heading}"));
    super::code(&fence.text).to_string()
}
