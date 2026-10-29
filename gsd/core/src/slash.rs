//! The slash commands that run a command of this extension (D-r41-14): one binding each, with upstream's argument hint.
//! The host renders the trampoline; the description is the command's. Each carries the notice of the upstream file.

use specforge_extension_sdk::prelude::*;

/// The notice of `commands/gsd/<name>.md`, the file whose hint and description a binding keeps.
fn attribution(name: &str) -> Attribution {
    Attribution {
        upstream: "gsd-core".to_string(),
        path: format!("commands/gsd/{name}.md"),
        version: "v1.18.0".to_string(),
        license: "MIT".to_string(),
        copyright: "Copyright (c) 2026 Open GSD".to_string(),
    }
}

/// Binds `command` to `/gsd-<command>`.
fn bind(c: &mut ContributionsBuilder, command: &str, hint: Option<&str>) {
    c.slash_command(command, |s| {
        if let Some(hint) = hint {
            s.argument_hint(hint);
        }
        s.tools(&[]).attribution(attribution(command));
    });
}

pub fn declare(c: &mut ContributionsBuilder) {
    bind(
        c,
        "phase",
        Some("[--insert | --remove | --edit] <phase-name-or-number>"),
    );
    bind(c, "workstreams", None);
    bind(c, "cleanup", None);
}
