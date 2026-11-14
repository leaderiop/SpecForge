//! A context route read over a project (ADR 0356): each section of the route evaluated from one subject through the
//! judged inputs' evaluator, so a section digests exactly as a judged input of the same selector does; the route's
//! settings as one last section; the pack fitted to a budget and digested over its subject and the sections it
//! served; and the records a route keeps of its pack, checked on each check ([`records`]).

pub(crate) mod records;

use std::collections::BTreeMap;

use serde_json::{Value, json};
use specforge_common::Sym;
use specforge_emitter::pack::{self, Cut, PackPlan, PackSection, Sealed, SectionBody, Served};
use specforge_emitter::{EmitFormat, EmitterError, GraphProtocolSchema};
use specforge_graph::{Computed, Graph};
use specforge_protocol_types::{CONTEXT_SETTINGS, ContextTier, JudgedFile};
use specforge_registry::{ContextRoute, Disk, EntitySnapshot};

use crate::judged::{Cx, DigestMemo, hash};
use crate::{CheckedValues, CompiledProject, Environment};

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

/// The sections of `route` read from `subject`, in declared order, the settings section (core, named `settings`) last
/// when the route carries settings. Files are read through `disk`.
pub fn read(
    project: &CompiledProject,
    disk: &dyn Disk,
    route: &ContextRoute,
    subject: &str,
) -> Vec<SectionRead> {
    read_over(&Over::of(project, disk), route, subject)
}

/// [`read`] over `over`.
fn read_over(over: &Over<'_>, route: &ContextRoute, subject: &str) -> Vec<SectionRead> {
    let named: Vec<_> = route
        .sections
        .iter()
        .map(|section| (section.name.clone(), section.selector.clone()))
        .collect();
    let inputs = over.cx().named(subject, &named);
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
        let values = over.env.settings().of(&route.extension);
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
    let (set, _) = Over::of(project, disk)
        .cx()
        .entity_set(&section.selector, ids.iter().cloned());
    hash(&set)
}

/// The digest of `subject` as a pack shows it: the judged digest of the one-member entity set, its checked values
/// with it, so a pack whose sections never serve the subject still moves when the subject does.
pub fn subject(project: &CompiledProject, disk: &dyn Disk, subject: &str) -> String {
    let (set, _) = Over::of(project, disk)
        .cx()
        .entity_set("", [subject.to_string()]);
    hash(&set)
}

/// The digest of a pack of `route` for `subject` (whose own digest, by [`subject`], is `entity`) over the sections it
/// served, each as `(name, selector, digest)` in the pack's order: what was left out moves nothing, what was served
/// moves it, and the subject, always served, always moves it.
pub fn digest(
    route: &str,
    subject: &str,
    entity: &str,
    served: &[(String, String, String)],
) -> String {
    let sections: Vec<Value> = served
        .iter()
        .map(|(name, selector, digest)| json!([name, selector, digest]))
        .collect();
    hash(&json!({"route": route, "subject": subject, "entity": entity, "sections": sections}))
}

/// What a pack is read over: a project's environment, graph and snapshot, and what its checks found. A compiled
/// project gives one ([`Over::of`]); a check gives one over the snapshot it has just merged.
pub(crate) struct Over<'a> {
    pub env: &'a Environment,
    pub graph: &'a Graph,
    pub entities: &'a EntitySnapshot,
    pub disk: &'a dyn Disk,
    pub digests: &'a DigestMemo,
    pub checked: &'a CheckedValues,
    pub computed: &'a Computed,
}

impl<'a> Over<'a> {
    pub(crate) fn of(project: &'a CompiledProject, disk: &'a dyn Disk) -> Self {
        Self {
            env: project.environment(),
            graph: project.graph(),
            entities: project.entities(),
            disk,
            digests: project.digests(),
            checked: project.checked_values(),
            computed: project.computed(),
        }
    }

    fn cx(&self) -> Cx<'a> {
        Cx::new(
            self.env,
            self.graph,
            self.entities,
            self.disk,
            self.digests,
            self.checked,
        )
    }
}

