//! What one written reference is (ADR 0117): decided once, here, against the graph's nodes and its
//! field's [`Target`]. The linker makes the edge when the decision says so and reports what the
//! decision names.

use std::collections::BTreeMap;

use specforge_common::{Diagnostic, DiagnosticData, Sym, codes, find_close_match};

use crate::field_form::Reference;
use crate::scope::Found;
use crate::{Graph, Node, Targets};

/// What one written reference is.
pub(super) enum Resolution<'g> {
    /// It names an entity its field takes (of its target kind, or any kind when the field declares
    /// none): `path`.
    Linked { path: Sym },
    /// It names an entity of another kind than its field's target kind: E022. `provider` only when the
    /// target is absent.
    Mistyped {
        path: Sym,
        target: Targets<'g>,
        provider: Option<&'g str>,
        actual: &'g str,
        did_you_mean: Option<String>,
    },
    /// It names no entity, and its field's target kind is loaded or there is none: E003. `candidates` are the
    /// entities that bear the written name outside the holder's scope chain (or, `ambiguous`, all that a short form
    /// matches), each spelled as the holder writes it.
    Unresolved {
        did_you_mean: Option<String>,
        candidates: Vec<String>,
        ambiguous: bool,
    },
    /// It names no entity, and no loaded extension declares a target kind of its field: I004. `kinds` are the target
    /// kinds no loaded extension declares, each with the extension that declares it where one is known.
    Unavailable {
        kinds: Vec<(&'g str, Option<&'g str>)>,
    },
}

impl Graph {
    /// The resolution of `id`, written in a field whose target is `target`; `owners` is
    /// `GraphInputs::target_owners` (an absent kind's provider). `from` is the path of the entity holding it:
    /// its scope chain answers (ADR 0284). `None` for a scope field, which has no chain yet.
    pub(super) fn resolve<'g>(
        &'g self,
        target: Option<Targets<'g>>,
        owners: &'g BTreeMap<String, String>,
        from: Option<Sym>,
        id: &str,
    ) -> Resolution<'g> {
        let admits = |kind: &str| target.as_ref().is_none_or(|t| t.admits(kind));
        let found = self.scopes.resolve(from, id, &admits);
        let named = |path: Sym| self.nodes[&path].kind.raw.as_str();
        match found {
            Found::One(path) => return Resolution::Linked { path },
            Found::Mistyped(path) => {
                let target = target.expect("a reference of any kind is never mistyped");
                let absent = target.absent();
                return Resolution::Mistyped {
                    path,
                    provider: absent
                        .first()
                        .and_then(|kind| owners.get(*kind))
                        .map(String::as_str),
                    actual: named(path),
                    did_you_mean: self.close_match(
                        from,
                        id,
                        target
                            .loaded()
                            .into_iter()
                            .flat_map(|kind| self.nodes_by_kind(kind))
                            .collect::<Vec<_>>()
                            .into_iter(),
                    ),
                    target,
                };
            }
            Found::Ambiguous(paths) => {
                return Resolution::Unresolved {
                    did_you_mean: None,
                    candidates: self.spelled(from, &paths, &admits),
                    ambiguous: true,
                };
            }
            Found::None(_) => {}
        }
        match target.as_ref().filter(|t| !t.absent().is_empty()) {
            Some(target) => Resolution::Unavailable {
                kinds: target
                    .absent()
                    .into_iter()
                    .map(|kind| (kind, owners.get(kind).map(String::as_str)))
                    .collect(),
            },
            None => {
                let Found::None(paths) = found else {
                    unreachable!("the other outcomes returned")
                };
                Resolution::Unresolved {
                    did_you_mean: self.close_match(from, id, self.nodes.values()),
                    candidates: self.spelled(from, &paths, &admits),
                    ambiguous: false,
                }
            }
        }
    }

    /// `paths`, each as `from` writes it.
    fn spelled(
        &self,
        from: Option<Sym>,
        paths: &[Sym],
        admits: &dyn Fn(&str) -> bool,
    ) -> Vec<String> {
        paths
            .iter()
            .map(|path| self.scopes.spell(from, *path, admits))
            .collect()
    }

    /// The closest name among `nodes` to the last name `id` writes, as `from` writes that entity.
    fn close_match<'n>(
        &self,
        from: Option<Sym>,
        id: &str,
        nodes: impl Iterator<Item = &'n Node>,
    ) -> Option<String> {
        let nodes: Vec<&Node> = nodes.collect();
        let written = id.rsplit('/').next().unwrap_or(id);
        let close = find_close_match(written, nodes.iter().map(|n| n.name()))?;
        let node = nodes.iter().find(|n| n.name() == close)?;
        Some(self.scopes.spell(from, node.id.raw, &|_| true))
    }
}

