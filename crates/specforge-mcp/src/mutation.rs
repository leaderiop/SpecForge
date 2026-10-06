//! What a mutation tool wrote, and the one place that acts on it (ADR 0022).
//!
//! A mutation handler returns [`Mutated`]: its reply and, unless it only
//! previewed, a [`Written`] built from its operation's typed outcome. The
//! dispatcher then calls [`refresh`] (inside the call, while it holds the
//! target) and [`report`]: nothing else in the crate brings a target up to
//! date after a write, records a mutation's events or tells the client
//! which files the call wrote.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_ops::{OpError, Writes};

use crate::state::McpState;
use crate::target::{Call, Reach};
use crate::tool::{IntoOutcome, McpError, ToolOutcome};

/// The reply key naming the files a mutation wrote, relative to the call
/// target's root (absolute outside it), sorted.
pub const FILES_WRITTEN: &str = "files_written";

/// What one mutation call wrote.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Written {
    /// Every file the call created, rewrote or removed: its operation's
    /// [`Writes`], whether the call then succeeded or failed.
    pub files: Writes,
    /// The IDs of the entities the call changed: the renamed entity (by its
    /// new ID), the entities a removal orphaned, the entities an inference
    /// step produced.
    pub entities: BTreeSet<String>,
    /// The domain event the call produces; recorded only when it succeeded.
    pub event: Option<MutationEvent>,
    /// Put what `specforge check` reports for the target, once brought up
    /// to date, in the reply's `diagnostics` (rename's contract).
    pub fresh_diagnostics: bool,
}

impl Written {
    /// The call wrote `files`.
    pub fn files(files: Writes) -> Self {
        Written {
            files,
            ..Written::default()
        }
    }

    /// The call wrote nothing (a run that found nothing to do, or failed
    /// before writing).
    pub fn nothing() -> Self {
        Written::default()
    }

    /// The call changed the entities `ids`.
    pub fn with_entities<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.entities.extend(ids.into_iter().map(Into::into));
        self
    }

    /// The call produces `event` when it succeeds.
    pub fn with_event(mut self, event: MutationEvent) -> Self {
        self.event = Some(event);
        self
    }

    /// The reply carries the target's diagnostics once brought up to date.
    pub fn with_fresh_diagnostics(mut self) -> Self {
        self.fresh_diagnostics = true;
        self
    }
}

/// A domain event a mutation produces (`spec/behaviors/mcp-operations.spec`:
/// init produces `project_initialized`, add_extension `extension_added`).
#[derive(Debug, Clone, PartialEq)]
pub enum MutationEvent {
    /// `extension_added`: an add that installed or enabled an extension.
    ExtensionAdded {
        extension: String,
        version: Option<String>,
    },
    /// `project_initialized`: the scaffold init wrote.
    ProjectInitialized { name: String, path: String },
}

impl MutationEvent {
    /// The event's name, as `spec/events` declares it.
    pub fn name(&self) -> &'static str {
        match self {
            MutationEvent::ExtensionAdded { .. } => "extension_added",
            MutationEvent::ProjectInitialized { .. } => "project_initialized",
        }
    }

    /// Its payload (`timestamp` is added when recorded).
    pub fn params(&self) -> Value {
        match self {
            MutationEvent::ExtensionAdded { extension, version } => {
                json!({ "extension": extension, "version": version })
            }
            MutationEvent::ProjectInitialized { name, path } => {
                json!({ "name": name, "path": path })
            }
        }
    }
}

/// A mutation handler's result: its reply, and what it wrote.
#[derive(Debug)]
pub struct Mutated {
    pub outcome: ToolOutcome,
    /// `None`: the call only previewed (`dry_run`, `check`, `diff`) and is
    /// no mutation: no refresh, no `mcp_mutation_completed`, no
    /// `files_written`.
    pub written: Option<Written>,
}

impl Mutated {
    /// A run that meant to write, and what it wrote (succeeded or not).
    pub fn wrote(outcome: impl IntoOutcome, written: Written) -> Self {
        Mutated {
            outcome: outcome.into_outcome(),
            written: Some(written),
        }
    }

    /// A preview.
    pub fn preview(outcome: impl IntoOutcome) -> Self {
        Mutated {
            outcome: outcome.into_outcome(),
            written: None,
        }
    }