/// A pack of a route for one subject, fitted and sealed: what [`pack::render`] renders.
pub struct Packed<'a> {
    pub plan: PackPlan<'a>,
    pub cut: Cut,
    pub sealed: Sealed,
    /// The pack's digest (`sealed.pack`).
    pub digest: String,
}

/// The pack of `route` for `subject` (by path) at `budget`, its `graph` member in `format` (the fit counts what it
/// renders) and the schema `schema` gives: its sections read, fitted, each served section digested (a part over the
/// members it kept), and the pack digested over the subject and them. Files are read through `disk`.
pub fn pack<'a>(
    project: &'a CompiledProject,
    disk: &'a dyn Disk,
    route: &'a ContextRoute,
    subject: &str,
    budget: usize,
    format: EmitFormat,
    schema: &'a GraphProtocolSchema,
) -> Result<Packed<'a>, EmitterError> {
    pack_over(
        &Over::of(project, disk),
        route,
        subject,
        budget,
        format,
        schema,
    )
}

/// [`pack`] over `over`.
pub(crate) fn pack_over<'a>(
    over: &Over<'a>,
    route: &'a ContextRoute,
    subject: &str,
    budget: usize,
    format: EmitFormat,
    schema: &'a GraphProtocolSchema,
) -> Result<Packed<'a>, EmitterError> {
    #[cfg(feature = "testing")]
    testing::count();
    let reads = read_over(over, route, subject);
    let plan = PackPlan {
        route: route.name.clone(),
        subject: Sym::new(subject),
        agent: route.agent.clone(),
        format,
        budget,
        sections: reads.iter().map(section).collect(),
        skipped: route
            .skipped
            .iter()
            .map(|s| (s.section.clone(), s.kind.clone()))
            .collect(),
        schema,
        registries: over.env.registries(),
        computed: Some(over.computed),
    };
    let cut = pack::fit(over.graph, &plan)?;
    let cx = over.cx();
    let sections: Vec<String> = reads
        .iter()
        .zip(&cut.served)
        .map(|(read, served)| match served {
            Served::Whole => read.digest.clone(),
            Served::Part(ids) => {
                let (set, _) =
                    cx.entity_set(&read.selector, ids.iter().map(|id| id.as_str().to_string()));
                hash(&set)
            }
            Served::Cut => String::new(),
        })
        .collect();
    let listed: Vec<(String, String, String)> = reads
        .iter()
        .zip(&sections)
        .filter(|(_, digest)| !digest.is_empty())
        .map(|(read, digest)| (read.name.clone(), read.selector.clone(), digest.clone()))
        .collect();
    let (set, _) = cx.entity_set("", [subject.to_string()]);
    let digest = self::digest(&route.name, subject, &hash(&set), &listed);
    let sealed = Sealed {
        sections,
        pack: digest.clone(),
    };
    Ok(Packed {
        plan,
        cut,
        sealed,
        digest,
    })
}

/// A section as read, as the emitter fits it.
fn section(read: &SectionRead) -> PackSection {
    let body = match read.kind {
        SectionKind::Entities => {
            SectionBody::Entities(read.members.iter().map(|id| Sym::new(id)).collect())
        }
        SectionKind::Files => SectionBody::Files(read.files.clone()),
        SectionKind::Tests => SectionBody::Tests(read.members.clone()),
        SectionKind::Settings => SectionBody::Settings(read.settings.clone()),
    };
    PackSection {
        name: read.name.clone(),
        tier: read.tier,
        selector: read.selector.clone(),
        body,
        digest: read.digest.clone(),
    }
}

/// How many packs this thread has evaluated (behind the `testing` feature): what a test counts a check's work by.
#[cfg(feature = "testing")]
pub mod testing {
    use std::cell::Cell;

    thread_local! {
        static PACKS: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn count() {
        PACKS.with(|packs| packs.set(packs.get() + 1));
    }

    /// The packs this thread has evaluated so far.
    pub fn packs_evaluated() -> usize {
        PACKS.with(Cell::get)
    }
}
