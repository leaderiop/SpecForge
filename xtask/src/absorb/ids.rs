//! The id map (ADR 0372 D3): every converted entity's new id and path, assigned before any draft, so a reference
//! names an entity whatever order the entities are drafted in, and the plan, its checks and the ledger key every new
//! entity by one path. The ids are GSD's:
//!
//! - A requirement is `<CAT>-<NN>`. Its category ([`category`]) is a constraint's own `category` word, its first
//!   letter upper-cased, else the old kind's name (`Feature`, `Invariant`, `Constraint`); the prefix is the category's
//!   ASCII letters and digits upper-cased. The same rule writes the requirement's `category`, so for a category led
//!   by an ASCII letter the heading and the id agree letter for letter (`Data-integrity`, `DATAINTEGRITY-01`). When
//!   the letters are empty, start with a digit, or are the decisions' `D`, the prefix is the old kind's name and the
//!   heading is written as it is (`9lives`, `CONSTRAINT-01`; `D`, `CONSTRAINT-02`).
//! - A decision in force is `D-<NN>`, at project scope (its create writes no `phase`).
//! - Numbers run from 1 per (new kind, prefix) in (old kind, old id) order, bytewise, at least two digits and as
//!   many as the group's size needs: on the root spec `FEATURE-001` to `FEATURE-141`, `INVARIANT-001` to
//!   `INVARIANT-149`, `RELIABILITY-01` to `RELIABILITY-34`, `PERFORMANCE-01` to `PERFORMANCE-17`, `SECURITY-01`, and
//!   `D-001` to `D-100`. Old kinds whose requirements share a prefix share its counter, so no two share a number. No
//!   requirement takes the prefix `D`: the target holds one id per entity whatever its kind, so a requirement `D-01`
//!   beside the decision `D-01` would be one entity, and the plan refused.
//! - A release's milestone is `v<version>`. A milestone a converted release lists is a phase under that version
//!   milestone, numbered from `01` in source order (file, line) within its release; its path is `<version id>/<NN>`
//!   and its create's id the number alone, placed by its `milestone` field (a name holds no `/`).
//!
//! The map is one to one within each new kind: two entities given one path stop the run before any write, naming
//! both.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use super::links::{Links, MILESTONE, RELEASE};
use super::plan::{self, Stop};
use super::read::OldEntity;

/// The new kinds whose ids are numbered GSD's way.
const REQUIREMENT: &str = "requirement";
const DECISION: &str = "decision";
/// A decision's id prefix, which no requirement takes.
const DECISION_PREFIX: &str = "D";

/// The scope field of each new kind absorb writes that is scoped (ADR 0284): a phase is placed by its milestone.
pub const SCOPED: &[(&str, &str)] = &[("phase", "milestone")];

/// One converted entity's new identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct New {
    /// Its `@gsd/core` kind.
    pub kind: &'static str,
    /// Its create's id: the name within its scope.
    pub name: String,
    /// Its scope field and the path that field names, for a scoped kind.
    pub scope: Option<(&'static str, String)>,
}

impl New {
    /// Its path: its scope's path, `/`, its name; its name when it is not scoped.
    pub fn path(&self) -> String {
        match &self.scope {
            Some((_, scope)) => format!("{scope}/{}", self.name),
            None => self.name.clone(),
        }
    }
}

/// The map from an old (kind, id) to the new identity of every entity a run converts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ids(BTreeMap<(String, String), New>);

impl Ids {
    /// The new identity of the old entity (`kind`, `id`); none when it does not convert.
    pub fn get(&self, kind: &str, id: &str) -> Option<&New> {
        self.0.get(&(kind.to_string(), id.to_string()))
    }

    /// The new path of the old entity (`kind`, `id`).
    pub fn path(&self, kind: &str, id: &str) -> Option<String> {
        self.get(kind, id).map(New::path)
    }

    /// Every entry, sorted by (old kind, old id): (old kind, old id, new identity).
    #[cfg(test)]
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, &New)> {
        self.0
            .iter()
            .map(|((kind, id), new)| (kind.as_str(), id.as_str(), new))
    }
}

