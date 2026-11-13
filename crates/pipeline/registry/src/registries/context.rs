//! The context routes a role reads (ADR 0356), each checked once against the loaded kinds, fields, agents and settings:
//! a route that cannot be served is E123 and is not registered; an agent binding that cannot hold is E124 and is
//! dropped, the route kept. A section that reaches a kind of an optional peer the project does not load is skipped and
//! recorded, not reported; a subject kind of one drops the route. A route that records its pack names, for each
//! `context` field of its extension it is recorded in, the chain to its subject, and the code a stale record raises.

use std::collections::{BTreeMap, BTreeSet};

use specforge_common::{Diagnostic, codes};
use specforge_diagnostics::{Codebook, qualify};
use specforge_protocol_types::{
    CONTEXT_SETTINGS, ContextDescriptor, ContextTier, ExtensionDeclaration, FieldWriter,
    is_valid_short,
};

use super::SettingsRegistry;
use crate::compilation::UnloadedPeers;
use crate::judged::{Chain, Head, Reader, Selector, check_paths, check_selector, is_word, walk};
use crate::rules::Registries;

/// One registered section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextSection {
    pub name: String,
    pub tier: ContextTier,
    pub selector: Selector,
}

/// A section dropped because it reaches `kind`, a kind of an optional peer the project does not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedSection {
    pub section: String,
    pub kind: String,
}

/// A field a route's pack is recorded in (ADR 0356 D10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextRecord {
    pub kind: String,
    pub field: String,
    /// From the recorded entity (`self`) to the route's subject.
    pub subject: Chain,
    /// What a stale record's finding suggests, `{entity}`, `{subject}` and `{route}` to fill in.
    pub suggestion: Option<String>,
}

/// A registered context route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextRoute {
    /// Qualified by its extension's short name: `gsd/executor`.
    pub name: String,
    pub extension: String,
    pub description: Option<String>,
    pub subject: String,
    /// The extension's own agent it is bound to; `None` when it names none or its binding was refused (E124).
    pub agent: Option<String>,
    pub budget: u32,
    /// The registered sections, in declared order.
    pub sections: Vec<ContextSection>,
    /// Settings of its extension whose values a pack carries, as declared.
    pub settings: Vec<String>,
    pub skipped: Vec<SkippedSection>,
    /// The fields its pack is recorded in.
    pub records: Vec<ContextRecord>,
    /// The qualified code a stale record raises (`gsd/W524`): present exactly when `records` is not empty.
    pub stale: Option<String>,
}

impl ContextRoute {
    /// The record of `kind.field`, when this route records one.
    pub fn record(&self, kind: &str, field: &str) -> Option<&ContextRecord> {
        self.records
            .iter()
            .find(|record| record.kind == kind && record.field == field)
    }
}

/// Every registered context route, and the route each agent is bound to.
#[derive(Debug, Clone, Default)]
pub struct ContextRegistry {
    routes: Vec<ContextRoute>,
    by_agent: BTreeMap<String, usize>,
}

impl ContextRegistry {
    /// Every route, in load order.
    pub fn iter(&self) -> impl Iterator<Item = &ContextRoute> {
        self.routes.iter()
    }

    /// The route of the qualified `name`.
    pub fn named(&self, name: &str) -> Option<&ContextRoute> {
        self.routes.iter().find(|route| route.name == name)
    }

    /// The route bound to `agent`, when one is.
    pub fn of_agent(&self, agent: &str) -> Option<&ContextRoute> {
        self.by_agent.get(agent).map(|&at| &self.routes[at])
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// The routes that record their pack in `kind.field`, in load order.
    pub fn recording<'a>(
        &'a self,
        kind: &'a str,
        field: &'a str,
    ) -> impl Iterator<Item = &'a ContextRoute> + 'a {
        self.routes
            .iter()
            .filter(move |route| route.record(kind, field).is_some())
    }
}

