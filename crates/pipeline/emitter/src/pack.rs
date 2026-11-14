//! A context pack (ADR 0356): one context route served for one entity, its sections fitted to a token budget in
//! declared order. The fit and the render are two calls: the caller digests what the fit served between them, so the
//! kernel has one digest scheme and the emitter hashes nothing.
//!
//! The declared strategy: the subject and every core section are the floor, never cut (a budget below them is E062
//! naming the route); enrich sections are served whole in declared order; the first that does not fit keeps its most
//! connected new entities that fit (a files, tests or settings section keeps nothing), and it and every later section
//! are listed as truncated.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;
use serde_json::value::RawValue;
use specforge_common::Sym;
use specforge_graph::{Computed, Graph};
use specforge_protocol_types::{ContextTier, JudgedFile};
use specforge_registry::RegistryBuild;

use crate::budget::fewest;
use crate::emit::{EmitFormat, Shape};
use crate::error::EmitterError;
use crate::schema::GraphProtocolSchema;
use specforge_protocol_types::tokens::estimate_tokens;

/// What a digest costs while it is not known yet: a digest's tokens are its scheme and one hex run, whatever its value.
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// What one section holds, owned.
#[derive(Debug, Clone, PartialEq)]
pub enum SectionBody {
    /// The entities its selector picks, by path.
    Entities(Vec<Sym>),
    /// The files it names, each with its digest.
    Files(Vec<JudgedFile>),
    /// The members of a `test(..)` section: they add no node.
    Tests(Vec<String>),
    /// The route's settings, by name.
    Settings(BTreeMap<String, Value>),
}

impl SectionBody {
    /// Its `kind` in the pack: `entities`, `files`, `tests` or `settings`.
    pub fn kind(&self) -> &'static str {
        match self {
            SectionBody::Entities(_) => "entities",
            SectionBody::Files(_) => "files",
            SectionBody::Tests(_) => "tests",
            SectionBody::Settings(_) => "settings",
        }
    }
}

/// One section of a pack, as read.
#[derive(Debug, Clone, PartialEq)]
pub struct PackSection {
    pub name: String,
    pub tier: ContextTier,
    /// The canonical selector text; empty for the settings section.
    pub selector: String,
    pub body: SectionBody,
    /// Its digest over everything it read.
    pub digest: String,
}

/// What a pack is fitted and rendered from.
pub struct PackPlan<'a> {
    /// The qualified route.
    pub route: String,
    pub subject: Sym,
    pub agent: Option<String>,
    /// The format of the pack's `graph` member: context, brief or graph (JSON).
    pub format: EmitFormat,
    /// The token budget.
    pub budget: usize,
    /// The sections, in declared order, the settings section last.
    pub sections: Vec<PackSection>,
    /// The sections the registry skipped, each with the unloaded kind it reaches.
    pub skipped: Vec<(String, String)>,
    pub schema: &'a GraphProtocolSchema,
    pub registries: &'a RegistryBuild,
    pub computed: Option<&'a Computed>,
}

/// What the fit served of one section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Served {
    /// Everything it read.
    Whole,
    /// Of an entity section, the members served: those an earlier section served and those the budget kept.
    Part(Vec<Sym>),
    /// Nothing.
    Cut,
}

/// What the fit served, and what it cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cut {
    /// One per section of the plan, in its order.
    pub served: Vec<Served>,
    pub truncated_sections: Vec<String>,
    /// The entities of the section served in part that no section served, by path.
    pub truncated_entities: Vec<String>,
    /// The cost of the pack the fit chose, by [`estimate_tokens`].
    pub estimated_tokens: usize,
}

/// The digests of what was served: each served section's (a part re-digested over its served members), in the plan's
/// order (an empty string for a section not served), and the pack's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    pub sections: Vec<String>,
    pub pack: String,
}

/// The members a section served under `served`: an entity section's served entities, by path.
pub fn served_members(section: &PackSection, served: &Served) -> Vec<Sym> {
    match (&section.body, served) {
        (_, Served::Cut) => Vec::new(),
        (SectionBody::Entities(_), Served::Part(ids)) => ids.clone(),
        (SectionBody::Entities(ids), Served::Whole) => ids.clone(),
        _ => Vec::new(),
    }
}

