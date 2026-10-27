//! `specforge.settings`: each extension's settings as the project resolves them
//! (`specforge_ops::extension::settings`).

use crate::args::Arguments;
use crate::reply::Answered;
use crate::target::View;
use crate::tool::{CoreTool, Effect, ReplySpec, ToolGroup};
use specforge_ops::view::ProjectView;

/// `specforge.settings`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// List the settings nothing sets and that have no default too
    all: bool,
    /// List only this extension's settings
    extension: Option<String>,
}

/// `specforge.settings`'s reply: the document `specforge settings --format json` prints
/// (`McpSettingsResult`).
type Reply = specforge_ops::extension::SettingsDocument;

pub(crate) struct Tool;

impl CoreTool for Tool {
    const NAME: &'static str = "specforge.settings";
    const DESCRIPTION: &'static str = "List each extension's settings: the value, where it came from, what it gates, and each secret's status (never its value)";
    const EFFECT: Effect = Effect::Reads {
        group: ToolGroup::Management,
    };
    const SPEC: ReplySpec = ReplySpec::Type("McpSettingsResult");
    /// A setting's value and default are the extension's own types.
    const OPEN_VALUES: &'static [&'static str] = &[
        "$.extensions[].settings[].default",
        "$.extensions[].settings[].value",
    ];
    type Args = Args;
    type Input = View;
    type Returns = Answered<Reply>;

    fn call(view: ProjectView<'_>, args: Args) -> Answered<Reply> {
        Ok(specforge_ops::extension::settings(&view, args.all, args.extension.as_deref()).into())
    }
}
