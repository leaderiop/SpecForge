//! Validating a plan (ADR 0151): the edited project is compiled from the planned texts; an error it reports that the
//! project's checked report (`CompiledProject::checked_diagnostics`: every check run, even after an update that
//! skipped them) does not have refuses the plan (E080), and so does a graph change its edits do not declare (ADR 0198). Diagnostics are matched without their positions: neither the
//! span nor the digits of a message are in the key, since a message may name a position (E002, E078) and an inserted
//! block moves every later one. A judged rule's finding is never such an error (ADR 0297): its judge settles it, not
//! the edit.

use super::EditField;
use super::declared::Declared;
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind, Refusal};
use specforge_common::{Diagnostic, DiagnosticList, Severity, codes};
use specforge_graph::{GraphDelta, compute_graph_delta_over};
use specforge_registry::RegistryBuild;
use std::collections::{BTreeMap, HashMap};

type Key = (String, Severity, String, String);

/// The message with each run of ASCII digits collapsed, so a line that moved is the same diagnostic.
fn shape(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    for c in message.chars() {
        if !c.is_ascii_digit() || !out.ends_with('#') {
            out.push(if c.is_ascii_digit() { '#' } else { c });
        }
    }
    out
}

fn key(d: &Diagnostic) -> Key {
    (
        d.code.clone(),
        d.severity,
        shape(&d.message),
        d.span.as_ref().map_or("", |s| s.file.as_str()).to_string(),
    )
}

/// Whether a diagnostic is a judged rule's finding (ADR 0297): no verdict, a stale one or a failing one. It carries
/// the rule's code and names the extension that owns the rule as its origin (the host's own and another extension's
/// diagnostics with that text are not).
pub(crate) fn judged_finding(registries: &RegistryBuild) -> impl Fn(&Diagnostic) -> bool + '_ {
    |d| {
        registries
            .judged_rule(&d.code)
            .is_some_and(|rule| d.origin() == Some(rule.extension.as_str()))
    }
}

/// The diagnostics of `after` that `before` does not have, one by one: (errors, warnings and infos). A diagnostic
/// `judged` names is never an error here, whatever its severity.
pub(crate) fn added(
    before: &[Diagnostic],
    after: Vec<Diagnostic>,
    judged: impl Fn(&Diagnostic) -> bool,
) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut had: HashMap<Key, usize> = HashMap::new();
    for d in before {
        *had.entry(key(d)).or_default() += 1;
    }
    let (mut errors, mut rest) = (Vec::new(), Vec::new());
    for d in after {
        // Another writer's journal, live while this write is planned, is not a thing this write adds.
        if d.code == codes::W167.to_string() {
            continue;
        }
        match had.get_mut(&key(&d)) {
            Some(n) if *n > 0 => *n -= 1,
            _ if d.severity == Severity::Error && !judged(&d) => errors.push(d),
            _ => rest.push(d),
        }
    }
    (errors, rest)
}

/// Refuse (E080) when `after` has an error `before` does not.
pub(crate) fn refuse_new_errors(
    before: &[Diagnostic],
    after: Vec<Diagnostic>,
    registries: &RegistryBuild,
) -> Result<(), Refusal<EditField>> {
    let (errors, _) = added(before, after, judged_finding(registries));
    if errors.is_empty() {
        Ok(())
    } else {
        Err(new_errors(errors, registries.codes()))
    }
}

/// What the project reports once `texts` are written that its checked report does not have now: the warnings and
/// infos; or E080 naming each new error; or, when the edited graph changes what `declared` does not name,
/// `undeclared_change` (ADR 0198) naming the remainder.
pub fn check(
    view: &ProjectView<'_>,
    texts: &BTreeMap<String, String>,
    declared: &Declared,
    ledger: Option<&[u8]>,
) -> Result<Vec<Diagnostic>, Refusal<EditField>> {
    let before = view.project().judged_diagnostics();
    // The project the write would leave, judged: the guard passes read it and the project before it (ADR 0289).
    let post = view
        .project()
        .edited_with(texts, ledger)
        .judged_against(view.project());
    let (errors, rest) = added(
        &before,
        post.diagnostics(),
        judged_finding(view.registries()),
    );
    if !errors.is_empty() {
        return Err(new_errors(errors, view.registries().codes()));
    }
    let delta = compute_graph_delta_over(
        view.project().graph(),
        post.graph(),
        texts.keys().map(String::as_str),
    );
    let beyond = declared.beyond(delta);
    if beyond.is_empty() {
        return Ok(rest);
    }
    Err(undeclared(&beyond))
}