/// The context routes `declarations` (in load order) declare, checked against `registries` and the settings their
/// extensions declare: E123 for each route that cannot be served, E124 for each agent binding that cannot hold.
pub(crate) fn build(
    declarations: &[ExtensionDeclaration],
    registries: Registries<'_>,
    settings: &SettingsRegistry,
    book: &Codebook,
    candidates: &crate::CandidateVocabulary,
) -> (ContextRegistry, Vec<Diagnostic>) {
    let loaded: Vec<&str> = declarations.iter().map(|d| d.name()).collect();
    let mut registry = ContextRegistry::default();
    let mut diagnostics = Vec::new();
    for declaration in declarations {
        let mut names = BTreeSet::new();
        let check = Check {
            declaration,
            registries,
            settings,
            book,
            unloaded: UnloadedPeers::of(declaration, &loaded, candidates),
        };
        for descriptor in &declaration.contexts {
            let qualified = format!("{}/{}", declaration.short(), descriptor.name);
            let checked = if !names.insert(descriptor.name.as_str()) {
                Err("a context route's name is unique in its extension".to_string())
            } else if registry.named(&qualified).is_some() {
                Err(format!(
                    "'{qualified}' is the name of an earlier extension's context route"
                ))
            } else {
                check.route(descriptor, qualified)
            };
            let mut route = match checked {
                Ok(Some(route)) => route,
                Ok(None) => continue,
                Err(why) => {
                    diagnostics.push(
                        Diagnostic::new(
                            codes::E123,
                            format!(
                                "extension '{}': context route '{}': {why} — the route was not registered",
                                declaration.name(),
                                descriptor.name
                            ),
                        )
                        .with_suggestion(format!(
                            "fix the context route's declaration in {}",
                            declaration.name()
                        )),
                    );
                    continue;
                }
            };
            if let Some(agent) = route.agent.clone() {
                let why = if !declaration.surfaces.agents.iter().any(|a| a.id == agent) {
                    Some(format!("the extension declares no agent '{agent}'"))
                } else {
                    registry.of_agent(&agent).map(|earlier| {
                        format!(
                            "agent '{agent}' already reads the context route '{}'",
                            earlier.name
                        )
                    })
                };
                match why {
                    Some(why) => {
                        diagnostics.push(
                            Diagnostic::new(
                                codes::E124,
                                format!(
                                    "extension '{}': context route '{}': {why} — the route was registered \
                                     without its agent",
                                    declaration.name(),
                                    route.name
                                ),
                            )
                            .with_suggestion(format!(
                                "name an agent of {} that no other context route names",
                                declaration.name()
                            )),
                        );
                        route.agent = None;
                    }
                    None => {
                        registry.by_agent.insert(agent, registry.routes.len());
                    }
                }
            }
            registry.routes.push(route);
        }
    }
    (registry, diagnostics)
}

/// What one extension's routes are checked against.
struct Check<'a> {
    declaration: &'a ExtensionDeclaration,
    registries: Registries<'a>,
    settings: &'a SettingsRegistry,
    book: &'a Codebook,
    /// The optional peers of its extension the project does not load: a section that reaches a kind of theirs is
    /// skipped, and a section that reaches any other kind no loaded extension declares is E123.
    unloaded: UnloadedPeers<'a>,
}

