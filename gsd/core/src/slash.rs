//! The slash commands that run a command of this extension (D-r41-14): one binding each, with upstream's argument hint.
//! The host renders the trampoline; the description is the command's. Each carries the notice of the upstream file.
//!
//! Also ports gsd-core bin/lib modules: command-aliases.cjs.

use specforge_extension_sdk::prelude::*;

use crate::settings::surface::{clusters, setting};

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

/// Binds `command` to `/gsd-<command>`, shown while the setting of every cluster that lists it is on.
fn bind(c: &mut ContributionsBuilder, command: &str, hint: Option<&str>) {
    let stem = command.replace('_', "-");
    c.slash_command(command, |s| {
        if let Some(hint) = hint {
            s.argument_hint(hint);
        }
        for cluster in clusters(&stem) {
            s.when(&setting(cluster));
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
