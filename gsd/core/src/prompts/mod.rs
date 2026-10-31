//! The prompts of `@gsd/core`: the lifecycle workflows of GSD, ported (r42). `assets` embeds the ported text,
//! `repoint` points its references at the command that serves them, `render` builds the text a model reads for a host,
//! and each family of prompts is a table of `workflow::Spec` rows.

mod args;
pub(crate) mod assets;
mod audit;
mod autonomous;
pub(crate) mod compose;
mod configure;
mod discuss;
mod execute;
mod git;
mod host;
mod learning;
mod navigate;
mod ns;
mod onboard;
mod plan;
mod preamble;
mod project;
mod quick;
pub(crate) mod render;
pub(crate) mod repoint;
mod review;
mod session;
pub(crate) mod setting;
mod ship;
mod slash;
mod spec;
mod update;
mod verify;
mod workflow;
mod workspace;

use specforge_extension_sdk::prelude::*;

pub use spec::GATES;

/// The text of the prompt of upstream command `command` for `host`, as the handler builds it without a tail: the
/// native tests render every flag combination through it.
pub fn render_text(command: &str, host: &str, arguments: &str) -> Result<String, String> {
    render::render(&render::Input {
        command,
        host,
        arguments,
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

pub fn declare(c: &mut ContributionsBuilder) {
    spec::declare(c);
    project::declare(c);
    discuss::declare(c);
    plan::declare(c);
    execute::declare(c);
    verify::declare(c);
    audit::declare(c);
    ship::declare(c);
    navigate::declare(c);
    quick::declare(c);
    autonomous::declare(c);
    session::declare(c);
    review::declare(c);
    configure::declare(c);
    git::declare(c);
    learning::declare(c);
    onboard::declare(c);
    workspace::declare(c);
    update::declare(c);
    ns::declare(c);
}