impl Check<'_> {
    /// The route registered under `qualified`; `None` when its subject kind is an unloaded optional peer's; `Err`
    /// the reason it cannot be served (E123).
    fn route(
        &self,
        d: &ContextDescriptor,
        qualified: String,
    ) -> Result<Option<ContextRoute>, String> {
        if !is_valid_short(&d.name) {
            return Err("a context route's name is kebab case".into());
        }
        if d.budget == 0 {
            return Err("a context route's budget is a positive number of tokens".into());
        }
        if !d.sections.iter().any(|s| s.tier == ContextTier::Core) {
            return Err("a context route has at least one core section".into());
        }
        if !self.registries.kinds.contains(&d.subject) {
            return match self.unloaded.declare(&d.subject) {
                true => Ok(None),
                false => Err(format!("subject kind '{}' is not loaded", d.subject)),
            };
        }
        let own = self.settings.of(self.declaration.name());
        if let Some(setting) = d
            .settings
            .iter()
            .find(|key| own.and_then(|s| s.find(key)).is_none())
        {
            return Err(format!(
                "'{setting}' names no setting of {}",
                self.declaration.name()
            ));
        }
        let subject = BTreeSet::from([d.subject.clone()]);
        let mut sections = Vec::new();
        let mut skipped = Vec::new();
        let mut names: Vec<&str> = Vec::new();
        for section in &d.sections {
            let name = section.name.as_str();
            if !is_word(name) || name == CONTEXT_SETTINGS || names.contains(&name) {
                return Err(format!(
                    "section '{name}': a section's name is a word, unique in the route, and not \
                     '{CONTEXT_SETTINGS}'"
                ));
            }
            names.push(name);
            let at =
                |why: String| format!("section '{name}': selector '{}': {why}", section.selector);
            let selector = Selector::parse(&section.selector).map_err(at)?;
            check_paths(&selector).map_err(at)?;
            let inert = check_selector(
                &selector,
                &subject,
                self.registries,
                Reader::Context,
                |kind| self.unloaded.declare(kind),
            )
            .map_err(|e| at(e.why()))?;
            match inert {
                Some(kind) => skipped.push(SkippedSection {
                    section: name.to_string(),
                    kind,
                }),
                None => sections.push(ContextSection {
                    name: name.to_string(),
                    tier: section.tier,
                    selector,
                }),
            }
        }
        let (records, stale) = self.records(d)?;
        Ok(Some(ContextRoute {
            name: qualified,
            extension: self.declaration.name().to_string(),
            description: d.description.clone(),
            subject: d.subject.clone(),
            agent: d.agent.clone(),
            budget: d.budget,
            sections,
            settings: d.settings.clone(),
            skipped,
            records,
            stale,
        }))
    }

    /// The route's records and its qualified stale code (ADR 0356 D10), or why they cannot be held.
    fn records(
        &self,
        d: &ContextDescriptor,
    ) -> Result<(Vec<ContextRecord>, Option<String>), String> {
        let extension = self.declaration.name();
        let stale = match (&d.stale, d.records.is_empty()) {
            (None, true) => return Ok((Vec::new(), None)),
            (Some(code), false) if self.book.catalogued(extension, code) => {
                qualify(extension, &code.to_uppercase())
            }
            (Some(code), false) => {
                return Err(format!(
                    "stale code '{code}' is not in the catalogue of {extension}"
                ));
            }
            _ => return Err("a context route declares records and stale together".into()),
        };
        let mut records: Vec<ContextRecord> = Vec::new();
        for declared in &d.records {
            let at = |why: String| format!("record '{}': {why}", declared.field);
            let (kind, field) = declared
                .field
                .split_once('.')
                .filter(|(_, field)| !field.contains('.'))
                .ok_or_else(|| at("a record names 'kind.field'".into()))?;
            if !self.registries.kinds.contains(kind) {
                return Err(at(format!("kind '{kind}' is not loaded")));
            }
            let entry = self
                .registries
                .fields
                .get(kind, field)
                .ok_or_else(|| at(format!("kind '{kind}' declares no field '{field}'")))?;
            if entry.writer() != Some(FieldWriter::Context) {
                return Err(at("the field is not written 'context'".into()));
            }
            if entry.source_extension() != extension {
                return Err(at(format!(
                    "the field is declared by {}, not by {extension}",
                    entry.source_extension()
                )));
            }
            if records.iter().any(|r| r.kind == kind && r.field == field) {
                return Err(at("the route records the field twice".into()));
            }
            let subject = match Selector::parse(&declared.subject) {
                Ok(Selector::Entities(chain)) if chain.head == Head::This => chain,
                Ok(_) => {
                    return Err(at(format!(
                        "subject '{}' is not a chain from self",
                        declared.subject
                    )));
                }
                Err(why) => return Err(at(format!("subject '{}': {why}", declared.subject))),
            };
            let from = BTreeSet::from([kind.to_string()]);
            let reached = walk(
                &subject,
                subject.steps.len(),
                &from,
                self.registries,
                Reader::Context,
            )
            .map_err(|e| at(format!("subject '{}': {}", declared.subject, e.why())))?;
            if reached != BTreeSet::from([d.subject.clone()]) {
                return Err(at(format!(
                    "subject '{}' reaches {}, not the route's subject kind '{}'",
                    declared.subject,
                    reached.into_iter().collect::<Vec<_>>().join("|"),
                    d.subject
                )));
            }
            records.push(ContextRecord {
                kind: kind.to_string(),
                field: field.to_string(),
                subject,
                suggestion: declared.suggestion.clone(),
            });
        }
        Ok((records, Some(stale)))
    }
}
