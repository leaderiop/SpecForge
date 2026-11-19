//! The prompts of `@gsd/core`: the lifecycle workflows of GSD, ported (r42). `assets` embeds the ported text,
//! `repoint` points its references at the command that serves them, `render` builds the text a model reads for a host,
//! and each family of prompts is a table of `workflow::Spec` rows.

use gsd_kit::prompt::args;
pub(crate) mod assets;
mod audit;
mod autonomous;
pub(crate) use gsd_kit::prompt::compose;
mod configure;
mod discuss;
mod execute;
mod git;
use gsd_kit::prompt::host;
mod learning;
mod navigate;
mod ns;
mod onboard;
mod plan;
mod project;
mod quick;
pub(crate) mod render;
pub(crate) mod repoint;
mod review;
mod session;
pub(crate) use gsd_kit::prompt::setting;
mod ship;
mod slash;
mod spec;
mod update;
mod verify;
mod workflow;
mod workspace;

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use workflow::{Do, Spec};

pub use spec::GATES;

/// The text of the prompt of upstream command `command` for `host`, as the handler builds it without a tail: the
/// native tests render every flag combination through it. `values` are the placeholders' (`args::values`).
pub fn render_text(
    command: &str,
    host: &str,
    arguments: &str,
    values: &args::Values,
) -> Result<String, String> {
    render::render(&render::Input {
        command,
        host,
        arguments,
        values,
        settings: &crate::settings::block(),
        tail: "",
    })
}

/// The text of an embedded asset, attribution line included.
pub use assets::get as asset;
/// The paths of every embedded asset.
pub use assets::paths as asset_paths;
/// `text` as `host` reads it (command names, tool names, paths).
pub(crate) use host::convert as convert_for_host;
/// The attribution line of an asset is a notice, not text a model reads.
pub use render::without_notice;
/// `text` with its install paths re-pointed at `specforge gsd reference`.
pub use repoint::repoint;

/// Every workflow the prompt modules declare, family by family in declaration order: what [`declare`] declares, and
/// what a gate guard reads of the workflow it gates ([`judges_run_by`]).
static FAMILIES: &[&[&Spec]] = &[
    spec::SPECS,
    project::SPECS,
    discuss::SPECS,
    plan::SPECS,
    execute::SPECS,
    verify::SPECS,
    audit::SPECS,
    ship::SPECS,
    navigate::SPECS,
    quick::SPECS,
    autonomous::SPECS,
    session::SPECS,
    review::SPECS,
    configure::SPECS,
    git::SPECS,
    learning::SPECS,
    onboard::SPECS,
    workspace::SPECS,
    update::SPECS,
    ns::SPECS,
];

/// The workflow registry: every prompt `@gsd/core` declares, in declaration order.
pub(crate) fn workflows() -> impl Iterator<Item = &'static Spec> {
    FAMILIES.iter().flat_map(|family| family.iter().copied())
}

/// The judges the workflow `id` runs itself: the workflow as `prompt:<id>` (a prompt judge) and the agents of its agent
/// steps. A workflow it continues with (a prompt step) is not counted: that one runs behind its own gate. An id that is
/// no workflow (a command's gate) runs none.
pub(crate) fn judges_run_by(id: &str) -> BTreeSet<String> {
    let Some(spec) = workflows().find(|s| s.id == id) else {
        return BTreeSet::new();
    };
    let agents = spec.steps.iter().filter_map(|step| match step.does {
        Do::Agent(agent) => Some(agent.to_string()),
        Do::Command(_) | Do::Prompt(_) => None,
    });
    std::iter::once(format!("prompt:{id}"))
        .chain(agents)
        .collect()
}

pub fn declare(c: &mut ContributionsBuilder) {
    for spec in workflows() {
        workflow::declare(c, spec);
    }
}

#[cfg(test)]
mod tests;
