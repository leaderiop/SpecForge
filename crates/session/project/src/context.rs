//! A context route read over a project (ADR 0356): each section of the route evaluated from one subject through the
//! judged inputs' evaluator, so a section digests exactly as a judged input of the same selector does; the route's
//! settings as one last section; and the digest of a pack over the sections it served.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use specforge_protocol_types::{CONTEXT_SETTINGS, ContextTier, JudgedFile};
use specforge_registry::{ContextRoute, Disk};

use crate::CompiledProject;
use crate::judged::{Cx, hash};

/// What a section reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Entities,
    Files,
    Tests,
    Settings,
}

/// One section of a route, read from one subject.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionRead {
    pub name: String,
    pub tier: ContextTier,
    /// Its selector, printed canonically; empty for the settings section.
    pub selector: String,
    pub kind: SectionKind,
    /// The entities (by path) or the test members it picked, or the paths of the files it read.
    pub members: Vec<String>,
    /// The files it read, with their digests: empty unless it reads files.
    pub files: Vec<JudgedFile>,
    /// The settings section's values by key, `null` for one unset; empty for any other section.
    pub settings: BTreeMap<String, Value>,
    pub digest: String,
}

fn cx<'a>(project: &'a CompiledProject, disk: &'a dyn Disk) -> Cx<'a> {
    Cx::new(
        project.environment(),
        project.graph(),
        project.entities(),
        disk,
        project.digests(),
        project.checked_values(),
    )
}

/// The sections of `route` read from `subject`, in declared order, the settings section (core, named `settings`) last
/// when the route carries settings. Files are read through `disk`.
pub fn read(
    project: &CompiledProject,
    disk: &dyn Disk,
    route: &ContextRoute,
    subject: &str,
) -> Vec<SectionRead> {
    let named: Vec<_> = route
        .sections
        .iter()
        .map(|section| (section.name.clone(), section.selector.clone()))
        .collect();
    let inputs = cx(project, disk).named(subject, &named);
    let mut sections: Vec<SectionRead> = route
        .sections
        .iter()
        .zip(inputs.reports)
        .map(|(section, report)| SectionRead {
            name: report.name,
            tier: section.tier,
            selector: report.selector,
            kind: match report.kind {
                "files" => SectionKind::Files,
                "tests" => SectionKind::Tests,
                _ => SectionKind::Entities,
            },
            members: report.members,
            files: report.files,
            settings: BTreeMap::new(),
            digest: report.digest,
        })
        .collect();
    if !route.settings.is_empty() {
        let values = project.environment().settings().of(&route.extension);
        let settings: BTreeMap<String, Value> = route
            .settings
            .iter()
            .map(|key| {
                let value = values
                    .and_then(|values| values.get(key))
                    .and_then(|setting| setting.value.clone());
                (key.clone(), value.unwrap_or(Value::Null))
            })
            .collect();
        sections.push(SectionRead {
            name: CONTEXT_SETTINGS.to_string(),
            tier: ContextTier::Core,
            selector: String::new(),
            kind: SectionKind::Settings,
            members: Vec::new(),
            files: Vec::new(),
            digest: hash(&json!({ "settings": settings })),
            settings,
        });
    }
    sections
}

/// The digest of an entity section served in part: its members `ids` (by path, in the section's order) digested as
/// the judged input of its selector over those members would be.
pub fn served(
    project: &CompiledProject,
    disk: &dyn Disk,
    section: &SectionRead,
    ids: &[String],
) -> String {
    let (set, _) = cx(project, disk).entity_set(&section.selector, ids.iter().cloned());
    hash(&set)
}

/// The digest of a pack of `route` for `subject` over the sections it served, each as `(name, selector, digest)` in
/// the pack's order: what was left out moves nothing, what was served moves it.
pub fn digest(route: &str, subject: &str, served: &[(String, String, String)]) -> String {
    let sections: Vec<Value> = served
        .iter()
        .map(|(name, selector, digest)| json!([name, selector, digest]))
        .collect();
    hash(&json!({"route": route, "subject": subject, "sections": sections}))
}