    /// Refused before it wrote anything: a failed mutation.
    pub fn refused(outcome: impl IntoOutcome) -> Self {
        Self::wrote(outcome, Written::nothing())
    }

    /// Refused before it wrote anything, or, for a preview, a failed
    /// preview.
    pub fn refused_unless_preview(preview: bool, outcome: impl IntoOutcome) -> Self {
        if preview {
            Self::preview(outcome)
        } else {
            Self::refused(outcome)
        }
    }

    /// An operation's refusal: a failed mutation carrying what the
    /// operation left written before it failed ([`OpError::writes`]), or,
    /// for a preview, a failed preview.
    pub fn refused_after(preview: bool, mut error: OpError) -> Self {
        let files = std::mem::take(&mut error.writes);
        let outcome = ToolOutcome::from(crate::operations::op_error(error));
        if preview {
            Self::preview(outcome)
        } else {
            Self::wrote(outcome, Written::files(files))
        }
    }

    /// The same, `extra` added to its reply's diagnostics.
    pub fn with_diagnostics(mut self, extra: Vec<Diagnostic>) -> Self {
        self.outcome = self.outcome.with_diagnostics(extra);
        self
    }

    /// The same, its failure naming `tool` ([`ToolOutcome::from_tool`]).
    pub fn from_tool(mut self, tool: &str) -> Self {
        self.outcome = self.outcome.from_tool(tool);
        self
    }
}

/// What a mutation handler returns when it refuses with `?` (only before it
/// writes: `call.project()?`). A refusal after a write must carry its
/// [`Written`] and is returned as `Ok(Mutated::wrote(error, written))`.
pub type MutationHandled = Result<Mutated, Box<McpError>>;

/// A mutation handler's return: [`Mutated`] or [`MutationHandled`].
pub trait IntoMutated {
    fn into_mutated(self) -> Mutated;
}

impl IntoMutated for Mutated {
    fn into_mutated(self) -> Mutated {
        self
    }
}

impl IntoMutated for MutationHandled {
    fn into_mutated(self) -> Mutated {
        self.unwrap_or_else(|refused| Mutated::refused(ToolOutcome::Refused(refused)))
    }
}

/// Bring the call's target up to date with what the mutation wrote, when
/// it wrote anything, succeeded or not; then, when asked, put the target's
/// diagnostics in the reply. The served project on disk is brought up to
/// date (`ensure_fresh`); a served project built in memory is replaced by
/// the project on disk at its root, for a tool that writes project files
/// (`Reach::WritesAnyProject`); another project is compiled again; the
/// directory init created is served when nothing is (ADR 0014 D5).
///
/// Returns the root the written files are named from: the call target's
/// (the directory init created, for init).
pub(crate) fn refresh(call: &mut Call<'_>, reach: Reach, mutated: &mut Mutated) -> Option<PathBuf> {
    if let Some(written) = &mutated.written {
        let wrote = !written.files.is_empty();
        if wrote || written.fresh_diagnostics {
            let diagnostics = call.bring_up_to_date(reach);
            if written.fresh_diagnostics {
                let diagnostics =
                    serde_json::to_value(specforge_common::diagnostics_json(&diagnostics))
                        .unwrap_or_default();
                mutated.outcome.set_field("diagnostics", diagnostics);
            }
        }
    }
    call.new_project_dir()
        .or_else(|| call.root())
        .map(Path::to_path_buf)
}

