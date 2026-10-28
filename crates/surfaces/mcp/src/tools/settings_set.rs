//! `specforge.settings_set`: write, or remove, one setting of `specforge.json` (`specforge_ops::extension::set_setting`,
//! ADR 0296).

use crate::args::{Arguments, SettingValue};
use crate::mutation::{Mutated, Mutation, MutationArgs, MutationReply, NoDiagnostics, Written};
use crate::refusal::ArgumentOf;
use crate::target::ProjectRef;
use crate::target::{AnyProject, Project};
use crate::tool::{CoreTool, Effect, ReplySpec, WriteHints};
use specforge_ops::extension::{SettingChange, SettingsSetField};
use specforge_ops::{OpError, OpErrorKind};

/// `specforge.settings_set`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// The extension, as specforge.settings lists it
    extension: String,
    /// The setting's name, or a family's key as written (`models.planning`)
    name: String,
    /// The value, by the setting's declared type (a number, a boolean, a string, a list, an object); exactly one of value and unset
    value: Option<SettingValue>,
    /// Remove the setting from specforge.json: it reverts to its preset or default
    unset: bool,
    /// Preview the change without writing specforge.json
    dry_run: bool,
}

impl MutationArgs for Args {
    fn previews(&self) -> bool {
        self.dry_run
    }
}

impl MutationReply for Reply {
    type After = NoDiagnostics;
}

/// `specforge.settings_set`'s reply: the document `specforge settings set --format json` prints
/// (`McpSettingsSetResult`).
type Reply = specforge_ops::extension::SettingsSetDocument;

pub(crate) struct Tool;

impl ArgumentOf<SettingsSetField> for Tool {
    fn argument(field: SettingsSetField) -> Option<&'static str> {
        match field {
            SettingsSetField::Extension => Some("extension"),
            SettingsSetField::Name => Some("name"),
            SettingsSetField::Value => Some("value"),
        }
    }
}

impl CoreTool for Tool {
    const NAME: &'static str = "specforge.settings_set";
    const DESCRIPTION: &'static str = "Write or remove one setting of specforge.json, admitted by its declared type; a secret is never written";
    const EFFECT: Effect = Effect::Mutates {
        hints: WriteHints {
            // It overwrites or removes a written setting.
            destructive: true,
            idempotent: true,
            open_world: false,
        },
    };
    const SPEC: ReplySpec = ReplySpec::Type("McpSettingsSetResult");
    /// A setting's value is the extension's own type.
    const OPEN_VALUES: &'static [&'static str] = &["$.value", "$.previous"];
    type Args = Args;
    type Input = Project<AnyProject>;
    type Returns = Mutation<Reply>;

    fn call(project: ProjectRef<'_>, args: Args) -> Mutation<Reply> {
        let change = match (args.value, args.unset) {
            (Some(value), false) => SettingChange::Json(value.0),
            (None, true) => SettingChange::Unset,
            _ => {
                let error = OpError::new(
                    OpErrorKind::InvalidInput,
                    "invalid_input",
                    "give exactly one of value and unset",
                );
                return Err(Self::failed(error.about(SettingsSetField::Value)));
            }
        };
        let request = specforge_ops::extension::SetRequest {
            extension: &args.extension,
            name: &args.name,
            change,
            dry_run: args.dry_run,
            caller: project.caller(),
        };
        let outcome = specforge_ops::extension::set_setting(&project.view(), &request)
            .map_err(Self::failed)?;
        Ok(Mutated::new(outcome.document, Written::of(outcome.wrote)))
    }
}