impl Resolution<'_> {
    /// The entity it names, when it links: the edge's target.
    pub(super) fn path(&self) -> Option<Sym> {
        match self {
            Self::Linked { path } | Self::Mistyped { path, .. } => Some(*path),
            _ => None,
        }
    }

    /// Its diagnostic, if any, at the reference's span (its ID, inside any quotes), else the entity's (the one span
    /// policy).
    pub(super) fn diagnostic(
        &self,
        node: &Node,
        field: Sym,
        reference: &Reference<'_>,
    ) -> Option<Diagnostic> {
        let span = reference
            .span
            .clone()
            .unwrap_or_else(|| node.source_span.clone());
        match self {
            Self::Linked { .. } => None,
            Self::Mistyped {
                target,
                provider,
                actual,
                did_you_mean,
                ..
            } => {
                let expected = target.text();
                let expected = expected.as_str();
                Some(
                    Diagnostic::new(
                        codes::E022,
                        format!(
                            "reference '{}' in field '{}' of {} '{}' targets a {}, but this field expects {}",
                            reference.id, field, node.kind.raw, node.id.raw, actual, expected
                        ),
                    )
                    .with_span(span)
                    .with_suggestion(match (target.absent().first(), did_you_mean, provider) {
                        (None, Some(close), _) => format!("did you mean '{close}'?"),
                        (None, None, _) => {
                            format!("name a {expected} here, or drop the reference")
                        }
                        (Some(_), _, Some(extension)) => format!(
                            "name a {expected} here (enable the extension that declares it with `{}`), or drop the reference",
                            specforge_common::remedy::command("specforge add {}", &[extension])
                        ),
                        (Some(kind), _, None) => format!(
                            "name a {expected} here (find an extension that declares it with `{}`), or drop the reference",
                            specforge_common::remedy::command("specforge search {}", &[kind])
                        ),
                    }),
                )
            }
            Self::Unresolved {
                did_you_mean,
                candidates,
                ambiguous,
            } => {
                let holder = node.id.raw;
                let message = match (ambiguous, candidates.is_empty()) {
                    (true, _) => format!(
                        "ambiguous reference '{}' in entity '{holder}': write one of {}",
                        reference.id,
                        candidates.join(", ")
                    ),
                    (false, false) => format!(
                        "unresolved reference '{}' in entity '{holder}': not in its scope chain; write one of {}",
                        reference.id,
                        candidates.join(", ")
                    ),
                    (false, true) => {
                        format!(
                            "unresolved reference '{}' in entity '{holder}'",
                            reference.id
                        )
                    }
                };
                let mut diagnostic = Diagnostic::new(codes::E003, message)
                    .with_span(span)
                    .with_data(DiagnosticData::UnresolvedReference {
                        target: reference.id.to_string(),
                        entity: holder.as_str().to_string(),
                        field: field.as_str().to_string(),
                        did_you_mean: did_you_mean.clone(),
                        candidates: candidates.clone(),
                    });
                if let Some(close) = did_you_mean {
                    diagnostic = diagnostic.with_suggestion(format!("did you mean '{close}'?"));
                } else if let [only] = candidates.as_slice() {
                    diagnostic = diagnostic.with_suggestion(format!("write '{only}'"));
                }
                Some(diagnostic)
            }
            Self::Unavailable { kinds } => {
                let named = kinds
                    .iter()
                    .map(|(kind, _)| format!("'{kind}'"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let plural = if kinds.len() == 1 { "kind" } else { "kinds" };
                let remedy = |(kind, provider): &(&str, Option<&str>)| match provider {
                    Some(extension) => format!(
                        "enable the extension that declares '{kind}' with `{}`",
                        specforge_common::remedy::command("specforge add {}", &[extension])
                    ),
                    None => format!(
                        "find an extension that declares '{kind}' with `{}`",
                        specforge_common::remedy::command("specforge search {}", &[kind])
                    ),
                };
                Some(
                    Diagnostic::new(
                        codes::I004,
                        format!(
                            "reference '{}' in field '{}' of '{}' targets {plural} {named}, which no enabled extension provides",
                            reference.id, field, node.id.raw
                        ),
                    )
                    .with_span(span)
                    .with_suggestion(format!(
                        "{}, or drop the field",
                        kinds.iter().map(remedy).collect::<Vec<_>>().join(", and ")
                    )),
                )
            }
        }
    }
}
