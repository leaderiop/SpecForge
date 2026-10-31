//! Ported from gsd-core v1.19.0 hooks/gsd-config-reload.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `config_reload` tells the session the settings when `specforge.json` changes (Claude Code's `FileChanged`, ADR
//! 0321). Upstream reads `.planning/config.json` and summarises its `runtime`, `mode`, `hooks`, `workflow` and `models`
//! keys; here the settings are the call's own (`Settings::current`, defaults included, so a family lists every key it
//! declares, not only the ones the file sets). A deleted file says so. Upstream's "could not be parsed" message is not
//! ported: a `specforge.json` that does not parse does not load the extension (E069), so the hook is never reached.
//! The host writes the context as `additionalContext`, as upstream does; Claude Code's docs read only `systemMessage`
//! from a `FileChanged` answer (ADR 0321 Limits).

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use super::facts::Facts;

/// The settings the summary names, as strings; a family is the keys under `hooks.`, `workflow.` or `models.`.
#[derive(Debug, Default)]
pub(super) struct Snapshot {
    pub runtime: Option<String>,
    pub mode: Option<String>,
    pub hooks: Vec<(String, String)>,
    pub workflow: Vec<(String, String)>,
    pub models: Vec<(String, String)>,
}

/// A value as JavaScript's template literal prints it.
fn show(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn family(s: &Settings, prefix: &str) -> Vec<(String, String)> {
    s.under(prefix)
        .map(|(k, v)| (k.to_string(), show(v)))
        .collect()
}

impl Snapshot {
    fn of(s: &Settings) -> Self {
        Snapshot {
            runtime: s.get("runtime").map(show),
            mode: s.get("mode").map(show),
            hooks: family(s, "hooks"),
            workflow: family(s, "workflow"),
            models: family(s, "models"),
        }
    }
}

pub(super) fn config_reload(f: &Facts, settings: &Snapshot) -> HookOutput {
    let context = if f.change.as_deref() == Some("unlink") {
        "GSD config (specforge.json) was deleted. Falling back to built-in defaults for this session.".to_string()
    } else {
        let mut lines = vec!["GSD config reloaded (specforge.json updated):".to_string()];
        if let Some(r) = &settings.runtime {
            lines.push(format!("  runtime: {r}"));
        }
        if let Some(m) = &settings.mode {
            lines.push(format!("  mode: {m}"));
        }
        for (name, keys) in [
            ("hooks", &settings.hooks),
            ("workflow", &settings.workflow),
            ("models", &settings.models),
        ] {
            if !keys.is_empty() {
                let body: Vec<String> = keys.iter().map(|(k, v)| format!("{k}={v}")).collect();
                lines.push(format!("  {name}: {{ {} }}", body.join(", ")));
            }
        }
        if lines.len() == 1 {
            lines.push("  (no notable keys changed)".into());
        }
        lines.join("\n")
    };
    HookOutput {
        context: Some(context),
        ..HookOutput::default()
    }
}

pub(super) fn declare(c: &mut ContributionsBuilder) {
    c.hook("config_reload", HookEvent::FileChanged, |h| {
        h.description("Tells the session the settings when specforge.json changes")
            .paths(&["specforge.json"])
            .handler(|call| {
                Ok(config_reload(
                    &Facts::of(call),
                    &Snapshot::of(&Settings::current()),
                ))
            });
    });
}

#[cfg(test)]
mod tests;
