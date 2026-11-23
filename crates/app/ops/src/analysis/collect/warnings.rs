//! A collector's warnings (ADR 0389): the lines its export returns about what it read but could not use, each one
//! W203 naming the collector, escaped, at most [`WARNINGS_SHOWN`] of them and then one counting the rest; and the host's
//! own W203 naming the report glob matches it did not read.

use super::Collector;
use specforge_common::remedy::printable;
use specforge_common::{Diagnostic, codes};

/// The most W203 lines one collector reports before one more counts the rest.
const WARNINGS_SHOWN: usize = 100;

/// W203 for each of `collector`'s warning `lines`, up to [`WARNINGS_SHOWN`], then one counting the lines not shown.
pub(super) fn warned(collector: &Collector, lines: &[String]) -> Vec<Diagnostic> {
    let mut warned: Vec<Diagnostic> = lines
        .iter()
        .take(WARNINGS_SHOWN)
        .map(|line| {
            Diagnostic::new(
                codes::W203,
                format!(
                    "collector '{}' of {} warns: {}",
                    collector.name,
                    collector.extension,
                    printable(line)
                ),
            )
        })
        .collect();
    if let Some(more) = lines.len().checked_sub(WARNINGS_SHOWN).filter(|n| *n > 0) {
        warned.push(Diagnostic::new(
            codes::W203,
            format!(
                "collector '{}' of {} returned {more} more warnings, not shown",
                collector.name, collector.extension
            ),
        ));
    }
    warned
}

/// The most report glob matches one W203 names before it counts the rest.
const UNREAD_SHOWN: usize = 5;

/// One W203 naming the report glob matches of `collector` that land outside the project and were not read (ADR 0389
/// D6): the first [`UNREAD_SHOWN`], then how many more; none when every match was read.
pub(super) fn unread(collector: &Collector, paths: &[String]) -> Option<Diagnostic> {
    (!paths.is_empty()).then(|| {
        Diagnostic::new(
            codes::W203,
            format!(
                "collector '{}' of {}: {}",
                collector.name,
                collector.extension,
                unread_paths(paths)
            ),
        )
    })
}

/// `<n> report glob match(es) land outside ...: ` and the first [`UNREAD_SHOWN`] of `paths`, escaped, then how many
/// more: what the W203 says, and what E045 adds when no other match was read.
pub(super) fn unread_paths(paths: &[String]) -> String {
    let mut named: Vec<String> = paths
        .iter()
        .take(UNREAD_SHOWN)
        .map(|path| printable(path).into_owned())
        .collect();
    if paths.len() > UNREAD_SHOWN {
        named.push(format!("and {} more", paths.len() - UNREAD_SHOWN));
    }
    format!(
        "{} report glob match(es) land outside the project through a link, so they were not read: {}",
        paths.len(),
        named.join(", ")
    )
}
