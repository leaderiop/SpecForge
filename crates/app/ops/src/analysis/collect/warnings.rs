//! A collector's warnings (ADR 0389): the lines its export returns about what it read but could not use, each one
//! W203 naming the collector, escaped, at most [`WARNINGS_SHOWN`] of them and then one counting the rest.

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