/// Record what the completed mutation produced, then return its reply: the
/// domain event when it succeeded, then `mcp_mutation_completed`
/// (`files_changed` = the files written, `entities_affected` = the
/// entities changed, `success`), and the reply's `files_written` (the same
/// files, named from `root`; on a refusal, in its `data`). A preview
/// records nothing and is returned as it is.
pub(crate) fn report(
    state: &mut McpState,
    tool: &str,
    root: Option<&Path>,
    mutated: Mutated,
) -> ToolOutcome {
    let Mutated { outcome, written } = mutated;
    let Some(written) = written else {
        return outcome;
    };
    let success = outcome.succeeded();
    if success && let Some(event) = &written.event {
        state.push_event(event.name(), event.params());
    }
    state.push_event(
        "mcp_mutation_completed",
        json!({
            "toolName": tool,
            "files_changed": written.files.len(),
            "entities_affected": written.entities.len(),
            "success": success,
        }),
    );
    let files: Vec<String> = match root {
        Some(root) => written.files.names_under(root),
        None => written
            .files
            .paths()
            .map(|path| path.display().to_string())
            .collect(),
    };
    outcome.with_field(FILES_WRITTEN, Value::from(files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ErrorCode;

    fn served() -> McpState {
        McpState::new()
    }

    fn completed(state: &McpState) -> Vec<Value> {
        state
            .events
            .iter()
            .filter(|e| e.name == "mcp_mutation_completed")
            .map(|e| {
                let mut params = e.params.clone();
                params.as_object_mut().unwrap().remove("timestamp");
                params
            })
            .collect()
    }

    fn payload(outcome: &ToolOutcome) -> Value {
        match outcome {
            ToolOutcome::Done {
                payload: crate::tool::Payload::Json(value),
                ..
            } => value.clone(),
            ToolOutcome::Refused(error) => error.to_json(),
            other => panic!("not JSON: {other:?}"),
        }
    }

    #[test]
    fn a_preview_reports_nothing() {
        let mut state = served();
        let outcome = report(
            &mut state,
            "specforge.rename",
            Some(Path::new("/p")),
            Mutated::preview(ToolOutcome::ok(json!({"dry_run": true}))),
        );
        assert!(state.events.is_empty());
        assert_eq!(payload(&outcome), json!({"dry_run": true}));
    }

    #[test]
    fn a_failed_call_reports_its_writes_and_no_domain_event() {
        let mut state = served();
        let failure = McpError::new(ErrorCode::InternalError, "failed to write /p/a.spec");
        let written = Written::files(Writes::from_iter(["/p/b.spec"])).with_event(
            MutationEvent::ExtensionAdded {
                extension: "@x/y".into(),
                version: None,
            },
        );

        let outcome = report(
            &mut state,
            "specforge.format",
            Some(Path::new("/p")),
            Mutated::wrote(failure, written),
        );

        assert!(!state.events.iter().any(|e| e.name == "extension_added"));
        assert_eq!(
            completed(&state),
            [
                json!({"toolName": "specforge.format", "files_changed": 1, "entities_affected": 0, "success": false})
            ]
        );
        assert_eq!(
            payload(&outcome)["data"]["files_written"],
            json!(["b.spec"])
        );
    }

    #[test]
    fn the_domain_event_precedes_the_mutation_event() {
        let mut state = served();
        let written = Written::files(Writes::from_iter(["/p/specforge.json"])).with_event(
            MutationEvent::ExtensionAdded {
                extension: "@x/y".into(),
                version: None,
            },
        );

        let outcome = report(
            &mut state,
            "specforge.add_extension",
            Some(Path::new("/p")),
            Mutated::wrote(ToolOutcome::ok(json!({"installed": true})), written),
        );

        let names: Vec<&str> = state.events.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["extension_added", "mcp_mutation_completed"]);
        assert_eq!(
            payload(&outcome),
            json!({"installed": true, "files_written": ["specforge.json"]})
        );
    }

    #[test]
    fn event_names_are_the_specs() {
        let added = MutationEvent::ExtensionAdded {
            extension: "@x/y".into(),
            version: Some("1.0.0".into()),
        };
        let initialized = MutationEvent::ProjectInitialized {
            name: "demo".into(),
            path: "/p".into(),
        };
        assert_eq!(added.name(), "extension_added");
        assert_eq!(initialized.name(), "project_initialized");
    }

    #[test]
    fn a_mutation_that_wrote_nothing_lists_no_file() {
        let mut state = served();
        let outcome = report(
            &mut state,
            "specforge.migrate",
            Some(Path::new("/p")),
            Mutated::wrote(ToolOutcome::ok(json!({})), Written::nothing()),
        );
        assert_eq!(payload(&outcome), json!({"files_written": []}));
        assert_eq!(
            completed(&state),
            [
                json!({"toolName": "specforge.migrate", "files_changed": 0, "entities_affected": 0, "success": true})
            ]
        );
    }
}
