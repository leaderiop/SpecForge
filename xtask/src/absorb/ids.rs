//! The id map (ADR 0372 D3): every converted entity's new id and path, assigned before any draft, so a reference
//! names an entity whatever order the entities are drafted in, and the plan, its checks and the ledger key every new
//! entity by one path.
//!
//! A release's milestone is `v<version>`. A milestone a converted release lists is a phase under that version
//! milestone, numbered from `01` in source order (file, line) within its release; its path is `<version id>/<NN>`
//! and its create's id the number alone, placed by its `milestone` field (a name holds no `/`). Every other
//! converting entity keeps its old id here, until the requirement and decision ids are GSD's. The map is one to one
//! within each new kind: two entities given one path stop the run before any write, naming both.

use serde_json::Value;
use std::collections::BTreeMap;

use super::links::{Links, MILESTONE, RELEASE};
use super::plan::{self, Stop};
use super::read::OldEntity;

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

/// The id of the decision a feature's solution becomes.
pub fn solution_id(id: &str) -> String {
    format!("{id}_solution")
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
            _ => New {
                kind,
                name: e.id.clone(),
                scope: None,
            },
        };
        given
            .entry((kind, new.path()))
            .or_default()
            .push(format!("{} '{}'", e.kind, e.id));
        if e.fields.contains_key("solution") {
            given
                .entry(("decision", solution_id(&e.id)))
                .or_default()
                .push(format!("{} '{}' (its solution)", e.kind, e.id));
        }
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