/// The plan fitted to its budget by the declared strategy.
///
/// Err: E062 when the subject and the core sections, with every enrich section listed as cut, cost more than the
/// budget.
pub fn fit(graph: &Graph, plan: &PackPlan<'_>) -> Result<Cut, EmitterError> {
    let enrich: Vec<usize> = (0..plan.sections.len())
        .filter(|&at| plan.sections[at].tier == ContextTier::Enrich)
        .collect();
    let names = |from: usize| -> Vec<String> {
        enrich[from..]
            .iter()
            .map(|&at| plan.sections[at].name.clone())
            .collect()
    };
    let mut served: Vec<Served> = plan
        .sections
        .iter()
        .map(|s| match s.tier {
            ContextTier::Core => Served::Whole,
            ContextTier::Enrich => Served::Cut,
        })
        .collect();
    let floor = cost(graph, plan, &served, &names(0), &[])?;
    if floor > plan.budget {
        return Err(EmitterError::BudgetTooSmall {
            reason: format!(
                "the token budget of {} cannot hold route `{}`'s core sections for `{}` ({floor} tokens: the \
                 envelope, the subject and its core sections); raise the budget",
                plan.budget, plan.route, plan.subject
            ),
        });
    }
    let mut cut = Cut {
        served: served.clone(),
        truncated_sections: names(0),
        truncated_entities: Vec::new(),
        estimated_tokens: floor,
    };
    for (nth, &at) in enrich.iter().enumerate() {
        served[at] = Served::Whole;
        let whole = cost(graph, plan, &served, &names(nth + 1), &[])?;
        if whole <= plan.budget {
            cut = Cut {
                served: served.clone(),
                truncated_sections: names(nth + 1),
                truncated_entities: Vec::new(),
                estimated_tokens: whole,
            };
            continue;
        }
        // The first section that does not fit: its most connected new entities that fit, and nothing after it.
        served[at] = Served::Cut;
        let truncated = names(nth);
        let SectionBody::Entities(members) = &plan.sections[at].body else {
            break;
        };
        // The subject is always in the graph: it is never new, never truncated.
        let mut before = served_entities(plan, &served);
        before.insert(plan.subject);
        let mut fresh: Vec<Sym> = members
            .iter()
            .copied()
            .filter(|id| !before.contains(id))
            .collect();
        fresh.sort_by_cached_key(|id| (Reverse(graph.degree(id.as_str()).total()), *id));
        let part = |dropped: usize| {
            let kept: BTreeSet<Sym> = fresh[..fresh.len() - dropped].iter().copied().collect();
            let ids: Vec<Sym> = members
                .iter()
                .copied()
                .filter(|id| before.contains(id) || kept.contains(id))
                .collect();
            let cut: Vec<String> = fresh[fresh.len() - dropped..]
                .iter()
                .map(|id| id.as_str().to_string())
                .collect();
            (Served::Part(ids), sorted(cut))
        };
        let mut attempt = served.clone();
        let mut costs = |dropped: usize| -> Result<usize, EmitterError> {
            let (part, cut) = part(dropped);
            attempt[at] = part;
            cost(graph, plan, &attempt, &truncated, &cut)
        };
        // Dropping one more new entity always lowers the cost; when even dropping them all does not fit (their IDs
        // listed cost more than the section's place), the section is cut whole.
        if costs(fresh.len())? > plan.budget {
            break;
        }
        let dropped = fewest(0, fresh.len(), |dropped| Ok(costs(dropped)? <= plan.budget))?;
        let (part, truncated_entities) = part(dropped);
        // Nothing kept: the section is cut whole, as when nothing fits.
        if part == Served::Part(Vec::new()) {
            break;
        }
        served[at] = part;
        let estimated_tokens = cost(graph, plan, &served, &truncated, &truncated_entities)?;
        cut = Cut {
            served,
            truncated_sections: truncated,
            truncated_entities,
            estimated_tokens,
        };
        break;
    }
    Ok(cut)
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

/// Every entity the served sections hold.
fn served_entities(plan: &PackPlan<'_>, served: &[Served]) -> BTreeSet<Sym> {
    plan.sections
        .iter()
        .zip(served)
        .flat_map(|(section, served)| served_members(section, served))
        .collect()
}

/// What the pack costs with `served`, its digests placeholders.
fn cost(
    graph: &Graph,
    plan: &PackPlan<'_>,
    served: &[Served],
    truncated_sections: &[String],
    truncated_entities: &[String],
) -> Result<usize, EmitterError> {
    let cut = Cut {
        served: served.to_vec(),
        truncated_sections: truncated_sections.to_vec(),
        truncated_entities: truncated_entities.to_vec(),
        estimated_tokens: 0,
    };
    let sealed = Sealed {
        sections: plan
            .sections
            .iter()
            .zip(served)
            .map(|(section, served)| match served {
                Served::Part(_) => PLACEHOLDER.to_string(),
                _ => section.digest.clone(),
            })
            .collect(),
        pack: PLACEHOLDER.to_string(),
    };
    Ok(estimate_tokens(&render(graph, plan, &cut, &sealed)?))
}

/// The pack document: the route, the subject, its digest, the budget and what was cut, the sections as served, the
/// sections skipped, and the export envelope of the subject and every served entity in the plan's format.
pub fn render(
    graph: &Graph,
    plan: &PackPlan<'_>,
    cut: &Cut,
    sealed: &Sealed,
) -> Result<String, EmitterError> {
    let mut nodes: BTreeSet<Sym> = served_entities(plan, &cut.served);
    nodes.insert(plan.subject);
    let shape = Shape {
        format: plan.format,
        schema: plan.schema,
        attach: None,
        fields: plan.registries.fields(),
        coverage: None,
        computed: plan.computed,
        judgements: None,
        checks: None,
    };
    let exported = shape.render(&graph.induced(nodes), None)?;
    let graph_json =
        RawValue::from_string(exported).map_err(|e| EmitterError::Serialization(e.to_string()))?;
    let sections: Vec<SectionJson<'_>> = plan
        .sections
        .iter()
        .zip(&cut.served)
        .zip(&sealed.sections)
        .filter(|((_, served), _)| **served != Served::Cut)
        .map(|((section, served), digest)| SectionJson::of(section, served, digest))
        .collect();
    let document = PackJson {
        route: &plan.route,
        subject: plan.subject.as_str(),
        agent: plan.agent.as_deref(),
        digest: &sealed.pack,
        budget: BudgetJson {
            strategy: "declared",
            budget_tokens: plan.budget,
            estimated_tokens: cut.estimated_tokens,
            truncated_sections: &cut.truncated_sections,
            truncated_entities: &cut.truncated_entities,
        },
        sections,
        skipped: plan
            .skipped
            .iter()
            .map(|(section, kind)| SkippedJson { section, kind })
            .collect(),
        graph: &graph_json,
    };
    serde_json::to_string(&document).map_err(|e| EmitterError::Serialization(e.to_string()))
}

#[derive(Serialize)]
struct PackJson<'a> {
    route: &'a str,
    subject: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<&'a str>,
    digest: &'a str,
    budget: BudgetJson<'a>,
    sections: Vec<SectionJson<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skipped: Vec<SkippedJson<'a>>,
    graph: &'a RawValue,
}

