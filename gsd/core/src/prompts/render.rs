//! The render pipeline (D-r42-6) lives in `gsd_kit::prompt::render`; this is @gsd/core's call of it: its assets, its
//! re-pointing and its own texts, [`OWN`]: `update` (ours, not a port: D-r43-16, upstream's workflow installs upstream
//! GSD over this one) and `verify-invariant` (GSD has no invariant).

use gsd_kit::prompt::args::Values;
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
    /// The values of the `{{arg:}}` placeholders (`args::values`).
    pub values: &'a Values,
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
            values: self.values,
            settings: self.settings,
            extension: "@gsd/core",
            files: gsd_kit::prompt::preamble::FILES,
            tail: self.tail,
            asset: assets::get,
            repoint: repoint_for,
        }
    }
}

/// The commands whose text is ours, rendered in place of a ported command body: no upstream file is read for them.
const OWN: &[(&str, &str)] = &[
    ("update", super::update::TEXT),
    ("verify-invariant", super::invariant::TEXT),
];

/// Our own text of `command`, when it has one rather than a ported upstream file.
pub fn own(command: &str) -> Option<&'static str> {
    OWN.iter()
        .find(|(c, _)| *c == command)
        .map(|(_, text)| *text)
}

/// The text of the prompt, or why it cannot be built.
pub fn render(input: &Input<'_>) -> Result<String, String> {
    match own(input.command) {
        Some(text) => Ok(assemble(text, input)),
        None => kit::render(&input.kit()),
    }
}

/// [`render`] over the text of the command file.
fn assemble(command: &str, input: &Input<'_>) -> String {
    kit::assemble(command, &input.kit())
}

#[cfg(test)]
mod tests;
