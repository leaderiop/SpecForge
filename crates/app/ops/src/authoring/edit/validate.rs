//! Validating a plan (ADR 0151): the edited project is compiled from the planned texts; an error it reports that the
//! project's checked report (`CompiledProject::checked_diagnostics`: every check run, even after an update that
//! skipped them) does not have refuses the plan (E080), and so does a graph change its edits do not declare (ADR 0198). Diagnostics are matched without their positions: neither the
//! span nor the digits of a message are in the key, since a message may name a position (E002, E078) and an inserted
//! block moves every later one. An extension's finding is also keyed by its subject entity, relocated through the
//! plan's moves, so a finding on another entity is another error (ADR 0302). A judged rule's finding is never such an
//! error (ADR 0297): its judge settles it, not the edit; only a finding the judged evaluation made is one. Nor is a
//! finding the write revealed by moving its entity into the owner state its code waits for (ADR 0363), nor a finding
//! a later step settles on an entity the write creates or creates something under, altering nothing the project held
//! there (ADR 0385).

use super::EditField;
use super::declared::Declared;
use super::steps::relocated;
use crate::authoring::planned::PlannedChange;
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind, Refusal};
use specforge_common::{Codebook, Diagnostic, DiagnosticList, Severity, codes};
use specforge_diagnostics::CodeRef;
use specforge_graph::{GraphDelta, compute_graph_delta_over};
use specforge_project::CompiledProject;
use specforge_registry::RegistryBuild;
use std::collections::HashMap;

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
/// judged evaluation of `post`, the project the write leaves (ADR 0302). A pass or a guard that reuses the rule's
/// code is not one.
pub(crate) fn judged_finding(post: &CompiledProject) -> impl Fn(&Diagnostic) -> bool + '_ {
    |d| post.judgements().is_finding(d)
}

/// Whether a diagnostic of `post`, the project a write leaves `before` as, is no error the write adds whatever its
/// key: a judged rule's finding ([`judged_finding`]), one the write revealed ([`revealed`]), or one a later step
/// settles on what the write creates ([`settled_later`]). `change` is the write's plan; a migration has none, so it
/// reveals and creates nothing.
pub(crate) fn not_added<'a>(
    before: &'a CompiledProject,
    post: &'a CompiledProject,
    change: Option<&'a PlannedChange>,
) -> impl Fn(&Diagnostic) -> bool + 'a {
    let judged = judged_finding(post);
    move |d| {
        judged(d)
            || change.is_some_and(|change| {
                revealed(before, post, change, d) || settled_later(post, change, d)
            })
    }
}

/// Whether `d` is a finding a later step of the authoring settles on what `change` creates (ADR 0385): its origin's
/// own catalog entry for its code declares `settled_later` (`Codebook::settled_later`), it names a subject, and the
/// change creates that subject or an entity under it and alters nothing the project held there
/// (`PlannedChange::opens`).
fn settled_later(post: &CompiledProject, change: &PlannedChange, d: &Diagnostic) -> bool {
    let (Some(extension), Some(subject)) = (d.origin(), d.subject()) else {
        return false;
    };
    change.opens(subject)
        && post
            .environment()
            .registries()
            .codes()
            .settled_later(extension, CodeRef::parse(&d.code).local)
}

/// Whether the write from `before` to `post` revealed `d` (ADR 0363): an extension's finding on an entity, whose code
/// waits for an owner state (`Codebook::waits`), and the write moved that entity from the other state into it.
/// `before` could not report it there, so the write did not add it: the owner's state moved. The entity must be the
/// one `before` held: never one the write's plan creates, deletes or moves, or is under one that it does
/// (`PlannedChange::renews`), whatever id `before` held; and both states must be known
/// (`CompiledProject::owner_pending`: none after an update that skipped the checks).
fn revealed(
    before: &CompiledProject,
    post: &CompiledProject,
    change: &PlannedChange,
    d: &Diagnostic,
) -> bool {
    let (Some(extension), Some(subject)) = (d.origin(), d.subject()) else {
        return false;
    };
    if change.renews(subject) {
        return false;
    }
    let codes = post.environment().registries().codes();
    let local = CodeRef::parse(&d.code).local;
    if !codes.waits(extension, local) {
        return false;
    }
    match (before.owner_pending(subject), post.owner_pending(subject)) {
        (Some(was), Some(is)) => {
            !codes.speaks(extension, local, was) && codes.speaks(extension, local, is)
        }
        _ => false,
    }
}