/// A pack's budget: always written, cut or not, so the budget is recorded beside every digest.
#[derive(Serialize)]
struct BudgetJson<'a> {
    strategy: &'static str,
    budget_tokens: usize,
    estimated_tokens: usize,
    truncated_sections: &'a [String],
    truncated_entities: &'a [String],
}

#[derive(Serialize)]
struct SkippedJson<'a> {
    section: &'a str,
    kind: &'a str,
}

#[derive(Serialize)]
struct SectionJson<'a> {
    name: &'a str,
    tier: &'static str,
    #[serde(skip_serializing_if = "str::is_empty")]
    selector: &'a str,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    members: Option<Vec<&'a str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<&'a [JudgedFile]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    settings: Option<&'a BTreeMap<String, Value>>,
    digest: &'a str,
}

impl<'a> SectionJson<'a> {
    fn of(section: &'a PackSection, served: &Served, digest: &'a str) -> Self {
        let (mut members, mut files, mut settings) = (None, None, None);
        match &section.body {
            SectionBody::Entities(ids) => {
                let ids = match served {
                    Served::Part(part) => part,
                    _ => ids,
                };
                members = Some(ids.iter().map(|id| id.as_str()).collect());
            }
            SectionBody::Tests(ids) => members = Some(ids.iter().map(String::as_str).collect()),
            SectionBody::Files(read) => files = Some(read.as_slice()),
            SectionBody::Settings(values) => settings = Some(values),
        }
        SectionJson {
            name: &section.name,
            tier: section.tier.as_str(),
            selector: &section.selector,
            kind: section.body.kind(),
            members,
            files,
            settings,
            digest,
        }
    }
}
