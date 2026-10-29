//! `settings` and `config` (D-r43-1 rows 12-13): the interactive settings menus. Every value is written by the host's
//! own `specforge settings set @gsd/core`, so the host validates the key and the type (D-r43-18).

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Slash, Spec};

/// What differs from GSD's `config.json`: one writer, and what is not a setting here.
fn writes(call: &PromptCall<'_>) -> String {
    let mut text = String::from(
        "\n\n## How a value is written\n\n\
         Every value is written with `specforge settings set @gsd/core <key> <value>` (the MCP tool \
         `specforge.settings_set`), which checks the key and its type. The settings are the file `specforge.json`; \
         there is no `config.json` and no workstream copy.\n\n\
         - A key another extension declares (`workflow.ui_*`, `workflow.code_review*`, `security_*`) is refused \
         while that extension is not installed. Once it is, set it with its name: \
         `specforge settings set @gsd/ui workflow.ui_phase false`.\n\
         - A search key (`brave_search`, `firecrawl`, `exa_search` and the others) is a secret: it is an environment \
         variable, and `settings set` never writes it. `brave_search true|false` only forces the source on or off.\n\
         - `commit_docs`, `search_gitignored` and `graphify.*` are not settings here: the spec is source and is \
         committed like code. Skip their questions.\n",
    );
    if let Some(profile) = call.str("profile") {
        text.push_str(&format!(
            "\n## The profile\n\nThe profile asked for is `{profile}`: run \
             `specforge settings set @gsd/core model_profile {profile}` and show its output.\n"
        ));
    }
    text
}

static SETTINGS: Spec = Spec {
    id: "settings",
    title: "Configure the workflow toggles and the model profile",
    description: "Configure GSD workflow toggles and model profile",
    command: "settings",
    args: &[],
    subject: None,
    gates: &[],
    steps: &[],
    slash: Some(Slash {
        hint: "",
        tools: &["read", "write", "shell", "search", "ask_user"],
        effort: None,
    }),
    tail: Some(writes),
};

static CONFIG: Spec = Spec {
    id: "config",
    title: "Configure the settings, the advanced knobs and the integrations",
    description: "Configure GSD settings — workflow toggles, advanced knobs, integrations, and model profile",
    command: "config",
    args: &[
        Arg::flag("advanced"),
        Arg::flag("integrations"),
        Arg::text("profile"),
    ],
    subject: None,
    gates: &[],
    steps: &[],
    slash: Some(Slash {
        hint: "[--advanced | --integrations | --profile <name>]",
        tools: &["read", "write", "shell", "search", "ask_user"],
        effort: None,
    }),
    tail: Some(writes),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &SETTINGS);
    workflow::declare(c, &CONFIG);
}
