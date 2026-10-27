//! What one written reference is (ADR 0117): decided once, here, against the graph's nodes and its
//! field's [`Target`]. The linker makes the edge when the decision says so and reports what the
//! decision names.

use std::collections::BTreeMap;

use specforge_common::{Diagnostic, DiagnosticData, Sym, codes, find_close_match};

use crate::field_form::Reference;
use crate::scope::Found;
use crate::{Graph, Node, Target};

/// What one written reference is.
pub(super) enum Resolution<'g> {
    /// It names an entity its field takes (of its target kind, or any kind when the field declares
    /// none): `path`.
    Linked { path: Sym },
    /// It names an entity of another kind than its field's target kind: E022. `provider` only when the
    /// target is absent.
    Mistyped {
        path: Sym,
        target: Target<'g>,
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
    /// It names no entity, and no loaded extension declares its field's target kind: I004.
    /// `provider` is the extension that declares it, where one is known.
    Unavailable {
        kind: &'g str,
        provider: Option<&'g str>,
    },
}

impl Graph {
    /// The resolution of `id`, written in a field whose target is `target`; `owners` is
    /// `GraphInputs::target_owners` (an absent kind's provider). `from` is the path of the entity holding it:
    /// its scope chain answers (ADR 0284). `None` for a scope field, which has no chain yet.
    pub(super) fn resolve<'g>(
        &'g self,
        target: Option<Target<'g>>,
        owners: &'g BTreeMap<String, String>,
        from: Option<Sym>,
        id: &str,
    ) -> Resolution<'g> {
        let admits = |kind: &str| target.is_none_or(|t| t.admits(kind));
        let found = self.scopes.resolve(from, id, &admits);
        let named = |path: Sym| self.nodes[&path].kind.raw.as_str();
        match found {
            Found::One(path) => return Resolution::Linked { path },
            Found::Mistyped(path) => {
                let target = target.expect("a reference of any kind is never mistyped");
                return Resolution::Mistyped {
                    path,
                    target,
                    provider: match target {
                        Target::Absent(kind) => owners.get(kind).map(String::as_str),
                        Target::Loaded(_) => None,
                    },
                    actual: named(path),
                    did_you_mean: self.close_match(
                        from,
                        id,
                        self.nodes_by_kind(target.kind()).into_iter(),
                    ),
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
        match target {
            Some(Target::Absent(kind)) => Resolution::Unavailable {
                kind,
                provider: owners.get(kind).map(String::as_str),
            },
            Some(Target::Loaded(_)) | None => {
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
                let expected = target.kind();
                Some(
                    Diagnostic::new(
                        codes::E022,
                        format!(
                            "reference '{}' in field '{}' of {} '{}' targets a {}, but this field expects {}",
                            reference.id, field, node.kind.raw, node.id.raw, actual, expected
                        ),
                    )
                    .with_span(span)
                    .with_suggestion(match (target, did_you_mean, provider) {
                        (Target::Loaded(_), Some(close), _) => format!("did you mean '{close}'?"),
                        (Target::Loaded(_), None, _) => {
                            format!("name a {expected} here, or drop the reference")
                        }
                        (Target::Absent(_), _, Some(extension)) => format!(
                            "name a {expected} here (enable the extension that declares it with `{}`), or drop the reference",
                            specforge_common::remedy::command("specforge add {}", &[extension])
                        ),
                        (Target::Absent(_), _, None) => format!(
                            "name a {expected} here (find an extension that declares it with `{}`), or drop the reference",
                            specforge_common::remedy::command("specforge search {}", &[expected])
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
                        format!("unresolved reference '{}' in entity '{holder}'", reference.id)
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
            Self::Unavailable { kind, provider } => Some(
                Diagnostic::new(
                    codes::I004,
                    format!(
                        "reference '{}' in field '{}' of '{}' targets kind '{}', which no enabled extension provides",
                        reference.id, field, node.id.raw, kind
                    ),
                )
                .with_span(span)
                .with_suggestion(match provider {
                    Some(extension) => format!(
                        "enable the extension that declares '{kind}' with `{}`, or drop the field",
                        specforge_common::remedy::command("specforge add {}", &[extension])
                    ),
                    None => format!(
                        "find an extension that declares '{kind}' with `{}`, or drop the field",
                        specforge_common::remedy::command("specforge search {}", &[kind])
                    ),
                }),
            ),
        }
    }
}
