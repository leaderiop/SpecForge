//! Phase numbers: GSD's grammar (`\d+[A-Z]?(\.\d+)*`, case-insensitive: `1`, `01`, `12A`, `12.1`, `3.2.1`; upstream
//! `PHASE_NUMBER_TOKEN_SOURCE`), their order (upstream `comparePhaseNum`) and `gsd/W104` over a milestone's phases.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::{milestone_of, of_kind};

/// A phase number as a key: the integer, an optional letter (`12A`), then the decimal parts (`.1.1`). The derived
/// order is `comparePhaseNum`'s: `12 < 12A < 12.1 < 12.2 < 12.10 < 13`, and a prefix sorts before its extensions.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Number {
    pub whole: u64,
    pub letter: Option<char>,
    pub parts: Vec<u64>,
}

impl Number {
    /// The number a phase id ends in (`v1.0/03.1` is `3.1`); `None` for a name outside the grammar (a custom-named
    /// phase), which the numbering checks leave alone (`spec/rules.md`, gsd/W104).
    pub fn of(id: &str) -> Option<Number> {
        let name = id.rsplit('/').next()?;
        let mut pieces = name.split('.');
        let head = pieces.next()?;
        let (digits, letter) = match head.char_indices().last() {
            Some((at, c)) if c.is_ascii_alphabetic() => (&head[..at], Some(c.to_ascii_uppercase())),
            _ => (head, None),
        };
        let number = |s: &str| {
            (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                .then(|| s.parse().ok())
                .flatten()
        };
        Some(Number {
            whole: number(digits)?,
            letter,
            parts: pieces.map(number).collect::<Option<_>>()?,
        })
    }

    /// A backlog phase is a `999.x`: outside the numbering.
    pub fn backlog(&self) -> bool {
        self.whole == 999
    }

    /// What an inserted phase follows: its number without the last decimal part.
    fn parent(&self) -> Number {
        let mut parent = self.clone();
        parent.parts.pop();
        parent
    }

    /// The phase an inserted `.n` follows when `n > 1`: the same number with `.n-1`.
    fn previous_sibling(&self) -> Option<Number> {
        let last = *self.parts.last().filter(|n| **n > 1)?;
        let mut sibling = self.clone();
        *sibling.parts.last_mut()? = last - 1;
        Some(sibling)
    }
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}", self.whole)?;
        if let Some(letter) = self.letter {
            write!(f, "{letter}")?;
        }
        self.parts.iter().try_for_each(|p| write!(f, ".{p}"))
    }
}

fn warning(message: String, suggestion: String, id: &str) -> PassDiagnostic {
    PassDiagnostic::new("W104", PassSeverity::Warning, message)
        .with_suggestion(suggestion)
        .with_entity(id)
}

/// W104: in each milestone the integer phases are continuous (a run of missing numbers is one warning), no two
/// phases share a number, and an inserted phase follows the phase it is numbered from. The work is
/// `O(phases log phases)` whatever the numbers are.
pub fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut by_milestone: BTreeMap<&str, Vec<(Number, &str)>> = BTreeMap::new();
    for phase in of_kind(input, "phase") {
        if let Some(number) = Number::of(&phase.id).filter(|n| !n.backlog()) {
            by_milestone
                .entry(milestone_of(&phase.id))
                .or_default()
                .push((number, phase.id.as_str()));
        }
    }
    let mut found = Vec::new();
    for phases in by_milestone.values() {
        found.extend(duplicates(phases));
        found.extend(gaps(phases));
        found.extend(inserted(phases));
    }
    found
}

/// Two ids with one number (`03` and `003`): reported on the one that sorts later.
fn duplicates(phases: &[(Number, &str)]) -> Vec<PassDiagnostic> {
    let mut first: BTreeMap<&Number, &str> = BTreeMap::new();
    let mut ordered: Vec<&(Number, &str)> = phases.iter().collect();
    ordered.sort_by_key(|(_, id)| *id);
    ordered
        .into_iter()
        .filter_map(|(number, id)| {
            first.insert(number, id).map(|earlier| {
                warning(
                    format!("phases '{earlier}' and '{id}' have the same number {number}"),
                    "give each phase its own number".into(),
                    id,
                )
            })
        })
        .collect()
}

/// The integers the milestone's phases are numbered from, with the first phase of each: a run of missing integers
/// between two of them is one warning, on the phase after the run.
fn gaps(phases: &[(Number, &str)]) -> Vec<PassDiagnostic> {
    let mut firsts: BTreeMap<u64, (&Number, &str)> = BTreeMap::new();
    for (number, id) in phases.iter().filter(|(n, _)| n.parts.is_empty()) {
        let slot = firsts.entry(number.whole).or_insert((number, id));
        if number < slot.0 {
            *slot = (number, id);
        }
    }
    let ascending: Vec<(&u64, &(&Number, &str))> = firsts.iter().collect();
    ascending
        .windows(2)
        .filter(|pair| *pair[1].0 > pair[0].0 + 1)
        .map(|pair| {
            let (from, to) = (pair[0].0 + 1, pair[1].0 - 1);
            let id = pair[1].1.1;
            let skipped = if from == to {
                format!("{from:02}")
            } else {
                format!("{from:02}-{to:02}")
            };
            warning(
                format!("phase numbers skip {skipped}, before '{id}'"),
                "renumber with /gsd-phase, or insert the missing phase".into(),
                id,
            )
        })
        .collect()
}

/// An inserted phase (`02.1`, `02.1.1`) needs the phase it is numbered from, and `.n` needs `.n-1`.
fn inserted(phases: &[(Number, &str)]) -> Vec<PassDiagnostic> {
    let numbers: BTreeSet<&Number> = phases.iter().map(|(n, _)| n).collect();
    phases
        .iter()
        .filter(|(n, _)| !n.parts.is_empty())
        .filter_map(|(number, id)| {
            let parent = number.parent();
            if !numbers.contains(&parent) {
                return Some(warning(
                    format!("inserted phase '{id}' follows phase {parent}, which does not exist"),
                    format!("an inserted phase sits after the phase it is numbered from; add phase {parent}"),
                    id,
                ));
            }
            let sibling = number.previous_sibling().filter(|s| !numbers.contains(s))?;
            Some(warning(
                format!("inserted phase '{id}' skips {sibling}"),
                "inserted phases are numbered .1, .2, … in order".into(),
                id,
            ))
        })
        .collect()
}
