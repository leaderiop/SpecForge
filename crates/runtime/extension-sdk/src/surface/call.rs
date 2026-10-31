//! [`CommandCall`]: one call of a command, as its handler sees it.

use super::Command;
use crate::{
    CommandError, CommandEvidence, CommandFormat, CommandGraph, CommandInput, CommandOutput,
    StateView,
};
use serde_json::Value;
use specforge_protocol_types::command_args::normalize_args;
use specforge_protocol_types::{CommandArgDescriptor, CommandArgType};

/// One call of a command, as its handler sees it: the args the caller set,
/// checked against the command's declaration, and the rest of the
/// [`CommandInput`].
///
/// The arg accessors read only args the command declares, as the type it
/// declares them: reading an undeclared arg, or a declared one as another
/// type, is a bug in the extension and panics (the host reports the trap
/// as E028), so a test calling the command finds it.
pub struct CommandCall<'a> {
    pub(super) command: &'a Command,
    pub(crate) input: &'a CommandInput,
    /// The declared args the caller set, normalized to their declared type
    /// (a string, an integer or a boolean).
    pub(super) values: serde_json::Map<String, Value>,
}

impl<'a> CommandCall<'a> {
    /// `input`'s args normalized by the host's rule
    /// ([`command_args::normalize_args`]): an absent arg takes its declared
    /// default and an unset flag is `false`; a required arg missing, a value
    /// of another type, or an arg the command does not declare is
    /// `INVALID_INPUT`.
    pub(super) fn new(command: &'a Command, input: &'a CommandInput) -> Result<Self, CommandError> {
        let values = normalize_args(&command.descriptor.args, &input.args)
            .map_err(|error| error.to_command_error())?;
        Ok(CommandCall {
            command,
            input,
            values,
        })
    }

    /// The id of the command called.
    pub fn id(&self) -> &str {
        &self.command.descriptor.id
    }

    /// The compiled project's graph.
    pub fn graph(&self) -> &'a CommandGraph {
        &self.input.graph
    }

    /// A builder for the edit plan this command answers: each op takes the revision of its file from this call.
    pub fn plan(&self) -> crate::PlanBuilder {
        crate::PlanBuilder::of_graph(&self.input.graph, &self.input.revisions)
    }

    /// The format the caller asked for.
    pub fn format(&self) -> CommandFormat {
        self.input.format
    }

    /// Whether the caller asked for `json`.
    pub fn is_json(&self) -> bool {
        self.input.is_json()
    }

    /// The extension's state, for a command that declares `reads_state`; `None` otherwise: the host sends nothing
    /// (ADR 0310).
    pub fn state(&self) -> Option<&'a StateView> {
        self.input.state.as_ref()
    }

    /// The host's date, `YYYY-MM-DD`, UTC; empty when the host passed none.
    pub fn today(&self) -> &'a str {
        &self.input.today
    }

    /// The project root, absolute and canonical (the host resolves symlinks).
    pub fn cwd(&self) -> &'a str {
        &self.input.cwd
    }

    /// What the project's recorded test report proves, per entity that
    /// counts toward coverage, as the host scores it; `none` without a
    /// report (or from a host older than the field).
    pub fn evidence(&self) -> &'a CommandEvidence {
        &self.input.evidence
    }

    /// Whether the declared arg `name` has a value: the caller's, or its
    /// declared default (a flag always has one, `false` unless set).
    pub fn is_set(&self, name: &str) -> bool {
        self.declared(name);
        self.values.contains_key(name)
    }

    /// The string, path or enum arg `name`, when set.
    pub fn str(&self, name: &str) -> Option<&str> {
        let arg = self.declared(name);
        self.expect(
            arg,
            matches!(
                arg.arg_type,
                CommandArgType::String | CommandArgType::Path | CommandArgType::Enum { .. }
            ),
            "a string",
        );
        self.values.get(name).and_then(Value::as_str)
    }

    /// The count arg `name` ([`ArgBuilder::count`]), when set; a count
    /// beyond `usize` (on a 32-bit guest) is `usize::MAX`.
    pub fn count(&self, name: &str) -> Option<usize> {
        let arg = self.declared(name);
        self.expect(arg, is_count(arg), "a count");
        self.values
            .get(name)
            .and_then(Value::as_u64)
            .map(|n| usize::try_from(n).unwrap_or(usize::MAX))
    }

    /// The integer (or count) arg `name`, when set.
    pub fn integer(&self, name: &str) -> Option<i64> {
        let arg = self.declared(name);
        self.expect(arg, arg.arg_type == CommandArgType::Integer, "an integer");
        self.values.get(name).and_then(Value::as_i64)
    }

    /// The flag `name`; `false` when unset.
    pub fn flag(&self, name: &str) -> bool {
        let arg = self.declared(name);
        self.expect(arg, arg.arg_type == CommandArgType::Bool, "a flag");
        self.values
            .get(name)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// `payload` as pretty JSON (one root object, then a newline) when the
    /// caller asked for `json`, else what `human` writes.
    pub fn render<T: serde::Serialize>(
        &self,
        payload: &T,
        human: impl FnOnce(&mut String),
    ) -> CommandOutput {
        let mut out = String::new();
        if self.is_json() {
            out = serde_json::to_string_pretty(payload).unwrap_or_default();
            out.push('\n');
        } else {
            human(&mut out);
        }
        CommandOutput::ok(out)
    }

    /// `error` on stderr in the format the caller asked for, exit
    /// `exit_code` ([`CommandOutput::error`]).
    pub fn fail(&self, error: &CommandError, exit_code: i32) -> CommandOutput {
        CommandOutput::error(self.format(), error, exit_code)
    }

    fn declared(&self, name: &str) -> &'a CommandArgDescriptor {
        let command = self.command;
        command
            .descriptor
            .args
            .iter()
            .find(|a| a.name == name)
            .unwrap_or_else(|| {
                panic!(
                    "command '{}' reads arg '{name}', which it does not declare",
                    command.descriptor.id
                )
            })
    }

    fn expect(&self, arg: &CommandArgDescriptor, ok: bool, what: &str) {
        assert!(
            ok,
            "command '{}' reads arg '{}' as {what}, but declares it {:?}{}",
            self.command.descriptor.id,
            arg.name,
            arg.arg_type,
            if is_count(arg) { " (a count)" } else { "" }
        );
    }
}

/// Whether `arg` is a count ([`ArgBuilder::count`]): an integer that is
/// never negative.
fn is_count(arg: &CommandArgDescriptor) -> bool {
    arg.arg_type == CommandArgType::Integer && arg.minimum.is_some_and(|minimum| minimum >= 0)
}
