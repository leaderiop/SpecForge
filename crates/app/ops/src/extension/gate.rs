//! A command's or prompt's gate (ADR 0291): local codes of its extension, evaluated in its subject's scope before
//! its export runs. A gate never writes and takes no lock; it reads the project the view holds, and asks the guard
//! passes that report one of its codes once.

use std::collections::HashSet;

use serde::{Serialize, Serializer};
use serde_json::{Map, Value, json};
use specforge_common::{Diagnostic, Severity};
use specforge_diagnostics::qualify;
use specforge_graph::Found;
use specforge_protocol_types::Trigger;

use crate::view::{self, ProjectView};
use crate::{OpError, OpErrorKind};

/// The code of a call a gate refused.
pub const GATE_FAILED: &str = "gate_failed";

/// A gate: `codes` are local codes of `extension`; `subject` is the full path of the entity they are scoped to, the
/// project when absent. `on` is the command or prompt the gate belongs to (the guard passes are told).
pub struct Gate<'a> {
    pub extension: &'a str,
    pub codes: &'a [String],
    pub subject: Option<&'a str>,
    pub on: &'a str,
}

/// One gated code, qualified: whether the settings turn it off, and the errors that make it fail.
#[derive(Debug, Clone, PartialEq)]
pub struct GateState {
    pub code: String,
    /// A setting gates the code off: it never fails the gate.
    pub off: bool,
    /// The errors of the code in the gate's scope: the project's report (the settings' raises applied) and the
    /// guard passes the gate asked.
    pub failing: Vec<Diagnostic>,
}

impl GateState {
    pub fn passes(&self) -> bool {
        self.failing.is_empty()
    }
}

impl Serialize for GateState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        json!({"code": self.code, "off": self.off, "passes": self.passes()}).serialize(serializer)
    }
}

/// Whether no gated code fails.
pub fn passes(states: &[GateState]) -> bool {
    states.iter().all(GateState::passes)
}

impl Gate<'_> {
    /// Each code's state over `view`.
    pub fn evaluate(&self, view: &ProjectView<'_>) -> Vec<GateState> {
        let gated = view.env().settings().gated();
        let mut states: Vec<GateState> = self
            .codes
            .iter()
            .map(|local| qualify(self.extension, local))
            .map(|code| GateState {
                off: gated.contains(&code),
                code,
                failing: Vec::new(),
            })
            .collect();
        let asked: Vec<String> = states
            .iter()
            .filter(|s| !s.off)
            .map(|s| s.code.clone())
            .collect();
        if asked.is_empty() {
            return states;
        }
        let trigger = Trigger::gate(self.on, self.subject.map(str::to_string));
        let reported = view.project().gated_against_self(&asked, &trigger);
        let scope = self.scope(view);
        for state in states.iter_mut().filter(|s| !s.off) {
            state.failing = reported
                .iter()
                .filter(|d| d.severity == Severity::Error && d.code == state.code)
                .filter(|d| scope.as_ref().is_none_or(|scope| scope.holds(view, d)))
                .cloned()
                .collect();
        }
        states
    }

    fn scope(&self, view: &ProjectView<'_>) -> Option<Scope> {
        let subject = self.subject?;
        let mut paths: HashSet<String> = view
            .graph()
            .scopes()
            .within(subject)
            .iter()
            .map(|p| p.to_string())
            .collect();
        paths.insert(subject.to_string());
        Some(Scope { paths })
    }
}

/// The entities a gate looks at: its subject and what the subject scopes.
struct Scope {
    paths: HashSet<String>,
}

impl Scope {
    /// Whether `diagnostic` is about an entity of the scope. One that names entities is by them; else by the
    /// entity whose block holds its span; one about neither is about the project, and counts.
    fn holds(&self, view: &ProjectView<'_>, diagnostic: &Diagnostic) -> bool {
        let named = diagnostic
            .data
            .as_deref()
            .map(|data| data.entities())
            .unwrap_or_default();
        if !named.is_empty() {
            return named.iter().any(|id| self.paths.contains(*id));
        }
        let Some(span) = &diagnostic.span else {
            return true;
        };
        let holder = view.graph().nodes().into_iter().find(|node| {
            let held = &node.source_span;
            held.file == span.file
                && held.start_line <= span.start_line
                && span.end_line <= held.end_line
        });
        holder.is_none_or(|node| self.paths.contains(node.id.raw.as_str()))
    }
}

/// The refusal of a call whose gate fails: `what` is the command or prompt as a surface names it.
pub fn refusal(what: &str, states: &[GateState]) -> OpError {
    let failing: Vec<&str> = states
        .iter()
        .filter(|s| !s.passes())
        .map(|s| s.code.as_str())
        .collect();
    let diagnostics: Vec<&Diagnostic> = states.iter().flat_map(|s| &s.failing).collect();
    OpError::new(
        OpErrorKind::PreconditionFailed,
        GATE_FAILED,
        format!("the gate of `{what}` fails: {}", failing.join(", ")),
    )
    .with_suggestion("fix what the failing diagnostics report, or turn the check off in the extension's settings")
    .with_data(json!({"gates": states, "failing": diagnostics}))
}

/// The entity a surface's `spelling` names, as its full path: a short form is accepted (ADR 0284), an ambiguous one
/// is `invalid_input` naming the candidates, an unknown one `entity_not_found`.
pub fn resolve_subject(view: &ProjectView<'_>, spelling: &str) -> Result<String, OpError> {
    match view.graph().find(spelling) {
        Found::One(path) => Ok(path.to_string()),
        Found::Ambiguous(paths) => Err(view::ambiguous(spelling, &paths)),
        Found::Mistyped(_) | Found::None(_) => Err(view::not_found(view.graph(), spelling)),
    }
}

/// The gate of the command `command` (with its extension and the `args` the caller gave, normalized): its subject
/// resolved and the argument replaced by its full path, then its gate evaluated. A failing gate or a subject that
/// names nothing is the refusal; the export has not been called.
pub(crate) fn admit(
    view: &ProjectView<'_>,
    command: &super::command::ExtensionCommand,
    args: &mut Map<String, Value>,
) -> Result<(), OpError> {
    let declared = command.declaration();
    let mut subject = None;
    if let Some(arg) = &declared.subject
        && let Some(Value::String(written)) = args.get(arg)
    {
        let path = resolve_subject(view, written)?;
        args.insert(arg.clone(), Value::String(path.clone()));
        subject = Some(path);
    }
    if declared.gates.is_empty() {
        return Ok(());
    }
    let gate = Gate {
        extension: command.extension(),
        codes: &declared.gates,
        subject: subject.as_deref(),
        on: command.id(),
    };
    let states = gate.evaluate(view);
    if passes(&states) {
        Ok(())
    } else {
        Err(refusal(&command.tool_name(), &states))
    }
}