/// Which of the errors a plan adds are written rather than refused (E080): the plan records history, which an
/// extension's rule cannot unmake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lands {
    /// None: every error the plan adds refuses it (a typed edit; a command's or a hook's plan).
    #[default]
    Nothing,
    /// Every extension's error, whatever its code: a collector's plan (ADR 0303).
    Extensions,
    /// An extension's rule about authored entities, its own catalogued code (`gsd.security/E101`, `gsd/E312`): an
    /// importing command's plan (ADR 0392, `Codebook::authored_rule`). Every other error still refuses: the host's
    /// own, a `@specforge/*` builtin's (the E016 a path's value check reports), and one that judges the records
    /// against the facts collected from the tree (`code/E101`, a reference naming no collected fact).
    ThirdParty,
}

impl Lands {
    /// Whether `d`, an error the plan adds, is written rather than refused; `codes` is the loaded extensions' catalog.
    pub(crate) fn lands(self, d: &Diagnostic, codes: &Codebook) -> bool {
        match self {
            Lands::Nothing => false,
            Lands::Extensions => d.origin().is_some(),
            Lands::ThirdParty => d
                .origin()
                .is_some_and(|origin| codes.authored_rule(origin, &d.code)),
        }
    }
}

/// The diagnostics of `after` that `before` does not have, one by one: (errors, warnings and infos). A diagnostic
/// `judged` names is never an error here, whatever its severity. `moved` relocates the subjects of `before`. An error
/// `lands` writes, read against the catalog `codes`, is not one either (ADR 0303, ADR 0392).
pub(crate) fn added(
    before: &[Diagnostic],
    after: Vec<Diagnostic>,
    judged: impl Fn(&Diagnostic) -> bool,
    moved: &Moves,
    lands: Lands,
    codes: &Codebook,
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
            _ if d.severity == Severity::Error && !judged(&d) && !lands.lands(&d, codes) => {
                errors.push(d)
            }
            _ => rest.push(d),
        }
    }
    (errors, rest)
}

/// Refuse (E080) when `after`, the project `change` leaves `before` as, has an error `before` does not.
pub(crate) fn refuse_new_errors(
    before: &CompiledProject,
    after: &CompiledProject,
    registries: &RegistryBuild,
    change: &PlannedChange,
    lands: Lands,
) -> Result<(), Refusal<EditField>> {
    let (errors, _) = added(
        &before.judged_diagnostics(),
        after.diagnostics(),
        not_added(before, after, Some(change)),
        change.moved(),
        lands,
        registries.codes(),
    );
    if errors.is_empty() {
        Ok(())
    } else {
        Err(new_errors(errors, registries.codes()))
    }
}

