//! Validating a plan (ADR 0151): the edited project is compiled from the planned texts; an error it reports that the
//! project's checked report (`CompiledProject::checked_diagnostics`: every check run, even after an update that
//! skipped them) does not have refuses the plan (E080), and so does a graph change its edits do not declare (ADR 0198). Diagnostics are matched without their positions: neither the
//! span nor the digits of a message are in the key, since a message may name a position (E002, E078) and an inserted
//! block moves every later one. An extension's finding is also keyed by its subject entity, relocated through the
//! plan's moves, so a finding on another entity is another error (ADR 0302). A judged rule's finding is never such an
//! error (ADR 0297): its judge settles it, not the edit; only a finding the judged evaluation made is one.

use super::EditField;
use super::declared::Declared;
use super::steps::relocated;
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind, Refusal};
use specforge_common::{Diagnostic, DiagnosticList, Severity, codes};
use specforge_graph::{GraphDelta, compute_graph_delta_over};
use specforge_project::CompiledProject;
use specforge_registry::RegistryBuild;
use std::collections::{BTreeMap, HashMap};

type Key = (String, Severity, String, String, String);

/// Old entity path prefix → new, as a plan moves entities.
pub(crate) type Moves = [(String, String)];

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

/// The key of `d`; an extension's finding adds its subject, which `moved` relocates (the before side's).
fn key(d: &Diagnostic, moved: &Moves) -> Key {
    let subject = if d.origin().is_some() {
        d.subject()
            .map_or_else(String::new, |s| relocated(s, moved))
    } else {
        String::new()
    };
    (
        d.code.clone(),
        d.severity,
        shape(&d.message),
        d.span.as_ref().map_or("", |s| s.file.as_str()).to_string(),
        subject,
    )
}

/// Whether a diagnostic is a judged rule's finding (ADR 0297): no verdict, a stale one or a failing one, made by the
/// judged evaluation of `post`, the project the write leaves (ADR 0302). A pass or a guard that reuses the rule's code is
/// not one.
pub(crate) fn judged_finding(post: &CompiledProject) -> impl Fn(&Diagnostic) -> bool + '_ {
    |d| post.judgements().is_finding(d)
}

/// The diagnostics of `after` that `before` does not have, one by one: (errors, warnings and infos). A diagnostic
/// `judged` names is never an error here, whatever its severity. `moved` relocates the subjects of `before`.
/// `host_only`: an extension's diagnostic is never an error here either (ADR 0303).
pub(crate) fn added(
    before: &[Diagnostic],
    after: Vec<Diagnostic>,
    judged: impl Fn(&Diagnostic) -> bool,
    moved: &Moves,
    host_only: bool,
) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut had: HashMap<Key, usize> = HashMap::new();
    for d in before {
        *had.entry(key(d, moved)).or_default() += 1;
    }
    let (mut errors, mut rest) = (Vec::new(), Vec::new());
    for d in after {
        // Another writer's journal, live while this write is planned, is not a thing this write adds.
        if d.code == codes::W167.to_string() {
            continue;
        }
        match had.get_mut(&key(&d, &[])) {
            Some(n) if *n > 0 => *n -= 1,
            _ if d.severity == Severity::Error
                && !judged(&d)
                && !(host_only && d.origin().is_some()) =>
            {
                errors.push(d)
            }
            _ => rest.push(d),
        }
    }
    (errors, rest)
}