fn new_errors(errors: Vec<Diagnostic>, codes: &specforge_common::Codebook) -> Refusal<EditField> {
    let mut message = format!(
        "the edit would leave the project with {} error{} it does not have; nothing was written",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" }
    );
    for error in &errors {
        message.push_str(&format!("\n  error[{}]: {}", error.code, error.message));
    }
    let data = serde_json::json!({ "diagnostics": DiagnosticList::new(errors, codes) });
    OpError::coded(OpErrorKind::CompilationFailed, codes::E080, message)
        .with_data(data)
        .into()
}

/// The change a typed edit planned and did not declare: a defect of the edit, refused before anything is written.
pub const UNDECLARED_CHANGE: &str = "undeclared_change";

fn undeclared(beyond: &GraphDelta) -> Refusal<EditField> {
    let mut message = String::from(
        "the edit would change what it does not ask for; nothing was written (a defect of the typed edit: please report it)",
    );
    for n in beyond.added_nodes.iter().chain(&beyond.removed_nodes) {
        message.push_str(&format!("\n  entity '{}': added or removed", n.id));
    }
    for n in &beyond.modified_nodes {
        let what: Vec<String> = n
            .changed_fields
            .iter()
            .map(|f| format!("field {f}"))
            .chain(
                n.changed_aspects
                    .iter()
                    .map(|a| format!("{a:?}").to_lowercase()),
            )
            .collect();
        message.push_str(&format!("\n  entity '{}': {}", n.id, what.join(", ")));
    }
    for (sign, edges) in [
        ("added", &beyond.added_edges),
        ("removed", &beyond.removed_edges),
    ] {
        for e in edges {
            message.push_str(&format!(
                "\n  edge {} -[{}]-> {} {sign}",
                e.source, e.label, e.target
            ));
        }
    }
    OpError::new(OpErrorKind::CompilationFailed, UNDECLARED_CHANGE, message)
        .with_data(serde_json::json!({ "delta": beyond }))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    #[specforge_test(
        behavior = "roll_back_interrupted_write",
        verify = "a pending journal another writer holds is no error this write adds"
    )]
    fn a_pending_journal_is_not_an_error_a_write_adds() {
        let mut pending = Diagnostic::new(codes::W167, "an interrupted write is pending rollback");
        // A preset that raises W167 makes it an error of the edited project.
        pending.severity = Severity::Error;
        let (errors, rest) = added(&[], vec![pending], |_| false);
        assert!(errors.is_empty() && rest.is_empty());
    }

    #[specforge_test(
        behavior = "validate_typed_edit",
        verify = "a judged rules finding is no error a write adds"
    )]
    fn a_judged_finding_is_not_an_error_a_write_adds() {
        let error = |code: &str| {
            let mut d = Diagnostic::new(codes::E080, "x");
            d.code = code.to_string();
            d.severity = Severity::Error;
            d
        };
        let judged = |d: &Diagnostic| d.code == "gsd/E501";
        let (errors, rest) = added(&[], vec![error("gsd/E501"), error("gsd/E999")], judged);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "gsd/E999");
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].code, "gsd/E501");
        // Already reported, it is paired and reported once, as an error of the project's own.
        let (errors, rest) = added(&[error("gsd/E501")], vec![error("gsd/E501")], judged);
        assert!(errors.is_empty() && rest.is_empty());
    }

    #[specforge_test(
        behavior = "validate_typed_edit",
        verify = "a diagnostic with a judged rules code from another origin is no judged finding"
    )]
    fn only_the_owning_extensions_diagnostic_is_a_judged_finding() {
        use specforge_project::testing::{gsd_judged_project, gsd_judged_runtime};
        use std::sync::Arc;
        let compiled = gsd_judged_project("").compile_in(Arc::new(gsd_judged_runtime()));
        let registries = crate::view::ProjectView::of(&compiled).registries();
        let owner = registries
            .judged_rule("gsd/E501")
            .unwrap()
            .extension
            .clone();
        let from = |origin: Option<&str>| {
            let mut d = Diagnostic::new(codes::E080, "x");
            d.code = "gsd/E501".to_string();
            if let Some(origin) = origin {
                d = Diagnostic::from_extension(origin, "gsd/E501", Severity::Error, "x");
            }
            d
        };
        let judged = judged_finding(registries);
        assert!(judged(&from(Some(&owner))));
        assert!(!judged(&from(Some("@other/x"))));
        assert!(!judged(&from(None)));
    }
}
