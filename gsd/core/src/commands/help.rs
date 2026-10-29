//! `help` (D-r41-13): what the extension serves, built from the declaration `build()` returns, so there is no second
//! table to keep in step. The slash commands come first, grouped by the cluster that shows them, then the prompts that
//! have no slash command, then every command.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, run};
use crate::settings::surface::{CLUSTERS, first_cluster};

/// The first line of a description: what a listing shows.
fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default().trim()
}

struct Slash {
    stem: String,
    description: String,
    cluster: Option<&'static str>,
}

fn help(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let declaration = crate::build().declaration();
    let surfaces = &declaration.surfaces;
    let mut slashes: Vec<Slash> = Vec::new();
    let mut bound_prompts = Vec::new();
    for slash in &surfaces.slash_commands {
        let (id, description) = match (&slash.command, &slash.prompt) {
            (Some(id), _) => (
                id.as_str(),
                surfaces
                    .commands
                    .iter()
                    .find(|c| &c.id == id)
                    .map(|c| c.description.as_str()),
            ),
            (None, Some(id)) => {
                bound_prompts.push(id.as_str());
                (
                    id.as_str(),
                    surfaces
                        .prompts
                        .iter()
                        .find(|p| &p.id == id)
                        .map(|p| p.description.as_str()),
                )
            }
            (None, None) => continue,
        };
        let stem = id.replace('_', "-");
        slashes.push(Slash {
            cluster: first_cluster(&stem),
            description: first_line(description.unwrap_or_default()).to_string(),
            stem,
        });
    }
    let mut human = String::from("Slash commands\n");
    let mut grouped = Vec::new();
    let order = CLUSTERS.iter().map(|(name, _)| Some(*name)).chain([None]);
    for cluster in order {
        let members: Vec<&Slash> = slashes.iter().filter(|s| s.cluster == cluster).collect();
        if members.is_empty() {
            continue;
        }
        human.push_str(&format!("\n  {}\n", cluster.unwrap_or("other")));
        for s in members {
            human.push_str(&format!("    /gsd-{:<18} {}\n", s.stem, s.description));
            grouped.push(json!({
                "slash": format!("/gsd-{}", s.stem), "cluster": s.cluster, "description": s.description,
            }));
        }
    }
    let prompts: Vec<_> = surfaces
        .prompts
        .iter()
        .filter(|p| !bound_prompts.contains(&p.id.as_str()))
        .collect();
    human.push_str("\nPrompts\n");
    let mut prompt_rows = Vec::new();
    for p in &prompts {
        human.push_str(&format!(
            "  specforge gsd {:<22} {}\n",
            p.id.replace('_', "-"),
            first_line(&p.description)
        ));
        prompt_rows.push(json!({ "prompt": p.id, "description": first_line(&p.description) }));
    }
    human.push_str("\nCommands\n");
    let mut command_rows = Vec::new();
    for c in &surfaces.commands {
        human.push_str(&format!(
            "  specforge gsd {:<22} {}{}\n",
            c.id.replace('_', "-"),
            first_line(&c.description),
            if c.writes { " (writes)" } else { "" }
        ));
        command_rows.push(json!({
            "command": c.id.replace('_', "-"), "writes": c.writes, "description": first_line(&c.description),
        }));
    }
    Ok(answer(
        call,
        json!({ "slash_commands": grouped, "prompts": prompt_rows, "commands": command_rows }),
        human,
        None,
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("help", |cmd| {
        cmd.description("List what @gsd/core serves: its slash commands by cluster, its prompts and its commands")
            .handler(|call| run(call, help));
    });
}
