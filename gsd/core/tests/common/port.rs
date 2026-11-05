//! The port rule (D-r40-5, D-r42-4), shared by the agent bodies and the workflow assets: an asset is upstream's text
//! line for line, and a line may differ only where upstream's names a tool this port replaced, or runs a program that
//! `specforge` now answers in its place.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

use regex::Regex;

#[path = "command_word.rs"]
mod command_word;
#[path = "specforge_only.rs"]
mod specforge_only;

/// Whether `mine` stands for upstream's line `theirs` as a block that calls only `specforge`: it runs no program
/// upstream's does not, besides `specforge` and the shell's own words, and leaves out one that upstream's runs.
pub fn served_by_specforge(theirs: &str, mine: &str) -> bool {
    let programs = |line: &str| {
        command_word::commands(line)
            .into_iter()
            .filter(|tool| !specforge_only::SHELL_WORDS.contains(&tool.as_str()))
            .collect::<Vec<_>>()
    };
    let (before, after) = (programs(theirs), programs(mine));
    after
        .iter()
        .all(|tool| tool == "specforge" || before.contains(tool))
        && before.iter().any(|tool| !after.contains(tool))
}

/// Checks one asset against the body lines of its upstream file: the header line, then every line. `header` is the
/// upstream path the attribution names; `rewritten` says which body lines (0-based) differ although upstream's names
/// no ported tool. Returns how many lines differ.
pub fn check_lines(
    label: &str,
    header: &str,
    asset: &str,
    body: &[String],
    rx: &Regex,
    rewritten: &dyn Fn(usize) -> bool,
) -> usize {
    let header_line = asset.split('\n').next().unwrap_or_default();
    let lines = body_lines(asset);
    assert!(
        header_line.starts_with(&format!(
            "<!-- Ported from gsd-core v{} {header}, MIT",
            gsd_kit::tracked_version!()
        )),
        "{label}: header {header_line}"
    );
    assert_eq!(lines.len(), body.len(), "{label}: line count");
    let mut changed = 0;
    for (n, (mine, theirs)) in lines.iter().zip(body).enumerate() {
        if rx.is_match(theirs) {
            assert_ne!(
                mine, theirs,
                "{label}:{n}: a ported line is left as upstream's"
            );
            assert!(
                !rx.is_match(mine),
                "{label}:{n}: still names upstream's tools: {mine}"
            );
            changed += 1;
        } else if rewritten(n) || served_by_specforge(theirs, mine) {
            assert_ne!(
                mine, theirs,
                "{label}:{n}: listed as rewritten, but is upstream's"
            );
            changed += 1;
        } else {
            assert_eq!(mine, theirs, "{label}:{n}: differs outside a ported line");
        }
    }
    changed
}

/// The lines of `asset` after its attribution header: body line `n` of the asset is upstream's line `n`.
pub fn body_lines(asset: &str) -> Vec<&str> {
    asset.split('\n').skip(1).collect()
}

/// Whether one of `spans` (`(path, first, last)`, inclusive) holds body line `n` of `path`.
pub fn in_spans<'a>(
    spans: impl IntoIterator<Item = &'a (&'a str, usize, usize)>,
    path: &str,
    n: usize,
) -> bool {
    spans
        .into_iter()
        .any(|(p, a, b)| *p == path && (*a..=*b).contains(&n))
}
