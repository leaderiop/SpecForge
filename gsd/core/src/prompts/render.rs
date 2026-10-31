//! The render pipeline (D-r42-6) lives in `gsd_kit::prompt::render`; this is @gsd/core's call of it: its assets, its
//! re-pointing and its own `update` text (ours, not a port: D-r43-16, upstream's workflow installs upstream GSD over
//! this one).

use gsd_kit::prompt::render as kit;

use super::{assets, repoint::repoint_for};

pub use kit::{runtime, without_notice};

pub struct Input<'a> {
    /// The upstream command file's stem: `plan-phase` for `commands/gsd/plan-phase.md`.
    pub command: &'a str,
    /// `claude-code`, `codex` or `cursor`.
    pub host: &'a str,
    /// What `$ARGUMENTS` stands for (`args::text`).
    pub arguments: &'a str,
    /// The settings block of the preamble (`settings::block()`).
    pub settings: &'a str,
    /// The prompt's own closing section, when it has one.
    pub tail: &'a str,
}

impl<'a> Input<'a> {
    fn kit(&self) -> kit::Input<'a> {
        kit::Input {
            command: self.command,
            host: self.host,
            arguments: self.arguments,
            settings: self.settings,
            tail: self.tail,
            asset: assets::get,
            repoint: repoint_for,
        }
    }
}

/// The text of the prompt, or why it cannot be built.
pub fn render(input: &Input<'_>) -> Result<String, String> {
    if input.command == "update" {
        return Ok(assemble(super::update::TEXT, input));
    }
    kit::render(&input.kit())
}

/// [`render`] over the text of the command file.
fn assemble(command: &str, input: &Input<'_>) -> String {
    kit::assemble(command, &input.kit())
}

#[cfg(test)]
mod tests;
