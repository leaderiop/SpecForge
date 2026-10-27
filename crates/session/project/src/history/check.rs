//! The history checks (ADR 0287): E084 for an immutable value that differs from the baseline, E085 for an
//! append-only list the baseline is not a prefix of, or an entity of an append-only kind that is gone. An entity is
//! judged against the baseline entity of the same (translated) path and kind; one the baseline lacks is new, a
//! computed value is not judged, and a field the baseline leaves unset may be set once.

use serde_json::Value;
use specforge_common::{Diagnostic, codes};
use specforge_graph::wire::resolved;
use specforge_graph::{FieldForm, FieldForms, Graph, Reading};
use specforge_registry::RegistryBuild;

use crate::baseline::{Baseline, BaselineEntity};

/// The diagnostics of `graph` judged against `baseline` (already translated through the ledger's moves).
pub fn check(
    baseline: &Baseline,
    graph: &Graph,
    forms: &FieldForms,
    registries: &RegistryBuild,
) -> Vec<Diagnostic> {
    let head = baseline.revision.short();
    let mut out = Vec::new();
    for node in graph.nodes() {
        let (path, kind) = (node.id.raw.as_str(), node.kind.raw.as_str());
        let Some(before) = baseline.entities.get(path).filter(|b| b.kind == kind) else {
            continue;
        };
        let appends = registries
            .kinds()
            .get(kind)
            .is_some_and(|k| k.declared.append_only);
        let as_was = before
            .was
            .as_deref()
            .map(|was| format!(" (as '{was}')"))
            .unwrap_or_default();
        let mut keys: Vec<&str> = before.values.keys().map(String::as_str).collect();
        keys.extend(node.fields.entries().iter().map(|e| e.key.as_str()));
        keys.sort_unstable();
        keys.dedup();
        for key in keys {
            let base = before.values.get(key);
            let now = resolved(graph, forms, node, key).map(|v| v.as_value().clone());
            let span = node
                .fields
                .entries()
                .iter()
                .find(|e| e.key.as_str() == key)
                .and_then(|e| e.value_span.clone())
                .unwrap_or_else(|| node.source_span.clone());
            let mut found = Vec::new();
            match forms.reading(kind, key) {
                // every authored field of an append-only kind is immutable: judged whole, once
                Reading::Declared(form) if appends && !form.immutable() && !form.append_only() => {
                    judge_whole(key, base, now.as_ref(), &mut found);
                }
                Reading::Declared(form) => judge(form, key, base, now.as_ref(), &mut found),
                _ if appends => judge_whole(key, base, now.as_ref(), &mut found),
                _ => {}
            }
            for (code, what) in found {
                out.push(
                    Diagnostic::new(
                        code,
                        format!("{what} of {kind} '{path}'{as_was}, at HEAD {head}"),
                    )
                    .with_span(span.clone()),
                );
            }
        }
    }
    for (path, before) in &baseline.entities {
        let kept = graph
            .node(path)
            .is_some_and(|n| n.kind.raw.as_str() == before.kind);
        let appends = registries
            .kinds()
            .get(&before.kind)
            .is_some_and(|k| k.declared.append_only);
        if appends && !kept {
            out.push(Diagnostic::new(
                codes::E085,
                format!(
                    "{} '{path}' of an append-only kind was removed (HEAD {head})",
                    before.kind
                ),
            ));
        }
    }
    out
}

/// `field` is immutable: a set baseline value is never different (an unset one may be set once).
fn judge_whole(
    key: &str,
    base: Option<&Value>,
    now: Option<&Value>,
    out: &mut Vec<(specforge_common::Code, String)>,
) {
    if let Some(base) = base
        && now != Some(base)
    {
        out.push((codes::E084, format!("immutable field '{key}' was {base}")));
    }
}

fn judge(
    form: &FieldForm,
    key: &str,
    base: Option<&Value>,
    now: Option<&Value>,
    out: &mut Vec<(specforge_common::Code, String)>,
) {
    if form.immutable() {
        return judge_whole(key, base, now, out);
    }
    if form.append_only() {
        let empty = Vec::new();
        let was = base.and_then(Value::as_array).unwrap_or(&empty);
        let is = now.and_then(Value::as_array).unwrap_or(&empty);
        if !is.starts_with(was) {
            out.push((
                codes::E085,
                format!(
                    "append-only field '{key}' no longer extends its baseline: HEAD holds {} item(s), now {}",
                    was.len(),
                    is.len()
                ),
            ));
        }
    }
    let Some(members) = form.members() else {
        return;
    };
    // members are judged item by item, for the items both hold
    let pairs: Vec<(String, Option<&Value>, Option<&Value>)> = match (base, now) {
        (Some(Value::Array(a)), Some(Value::Array(b))) => a
            .iter()
            .zip(b)
            .enumerate()
            .flat_map(|(i, (a, b))| member_pairs(members, a, b, &format!("{key}[{i}]")))
            .collect(),
        (Some(a @ Value::Object(_)), Some(b @ Value::Object(_))) => {
            member_pairs(members, a, b, key)
        }
        _ => return,
    };
    for (path, a, b) in pairs {
        if let Some(form) = member_form(members, &path) {
            judge(form, &path, a, b, out);
        }
    }
}

fn member_pairs<'v>(
    members: &specforge_graph::field_form::BlockMembers,
    base: &'v Value,
    now: &'v Value,
    prefix: &str,
) -> Vec<(String, Option<&'v Value>, Option<&'v Value>)> {
    let (Some(a), Some(b)) = (base.as_object(), now.as_object()) else {
        return Vec::new();
    };
    let mut names: Vec<&String> = a.keys().chain(b.keys()).collect();
    names.sort();
    names.dedup();
    names
        .into_iter()
        .filter(|n| members.get(n).is_some())
        .map(|n| (format!("{prefix}.{n}"), a.get(n), b.get(n)))
        .collect()
}

/// The member form a written path `a[0].b` names.
fn member_form<'m>(
    members: &'m specforge_graph::field_form::BlockMembers,
    path: &str,
) -> Option<&'m FieldForm> {
    let name = path.rsplit('.').next()?;
    members.get(name)
}

impl BaselineEntity {
    pub(crate) fn moved_from(mut self, old: &str) -> Self {
        self.was = Some(old.to_string());
        self
    }
}
