//! `surface` (D-r41-13, U31): which skills are shown. Upstream's `/gsd:surface` switches a cluster without a
//! reinstall; a SpecForge command cannot write a setting, so `list` and `status` report each cluster's setting and
//! the others print the exact `specforge settings` and `specforge install` lines that do it.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, required, run, text};
use crate::settings::surface::{CLUSTERS, setting};

const ACTIONS: &[&str] = &["list", "status", "enable", "disable", "reset", "profile"];
const EXTENSION: &str = "@gsd/core";
const REINSTALL: &str = "specforge install claude-code    (or codex, cursor)";

fn cluster<'a>(call: &CommandCall<'a>) -> Result<&'static str, String> {
    let name = required(call, "cluster")?;
    CLUSTERS
        .iter()
        .map(|(c, _)| *c)
        .find(|c| *c == name)
        .ok_or_else(|| {
            format!(
                "no cluster '{name}': the clusters are {}",
                CLUSTERS
                    .iter()
                    .map(|(c, _)| *c)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

fn list() -> (serde_json::Value, String) {
    let settings = Settings::current();
    let mut rows = Vec::new();
    let mut human = String::new();
    for (name, members) in CLUSTERS {
        let key = setting(name);
        let on = settings.bool(&key).unwrap_or(true);
        human.push_str(&format!(
            "{name}: {} ({key})\n  {}\n",
            if on { "shown" } else { "hidden" },
            members.join(", ")
        ));
        rows.push(json!({ "cluster": name, "setting": key, "shown": on, "skills": members }));
    }
    (json!({ "clusters": rows }), human)
}

fn surface(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let action = required(call, "action")?;
    let (payload, human) = match action {
        "list" | "status" => {
            if text(call, "cluster")?.is_some() {
                return Err(format!("'{action}' takes no cluster"));
            }
            list()
        }
        "enable" | "disable" => {
            let name = cluster(call)?;
            let value = action == "enable";
            let line = format!(
                "specforge settings set {EXTENSION} {} {value}",
                setting(name)
            );
            (
                json!({ "action": action, "cluster": name, "setting": setting(name), "run": [line, REINSTALL] }),
                format!("To {action} {name}, run:\n  {line}\n  {REINSTALL}\n"),
            )
        }
        "reset" => {
            let lines: Vec<String> = CLUSTERS
                .iter()
                .map(|(name, _)| format!("specforge settings unset {EXTENSION} {}", setting(name)))
                .collect();
            let human = format!(
                "To show every cluster again, run:\n  {}\n  {REINSTALL}\n",
                lines.join("\n  ")
            );
            (json!({ "action": "reset", "run": lines }), human)
        }
        _ => {
            return Err("upstream's surface profiles have no counterpart here: the `profile` setting picks the GSD preset, and each cluster is a setting (`surface list`)".into());
        }
    };
    Ok(answer(call, payload, human, None))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("surface", |cmd| {
        cmd.description("Show which skill clusters are shown, or print the lines that show or hide one; it changes no setting itself")
            .arg("action", |a| {
                a.required().one_of(ACTIONS).description("what to do (profile is refused)");
            })
            .arg("cluster", |a| {
                a.description("the cluster (enable and disable)");
            })
            .handler(|call| run(call, surface));
    });
}