/// What the project reports once `change` is written that its checked report does not have now (the change's
/// moves, creations and deletions read by [`not_added`]): the warnings and infos, and the errors [`not_added`]
/// excuses; or E080 naming each new error; or, when the edited graph changes what `declared` does not name,
/// `undeclared_change` (ADR 0198) naming the remainder.
pub fn check(
    view: &ProjectView<'_>,
    change: &PlannedChange,
    declared: &Declared,
    lands: Lands,
) -> Result<Vec<Diagnostic>, Refusal<EditField>> {
    let before = view.project().judged_diagnostics();
    let texts = change.after();
    // The project the write would leave, judged: the guard passes read it and the project before it (ADR 0289).
    let post = view
        .project()
        .edited_with(&texts, change.ledger_after())
        .judged_against(view.project());
    let (errors, rest) = added(
        &before,
        post.diagnostics(),
        not_added(view.project(), &post, Some(change)),
        change.moved(),
        lands,
        view.registries().codes(),
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

fn new_errors(errors: Vec<Diagnostic>, codes: &Codebook) -> Refusal<EditField> {
    let mut message = format!(
        "the edit would leave the project with {} error{} it does not have; nothing was written",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" }
    );
    // Each error with its help line, as `check` renders it: an E061 out of bounds names the range to write in.
    for error in &errors {
        message.push_str(&format!("\n  error[{}]: {}", error.code, error.message));
        if let Some(help) = &error.suggestion {
            message.push_str(&format!("\n    help: {help}"));
        }
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

    #[specforge_test(requirement = "FEATURE-131")]
    fn e080_lists_each_added_error_with_its_help_line() {
        let mut bounded = Diagnostic::new(
            codes::E061,
            "field 'severity' of risk 'RK1' is declared integer, but was given 11",
        );
        bounded.suggestion = Some("write a whole number from 1 to 10".into());
        let plain = Diagnostic::new(codes::E003, "unresolved reference 'x' in entity 'a'");
        let refusal = new_errors(vec![bounded, plain], &specforge_common::Codebook::default());
        assert_eq!(
            refusal.error.message,
            "the edit would leave the project with 2 errors it does not have; nothing was written\n  \
             error[E061]: field 'severity' of risk 'RK1' is declared integer, but was given 11\n    \
             help: write a whole number from 1 to 10\n  \
             error[E003]: unresolved reference 'x' in entity 'a'"
        );
    }

    #[specforge_test(requirement = "FEATURE-131")]
    fn a_pending_journal_is_not_an_error_a_write_adds() {
        let mut pending = Diagnostic::new(codes::W167, "an interrupted write is pending rollback");
        // A preset that raises W167 makes it an error of the edited project.
        pending.severity = Severity::Error;
        let (errors, rest) = added(
            &[],
            vec![pending],
            |_| false,
            &[],
            Lands::Nothing,
            &Codebook::default(),
        );
        assert!(errors.is_empty() && rest.is_empty());
    }

    #[specforge_test(requirement = "FEATURE-131")]
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
            Lands::Nothing,
            &Codebook::default(),
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
            Lands::Nothing,
            &Codebook::default(),
        );
        assert!(errors.is_empty() && rest.is_empty());
    }

    #[specforge_test(requirement = "FEATURE-131")]
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

    #[specforge_test(requirement = "FEATURE-131")]
    fn a_pass_finding_on_another_entity_is_an_added_error() {
        let before = [finding("v1.0/01/01-01/TR1")];
        let (errors, _) = added(
            &before,
            vec![finding("v1.0/01/01-01/TR2")],
            |_| false,
            &[],
            Lands::Nothing,
            &Codebook::default(),
        );
        assert_eq!(errors.len(), 1);
    }

    /// A guard, not red-first: it holds before and after, and fails if the relocation is dropped.
    #[specforge_test(requirement = "FEATURE-131")]
    fn a_moved_entitys_finding_is_still_paired() {
        let before = [finding("v1.0/01/01-01/TR1")];
        let moves = [("v1.0/01".to_string(), "v1.0/02".to_string())];
        let after = vec![finding("v1.0/02/01-01/TR1")];
        assert!(
            added(
                &before,
                after.clone(),
                |_| false,
                &moves,
                Lands::Nothing,
                &Codebook::default()
            )
            .0
            .is_empty()
        );
        assert_eq!(
            added(
                &before,
                after,
                |_| false,
                &[],
                Lands::Nothing,
                &Codebook::default()
            )
            .0
            .len(),
            1
        );
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

    #[specforge_test(requirement = "FEATURE-128")]
    fn a_collectors_plan_lands_an_extensions_error_and_keeps_a_host_one() {
        let host = Diagnostic::untyped("E003", Severity::Error, "unresolved");
        let ext = finding("v1.0/01/01-01/TR1");
        let (errors, _) = added(
            &[],
            vec![host, ext.clone()],
            |_| false,
            &[],
            Lands::Extensions,
            &Codebook::default(),
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "E003");
        assert_eq!(
            added(
                &[],
                vec![ext],
                |_| false,
                &[],
                Lands::Nothing,
                &Codebook::default()
            )
            .0
            .len(),
            1
        );
    }

    #[specforge_test(requirement = "FEATURE-131")]
    fn an_import_lands_an_authored_rule_error_and_keeps_every_other() {
        let entry = |local: &str, facts: &[&str]| specforge_diagnostics::ExtensionCode {
            local: local.into(),
            title: "t".into(),
            explanation: "e".into(),
            docs: None,
            machine: false,
            reported_while: None,
            settled_later: false,
            facts: facts.iter().map(|kind| kind.to_string()).collect(),
        };
        let mut book = Codebook::default();
        book.add("@gsd/core", vec![entry("E502", &[])]).unwrap();
        book.add("@code/core", vec![entry("E101", &["test"])])
            .unwrap();
        let host = Diagnostic::untyped("E003", Severity::Error, "unresolved");
        let path = Diagnostic::from_extension("@specforge/checks", "E016", Severity::Error, "gone");
        // A builtin outside `@specforge/` that judges collected facts: a reference naming no fact refuses (WR-A01).
        let dangling =
            Diagnostic::from_extension("@code/core", "code/E101", Severity::Error, "no fact");
        let uncatalogued =
            Diagnostic::from_extension("@gsd/core", "gsd/E999", Severity::Error, "not listed");
        let rule = finding("v1.0/01/01-01/TR1");
        let (errors, _) = added(
            &[],
            vec![host, path.clone(), dangling, uncatalogued, rule.clone()],
            |_| false,
            &[],
            Lands::ThirdParty,
            &book,
        );
        let codes: Vec<&str> = errors.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(codes, ["E003", "E016", "code/E101", "gsd/E999"]);
        assert!(Lands::ThirdParty.lands(&rule, &book));
        // A collector's plan lands the first-party extension's error too (ADR 0303).
        assert!(Lands::Extensions.lands(&path, &book) && !Lands::ThirdParty.lands(&path, &book));
    }

    #[specforge_test(requirement = "FEATURE-131")]
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
