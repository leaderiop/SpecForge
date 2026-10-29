//! The slash commands that run a command of this extension (D-r41-14): one binding each, with upstream's argument hint.
//! The host renders the trampoline; the description is the command's. Each carries the notice of the upstream file.

use specforge_extension_sdk::prelude::*;

use crate::settings::surface::{first_cluster, setting};

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

/// Binds `command` to `/gsd-<command>`, shown while the setting of the first cluster that lists it is on.
fn bind(c: &mut ContributionsBuilder, command: &str, hint: Option<&str>) {
    let stem = command.replace('_', "-");
    let when = first_cluster(&stem).map(setting);
    c.slash_command(command, |s| {
        if let Some(hint) = hint {
            s.argument_hint(hint);
        }
        if let Some(when) = &when {
            s.when(when);
        }
        s.tools(&[]).attribution(attribution(&stem));
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
    bind(c, "help", None);
    bind(c, "health", None);
    bind(c, "stats", None);
    bind(
        c,
        "surface",
        Some("[list | status | enable --cluster <c> | disable --cluster <c> | reset]"),
    );
}