/// Refuse (E080) when `after` has an error `before` does not.
pub(crate) fn refuse_new_errors(
    before: &[Diagnostic],
    after: &CompiledProject,
    registries: &RegistryBuild,
    moved: &Moves,
    host_only: bool,
) -> Result<(), Refusal<EditField>> {
    let (errors, _) = added(
        before,
        after.diagnostics(),
        judged_finding(after),
        moved,
        host_only,
    );
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
    moved: &Moves,
    host_only: bool,
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
        judged_finding(&post),
        moved,
        host_only,
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
        let (errors, rest) = added(&[], vec![pending], |_| false, &[], false);
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
        let (errors, rest) = added(
            &[],
            vec![error("gsd/E501"), error("gsd/E999")],
            judged,
            &[],
            false,
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "gsd/E999");
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].code, "gsd/E501");
        // Already reported, it is paired and reported once, as an error of the project's own.
        let (errors, rest) = added(
            &[error("gsd/E501")],
            vec![error("gsd/E501")],
            judged,
            &[],
            false,
        );
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
        let judged = judged_finding(&compiled);
        // A truth with no verdict: the evaluation's own finding.
        let own = compiled
            .diagnostics()
            .into_iter()
            .find(|d| d.code == "gsd/E501")
            .expect("a judged finding");
        assert!(judged(&own));
        let mut other = own.clone();
        other.origin = Some("@other/x".into());
        assert!(!judged(&other));
        let mut host = own.clone();
        host.origin = None;
        assert!(!judged(&host));
    }

    #[specforge_test(
        behavior = "validate_typed_edit",
        verify = "an extension's finding is paired by its subject, through the plan's moves"
    )]
    fn a_pass_finding_on_another_entity_is_an_added_error() {
        let before = [finding("v1.0/01/01-01/TR1")];
        let (errors, _) = added(
            &before,
            vec![finding("v1.0/01/01-01/TR2")],
            |_| false,
            &[],
            false,
        );
        assert_eq!(errors.len(), 1);
    }

    /// A guard, not red-first: it holds before and after, and fails if the relocation is dropped.
    #[specforge_test(
        behavior = "validate_typed_edit",
        verify = "an extension's finding is paired by its subject, through the plan's moves"
    )]
    fn a_moved_entitys_finding_is_still_paired() {
        let before = [finding("v1.0/01/01-01/TR1")];
        let moves = [("v1.0/01".to_string(), "v1.0/02".to_string())];
        let after = vec![finding("v1.0/02/01-01/TR1")];
        assert!(
            added(&before, after.clone(), |_| false, &moves, false)
                .0
                .is_empty()
        );
        assert_eq!(added(&before, after, |_| false, &[], false).0.len(), 1);
    }

    fn finding(entity: &str) -> Diagnostic {
        Diagnostic::from_extension(
            "@gsd/core",
            "gsd/E502",
            Severity::Error,
            format!("explicit truth '{entity}' is verified, but a test recorded for it failed"),
        )
        .with_data(specforge_common::DiagnosticData::Subject {
            entity: entity.to_string(),
        })
    }

    #[specforge_test(
        behavior = "answer_edit_plan",
        verify = "a collector's plan is judged for host errors only"
    )]
    fn host_only_ignores_an_extensions_error_and_keeps_a_host_one() {
        let host = Diagnostic::untyped("E003", Severity::Error, "unresolved");
        let ext = finding("v1.0/01/01-01/TR1");
        let (errors, _) = added(&[], vec![host, ext.clone()], |_| false, &[], true);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "E003");
        assert_eq!(added(&[], vec![ext], |_| false, &[], false).0.len(), 1);
    }

    #[specforge_test(
        behavior = "validate_typed_edit",
        verify = "only a finding the judged rules made is exempt"
    )]
    fn a_finding_the_judged_evaluation_did_not_make_is_counted() {
        use specforge_project::testing::{gsd_judged_project, gsd_judged_runtime};
        use std::sync::Arc;
        let compiled = gsd_judged_project("").compile_in(Arc::new(gsd_judged_runtime()));
        let owner = compiled
            .environment()
            .registries()
            .judged_rule("gsd/E501")
            .unwrap()
            .extension
            .clone();
        // The owning extension's code and origin, but a guard's text: the evaluation did not make it.
        let reused = Diagnostic::from_extension(
            owner,
            "gsd/E501",
            Severity::Error,
            "guard pass 'g' could not be evaluated: x",
        );
        assert!(!judged_finding(&compiled)(&reused));
    }
}