/// The id map of `entities` under `links`, and the stops: a release with no version, and two entities given one path
/// within one new kind (the map keeps the first). Pure, and the same for the same entities.
pub fn assign(entities: &[OldEntity], links: &Links) -> (Ids, Vec<Stop>) {
    let mut stops = Vec::new();
    let mut versions: BTreeMap<&str, String> = BTreeMap::new();
    for release in entities.iter().filter(|e| e.kind == RELEASE) {
        match release.fields.get("version").and_then(Value::as_str) {
            Some(version) if !version.trim().is_empty() => {
                versions.insert(&release.id, version_id(version));
            }
            _ => stops.push(Stop {
                kind: release.kind.clone(),
                id: release.id.clone(),
                field: "version".to_string(),
                why: "a release becomes the milestone its version names, and it has none"
                    .to_string(),
            }),
        }
    }
    let phases = phase_names(entities, links);
    let numbered = numbered(entities, links);
    let mut map = BTreeMap::new();
    // (new kind, path) → the old entities given it, as a stop names them.
    let mut given: BTreeMap<(&'static str, String), Vec<String>> = BTreeMap::new();
    for e in entities {
        let Some(kind) = plan::converts_to(e, links) else {
            continue;
        };
        let new = match e.kind.as_str() {
            RELEASE => match versions.get(e.id.as_str()) {
                Some(version) => New {
                    kind,
                    name: version.clone(),
                    scope: None,
                },
                None => continue,
            },
            MILESTONE => {
                let release = links.release_of(&e.id).unwrap_or_default();
                let (Some(version), Some(name)) = (versions.get(release), phases.get(&e.id)) else {
                    continue;
                };
                New {
                    kind,
                    name: name.clone(),
                    scope: Some((scope_field(kind), version.clone())),
                }
            }
            _ => match numbered.get(&(e.kind.clone(), e.id.clone())) {
                Some(name) => New {
                    kind,
                    name: name.clone(),
                    scope: None,
                },
                None => continue,
            },
        };
        given
            .entry((kind, new.path()))
            .or_default()
            .push(format!("{} '{}'", e.kind, e.id));
        map.entry((e.kind.clone(), e.id.clone())).or_insert(new);
    }
    for ((kind, path), makers) in given.into_iter().filter(|(_, m)| m.len() > 1) {
        stops.push(Stop {
            kind: String::new(),
            id: path,
            field: String::new(),
            why: format!(
                "two entities given one {kind} id, by {}",
                makers.join(" and ")
            ),
        });
    }
    (Ids(map), stops)
}

/// A requirement's category (ADR 0372 D3): a constraint's own `category` word, trimmed, its first letter upper-cased;
/// else the old kind's name, capitalised (`Feature`, `Invariant`, `Constraint`). The written `category` and the id's
/// prefix ([`prefix`]) both come from it.
pub fn category(e: &OldEntity) -> String {
    let word = (e.kind == "constraint")
        .then(|| e.fields.get("category").and_then(Value::as_str))
        .flatten()
        .map(str::trim)
        .filter(|word| !word.is_empty());
    let word = word.unwrap_or(&e.kind);
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// The id prefix of a requirement of `category` converted from an old `kind`: the category's ASCII letters and digits,
/// upper-cased; the old kind's name upper-cased when that is empty, starts with a digit, or is the decisions' `D`. It
/// matches `^[A-Z][A-Z0-9]*$`, and is never `D`.
fn prefix(category: &str, kind: &str) -> String {
    let letters = category
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_uppercase();
    if letters.starts_with(|c: char| c.is_ascii_uppercase()) && letters != DECISION_PREFIX {
        letters
    } else {
        kind.to_ascii_uppercase()
    }
}

/// Each converting requirement's and decision's id: `<prefix>-<NN>` for a requirement, `D-<NN>` for a decision,
/// numbered from 1 per (new kind, prefix) in (old kind, old id) order, at least two digits and as many as the group's
/// size needs. Two entities of one old (kind, id) get one id, which the map's one-to-one stop names.
fn numbered(entities: &[OldEntity], links: &Links) -> BTreeMap<(String, String), String> {
    let mut groups: BTreeMap<(&str, String), BTreeSet<(String, String)>> = BTreeMap::new();
    for e in entities {
        let group = match plan::converts_to(e, links) {
            Some(REQUIREMENT) => (REQUIREMENT, prefix(&category(e), &e.kind)),
            Some(DECISION) => (DECISION, DECISION_PREFIX.to_string()),
            _ => continue,
        };
        groups
            .entry(group)
            .or_default()
            .insert((e.kind.clone(), e.id.clone()));
    }
    let mut names = BTreeMap::new();
    for ((_, prefix), members) in groups {
        let width = members.len().to_string().len().max(2);
        for (n, old) in members.into_iter().enumerate() {
            names.insert(old, format!("{prefix}-{:0width$}", n + 1));
        }
    }
    names
}

/// The id of the milestone a release of `version` becomes: `v` and the version, with no second `v`.
fn version_id(version: &str) -> String {
    let version = version.trim();
    if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{version}")
    }
}

/// The scope field of a scoped new kind.
fn scope_field(kind: &str) -> &'static str {
    SCOPED
        .iter()
        .find(|(scoped, _)| *scoped == kind)
        .map_or("", |(_, field)| field)
}

/// Each milestone with a release, numbered from 1 in its release in source order (file, line, then id), at least two
/// digits and as many as the release's count needs.
fn phase_names(entities: &[OldEntity], links: &Links) -> BTreeMap<String, String> {
    let mut by_release: BTreeMap<&str, Vec<&OldEntity>> = BTreeMap::new();
    for e in entities.iter().filter(|e| e.kind == MILESTONE) {
        if let Some(release) = links.release_of(&e.id) {
            by_release.entry(release).or_default().push(e);
        }
    }
    let mut names = BTreeMap::new();
    for milestones in by_release.values_mut() {
        milestones.sort_by(|a, b| (&a.file, a.line, &a.id).cmp(&(&b.file, b.line, &b.id)));
        let width = milestones.len().to_string().len().max(2);
        for (n, e) in milestones.iter().enumerate() {
            names.insert(e.id.clone(), format!("{:0width$}", n + 1));
        }
    }
    names
}
