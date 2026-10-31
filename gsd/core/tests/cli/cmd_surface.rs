//! `help`, `surface` and the cluster settings (D-r41-13).

use serde_json::json;

use super::commands_support::*;
use super::harness::project;
use gsd_core::settings::surface::CLUSTERS;

#[test]
fn specforge_gsd_help_runs() {
    let p = project("empty");
    let run = gsd(&p, "help", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
}

#[test]
fn help_lists_the_slash_commands_and_the_prompt() {
    let p = project("empty");
    let found = json(&gsd(&p, "help", &["--format", "json"]));
    let slashes: Vec<&str> = found["slash_commands"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|s| s["slash"].as_str().expect("a name"))
        .collect();
    for want in [
        "/gsd-phase",
        "/gsd-workstreams",
        "/gsd-cleanup",
        "/gsd-help",
        "/gsd-health",
        "/gsd-stats",
        "/gsd-surface",
        "/gsd-new-project",
        "/gsd-new-milestone",
        "/gsd-discuss-phase",
        "/gsd-spec-phase",
        "/gsd-mvp-phase",
    ] {
        assert!(slashes.contains(&want), "{want} in {slashes:?}");
    }
    let prompts: Vec<&str> = found["prompts"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|s| s["prompt"].as_str().expect("an id"))
        .collect();
    // A prompt with a slash command is listed under it, not twice.
    assert!(!prompts.contains(&"spec_phase"), "{prompts:?}");
    let commands = found["commands"].as_array().expect("a list");
    assert!(
        commands
            .iter()
            .any(|c| c["command"] == "help" && c["writes"] == false)
    );
    assert!(
        commands
            .iter()
            .any(|c| c["command"] == "phase" && c["writes"] == true)
    );
    // The cluster that shows /gsd-help is core_loop.
    let help = found["slash_commands"]
        .as_array()
        .expect("a list")
        .iter()
        .find(|s| s["slash"] == "/gsd-help")
        .expect("help");
    assert_eq!(help["cluster"], "core_loop");
}

#[test]
fn surface_list_reports_each_clusters_setting() {
    let p = project("empty");
    p.settings(json!({ "specforge.surface.docs": false }));
    let found = json(&gsd(&p, "surface", &["list", "--format", "json"]));
    let rows = found["clusters"].as_array().expect("a list");
    assert_eq!(rows.len(), 10);
    let docs = rows.iter().find(|r| r["cluster"] == "docs").expect("docs");
    assert_eq!(docs["shown"], false);
    assert_eq!(docs["setting"], "specforge.surface.docs");
    assert!(rows.iter().filter(|r| r["shown"] == true).count() == 9);
}

#[test]
fn surface_disable_prints_the_settings_line() {
    let p = project("empty");
    let before = bytes(&p);
    let run = gsd(&p, "surface", &["disable", "--cluster", "utility"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let all = run.all();
    assert!(
        all.contains("specforge settings set @gsd/core specforge.surface.utility false"),
        "{all}"
    );
    assert!(all.contains("specforge install"), "{all}");
    let enable = gsd(&p, "surface", &["enable", "--cluster", "utility"]);
    assert!(
        enable.all().contains("specforge.surface.utility true"),
        "{}",
        enable.all()
    );
    let reset = gsd(&p, "surface", &["reset"]);
    assert!(
        reset
            .all()
            .contains("specforge settings unset @gsd/core specforge.surface.core_loop"),
        "{}",
        reset.all()
    );
    refused(
        &gsd(&p, "surface", &["disable", "--cluster", "nope"]),
        "invalid_input",
        &["no cluster 'nope'"],
    );
    unchanged(&p, &before);
}

#[test]
fn surface_profile_is_refused() {
    let p = project("empty");
    refused(
        &gsd(&p, "surface", &["profile"]),
        "invalid_input",
        &["no counterpart"],
    );
}

#[test]
fn a_disabled_cluster_hides_its_skills_on_install() {
    let p = project("empty");
    p.settings(json!({ "specforge.surface.utility": false }));
    let run = p.install("claude-code", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    for gone in ["gsd-phase", "gsd-cleanup", "gsd-surface"] {
        assert!(
            !p.exists(&format!(".claude/skills/{gone}/SKILL.md")),
            "{gone} is hidden"
        );
    }
    assert!(p.exists(".claude/skills/gsd-workstreams/SKILL.md"));
}

/// Upstream hides a skill in any disabled cluster: `help`, `update` and `review` are in `utility` and in another.
#[test]
fn a_skill_in_two_clusters_is_hidden_by_either() {
    let utility = project("empty");
    utility.settings(json!({ "specforge.surface.utility": false }));
    assert_eq!(utility.install("claude-code", &[]).code, 0);
    for gone in ["gsd-help", "gsd-update", "gsd-review"] {
        assert!(
            !utility.exists(&format!(".claude/skills/{gone}/SKILL.md")),
            "{gone} is hidden by utility"
        );
    }
    let ideate = project("empty");
    ideate.settings(json!({ "specforge.surface.research_ideate": false }));
    assert_eq!(ideate.install("claude-code", &[]).code, 0);
    assert!(!ideate.exists(".claude/skills/gsd-ns-ideate/SKILL.md"));
    assert!(ideate.exists(".claude/skills/gsd-help/SKILL.md"));
}

/// The members of each cluster of the vendored `clusters.cts`, in its order.
fn upstream() -> Vec<(String, Vec<String>)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity/upstream/v1.18.0/src/clusters.cts");
    let text = std::fs::read_to_string(path).expect("the vendored clusters");
    let body = text
        .split("export const CLUSTERS")
        .nth(1)
        .expect("CLUSTERS");
    let body = body.split("export function").next().expect("its end");
    let mut found: Vec<(String, Vec<String>)> = Vec::new();
    for line in body.lines().map(str::trim) {
        if let Some(name) = line.strip_suffix(": Object.freeze([") {
            found.push((name.to_string(), Vec::new()));
        } else if let Some(stem) = line.strip_prefix('\'').and_then(|l| l.strip_suffix("',")) {
            found
                .last_mut()
                .expect("a cluster")
                .1
                .push(stem.to_string());
        }
    }
    found
}

#[test]
fn clusters_match_upstream() {
    let ported: Vec<(String, Vec<String>)> = CLUSTERS
        .iter()
        .map(|(name, members)| {
            (
                name.to_string(),
                members.iter().map(|m| m.to_string()).collect(),
            )
        })
        .collect();
    let upstream = upstream();
    assert_eq!(upstream.len(), 10);
    assert_eq!(ported, upstream);
}

#[test]
fn help_lists_fifty_slash_commands() {
    let p = project("empty");
    let found = json(&gsd(&p, "help", &["--format", "json"]));
    let slashes = found["slash_commands"].as_array().expect("a list");
    assert_eq!(slashes.len(), 50, "{slashes:?}");
}
