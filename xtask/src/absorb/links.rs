//! The facts a run reads across entities, computed once from the entities it converts (ADR 0372 D4): the release a
//! milestone sits in, and the old ids a written phase lists as its requirements. A pure rule of one entity cannot see
//! them: an old milestone names no release (the release lists it), and an old feature names no milestone (the
//! milestone lists it). The id map ([`super::ids`]), the whole drops and the tier rule read them.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use super::how;
use super::plan::Stop;
use super::read::OldEntity;

/// The old kind a release is.
pub const RELEASE: &str = "release";
/// The old kind a milestone is.
pub const MILESTONE: &str = "milestone";

/// The cross-entity facts of one run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Links {
    /// Each converted milestone's release: the release among the run's entities whose `milestones` names it.
    release_of: BTreeMap<String, String>,
    /// The old ids the `features` of a milestone with a release names: what the written phases list.
    listed: BTreeSet<String>,
}

impl Links {
    /// The links of `entities`, and a stop for each milestone two releases list (it would sit under both).
    pub fn of(entities: &[OldEntity]) -> (Links, Vec<Stop>) {
        let milestones: BTreeSet<&str> = entities
            .iter()
            .filter(|e| e.kind == MILESTONE)
            .map(|e| e.id.as_str())
            .collect();
        let mut releases: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for release in entities.iter().filter(|e| e.kind == RELEASE) {
            for listed in listed_ids(release.fields.get("milestones")) {
                if let Some(&milestone) = milestones.get(listed.as_str()) {
                    let held = releases.entry(milestone).or_default();
                    if !held.contains(&release.id.as_str()) {
                        held.push(&release.id);
                    }
                }
            }
        }
        let mut links = Links::default();
        let mut stops = Vec::new();
        for (milestone, held) in releases {
            if let [release] = held.as_slice() {
                links
                    .release_of
                    .insert(milestone.to_string(), release.to_string());
            } else {
                let named: Vec<String> = held.iter().map(|r| format!("'{r}'")).collect();
                stops.push(Stop {
                    kind: MILESTONE.to_string(),
                    id: milestone.to_string(),
                    field: String::new(),
                    why: format!(
                        "listed by {} releases, {}: a phase sits under one version milestone",
                        held.len(),
                        named.join(" and ")
                    ),
                });
            }
        }
        for milestone in entities
            .iter()
            .filter(|e| e.kind == MILESTONE && links.release_of.contains_key(&e.id))
        {
            links
                .listed
                .extend(listed_ids(milestone.fields.get("features")));
        }
        (links, stops)
    }

    /// The release the milestone `id` sits in: none when no release of the run lists it, so it becomes no phase.
    pub fn release_of(&self, id: &str) -> Option<&str> {
        self.release_of.get(id).map(String::as_str)
    }

    /// Whether a written phase lists the old id `id` among its requirements.
    pub fn listed(&self, id: &str) -> bool {
        self.listed.contains(id)
    }

    /// Whether the run writes a phase: `gsd/E101` waits for the first one (ADR 0372 D7).
    pub fn writes_phase(&self) -> bool {
        !self.release_of.is_empty()
    }
}

/// The ids a list of references names, each once.
fn listed_ids(value: Option<&Value>) -> Vec<String> {
    let mut ids = Vec::new();
    for id in value.map(how::items).unwrap_or_default() {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}
