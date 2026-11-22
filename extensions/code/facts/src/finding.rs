//! The `static_finding` facts of a SARIF report (ADR 0388): what a reader (`specforge-code-sarif`) read of each
//! result, and the facts minted from it, each citing the innermost served `item` holding its location, else the
//! `package` holding its file. A level is what the report states, never SARIF's own `warning` default (F5).

use specforge_extension_sdk::prelude::{Fact, FactLocation};

use crate::places::Places;
use crate::report::{id_part, Repeats, STATIC_FINDING, UNKNOWN};

/// A result's level: SARIF's four, or [`UNKNOWN`] when the report states none (F5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Error,
    Warning,
    Note,
    None,
    Unknown,
}

/// Each level and its name: the one table [`Level::NAMES`], [`Level::as_str`] and [`Level::parse`] read.
const LEVELS: [(Level, &str); 5] = [
    (Level::Error, "error"),
    (Level::Warning, "warning"),
    (Level::Note, "note"),
    (Level::None, "none"),
    (Level::Unknown, UNKNOWN),
];

impl Level {
    /// Every level's name, SARIF's four then `unknown`: `@code/core`'s vocabulary of `static_finding.level`.
    pub const NAMES: [&'static str; 5] = {
        let mut names = [""; 5];
        let mut at = 0;
        while at < names.len() {
            names[at] = LEVELS[at].1;
            at += 1;
        }
        names
    };

    pub fn as_str(self) -> &'static str {
        LEVELS
            .iter()
            .find(|(level, _)| *level == self)
            .map_or(UNKNOWN, |(_, name)| name)
    }

    /// One of SARIF's four level values (`error`, `warning`, `note`, `none`); `None` for any other text, `unknown`
    /// included, which no report writes.
    pub fn parse(text: &str) -> Option<Level> {
        LEVELS
            .iter()
            .find(|(level, name)| *level != Level::Unknown && *name == text)
            .map(|(level, _)| *level)
    }
}

/// What a SARIF report says of one result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SarifResult {
    /// The run's `tool.driver.name`.
    pub tool: String,
    /// The rule id, [`UNKNOWN`] when the result states none.
    pub rule: String,
    pub level: Level,
    /// `message.text`, empty when absent.
    pub message: String,
    /// The file of its first physical location, project-relative, with `/` separators.
    pub file: Option<String>,
    /// Its 1-based start line.
    pub line: Option<usize>,
    /// Its 1-based end line, inclusive.
    pub end_line: Option<usize>,
}

/// The `static_finding` facts of `results`, in document order, attributed by the places of the `served` facts.
///
/// The id is `sarif:<tool>/<rule>`, then `#<file>:<line>` when both are known or `#<file>` when only the file is, each
/// part through [`id_part`]; a second and later result of one id take `~2`, `~3`, … (F4). Fields: `tool`, `rule`,
/// `level`, `message`, and `file` and `line` when known; `item`, the innermost served item holding the line, else
/// `package`, the served package holding the file, else neither. Its place spans whole lines, `line` to `end_line`:
/// SARIF columns have no fixed unit, so none is claimed.
pub fn finding_facts(results: &[SarifResult], served: &[Fact]) -> Vec<Fact> {
    let places = Places::new(served);
    let mut repeats = Repeats::default();
    results
        .iter()
        .map(|result| {
            let mut id = format!(
                "{}:{}/{}",
                STATIC_FINDING.scheme,
                id_part(&result.tool),
                id_part(&result.rule)
            );
            match (&result.file, result.line) {
                (Some(file), Some(line)) => id.push_str(&format!("#{}:{line}", id_part(file))),
                (Some(file), None) => id.push_str(&format!("#{}", id_part(file))),
                _ => {}
            }
            let mut fact = Fact::new(STATIC_FINDING.kind, repeats.id(id))
                .field("tool", result.tool.as_str())
                .field("rule", result.rule.as_str())
                .field("level", result.level.as_str())
                .field("message", result.message.as_str());
            let Some(file) = &result.file else {
                return fact;
            };
            fact = fact.field("file", file.as_str());
            if let Some(line) = result.line {
                let last = result.end_line.unwrap_or(line).max(line);
                fact = fact.field("line", line).at(FactLocation {
                    file: file.clone(),
                    start_line: line,
                    start_col: 1,
                    end_line: last.saturating_add(1),
                    end_col: 1,
                });
            }
            match result.line.and_then(|line| places.innermost(file, line)) {
                Some(item) => fact.field("item", item),
                None => match places.package_of(file) {
                    Some(package) => fact.field("package", package),
                    None => fact,
                },
            }
        })
        .collect()
}
