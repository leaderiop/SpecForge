use serde_json::json;

use super::*;

fn manifest(loaded: &[&str]) -> Manifest {
    Manifest {
        spec_root: "spec".to_string(),
        security: loaded.contains(&"@gsd/security"),
        loaded: loaded.iter().map(|s| s.to_string()).collect(),
    }
}

/// The lines and `(key, outcome, reason)` of each entry of `config`.
fn run(config: Value, loaded: &[&str]) -> (Vec<String>, Vec<(String, Outcome, String)>) {
    let (lines, entries) = settings_lines(&config.to_string(), &manifest(loaded));
    let entries = entries
        .into_iter()
        .filter(|e| e.section != "worktree.baseRef")
        .map(|e| (e.section, e.outcome, e.reason))
        .collect();
    (lines, entries)
}

#[test]
fn settings_lines_quote_each_value_for_a_shell() {
    assert_eq!(shell_quote("plain"), "'plain'");
    assert_eq!(shell_quote("'; rm -rf ~ #"), "''\\''; rm -rf ~ #'");
    assert_eq!(shell_quote("$(id)"), "'$(id)'");
    let (lines, entries) = run(
        json!({
            "mode": "'; rm -rf ~ #",
            "phase_naming": "$(id)",
            "parallelization": false,
            "workflow": {"node_repair_budget": 2, "research": true},
            "agent_skills": {"gsd-executor": ["a b", "it's"]},
            "ship": {"pr_body_sections": []},
        }),
        &[],
    );
    assert_eq!(
        lines,
        [
            "specforge settings set @gsd/core agent_skills.gsd-executor 'a b' 'it'\\''s'",
            "specforge settings set @gsd/core mode ''\\''; rm -rf ~ #'",
            "specforge settings set @gsd/core parallelization false",
            "specforge settings set @gsd/core phase_naming '$(id)'",
            "specforge settings set @gsd/core ship.pr_body_sections --value-json '[]'",
            "specforge settings set @gsd/core workflow.node_repair_budget 2",
            "specforge settings set @gsd/core workflow.research true",
        ]
    );
    assert!(entries.is_empty(), "{entries:?}");
}

#[test]
fn settings_lines_refuse_a_control_character() {
    let (lines, entries) = run(
        json!({"mode": "yolo\nrm -rf ~", "granularity": "fine\u{7}", "models": {"plan": ["a\u{202e}"]}}),
        &[],
    );
    assert!(lines.is_empty(), "{lines:?}");
    let reasons: Vec<&str> = entries.iter().map(|(_, _, r)| r.as_str()).collect();
    assert_eq!(
        reasons,
        [
            "control character in the value of granularity",
            "control character in the value of mode",
            "control character in the value of models.plan",
        ]
    );
    assert!(entries.iter().all(|(_, o, _)| *o == Outcome::Unmapped));
}

#[test]
fn settings_lines_never_print_a_secret() {
    let (lines, entries) = run(
        json!({"brave_search": "sk-SECRET-1", "jina": "jina-SECRET-2", "exa_search": false}),
        &[],
    );
    assert_eq!(
        lines,
        [
            "specforge settings set @gsd/core brave_search true",
            "specforge settings set @gsd/core exa_search false",
            "specforge settings set @gsd/core jina true",
        ]
    );
    assert_eq!(entries.len(), 2);
    assert!(entries[0].2.contains("BRAVE_API_KEY"), "{entries:?}");
    assert!(entries[1].2.contains("JINA_API_KEY"), "{entries:?}");
    let all = format!("{lines:?}{entries:?}");
    assert!(!all.contains("SECRET"), "{all}");
}

#[test]
fn settings_lines_report_what_has_no_line() {
    let (lines, entries) = run(
        json!({
            "project_code": null,
            "commit_docs": true,
            "search_gitignored": false,
            "claude_md_path": "./.claude/CLAUDE.md",
            "workflow": {"auto_prune_state": false},
            "git": {"repo_tagging": true},
            "made_up": 1,
            "a;rm": {"b": 1},
        }),
        &[],
    );
    assert!(lines.is_empty(), "{lines:?}");
    let got: Vec<(&str, Outcome)> = entries.iter().map(|(k, o, _)| (k.as_str(), *o)).collect();
    assert_eq!(
        got,
        [
            ("a;rm.b", Outcome::Unmapped),
            ("claude_md_path", Outcome::Unmapped),
            ("commit_docs", Outcome::Unmapped),
            ("git.repo_tagging", Outcome::Unmapped),
            ("made_up", Outcome::Unmapped),
            ("project_code", Outcome::Ignored),
            ("search_gitignored", Outcome::Unmapped),
            ("workflow.auto_prune_state", Outcome::Unmapped),
        ]
    );
    let reason = |key: &str| {
        entries
            .iter()
            .find(|(k, ..)| k == key)
            .map(|(.., r)| r.as_str())
            .expect("an entry")
    };
    assert_eq!(reason("commit_docs"), NO_PLANNING);
    assert_eq!(reason("claude_md_path"), HOST_FILES);
    assert_eq!(
        reason("workflow.auto_prune_state"),
        "STATE.md is derived, nothing to prune"
    );
    assert_eq!(reason("made_up"), "not a gsd-core 1.19 setting");
    assert_eq!(reason("a;rm.b"), "not a setting name");
}

#[test]
fn settings_lines_map_gsds_loose_shapes() {
    let (lines, entries) = run(
        json!({
            "resolve_model_ids": "omit",
            "agent_tools": {"*": ["Read"], "gsd-planner": ["Bash"]},
            "model_profile_overrides": {
                "claude": {"opus": "claude-opus-4", "haiku": {"model": "h", "reasoning_effort": "low"}}
            },
        }),
        &[],
    );
    assert_eq!(
        lines,
        [
            "specforge settings set @gsd/core agent_tools.all 'Read'",
            "specforge settings set @gsd/core agent_tools.gsd-planner 'Bash'",
            "specforge settings set @gsd/core model_profile_overrides.claude.haiku --value-json '{\"model\":\"h\",\"reasoning_effort\":\"low\"}'",
            "specforge settings set @gsd/core model_profile_overrides.claude.opus --value-json '{\"model\":\"claude-opus-4\"}'",
            "specforge settings set @gsd/core resolve_model_ids omit",
        ]
    );
    assert!(entries.is_empty(), "{entries:?}");
    for (value, word) in [(false, "aliases"), (true, "ids")] {
        let (lines, _) = run(json!({ "resolve_model_ids": value }), &[]);
        assert_eq!(
            lines,
            [format!(
                "specforge settings set @gsd/core resolve_model_ids {word}"
            )]
        );
    }
}

#[test]
fn settings_lines_need_the_owning_extension() {
    let config = json!({
        "workflow": {
            "security_asvs_level": 1,
            "ui_phase": true,
            "ai_integration_phase": true,
            "code_review_depth": "standard",
            "drift_action": "warn",
        },
        "security": {"injection_blocking": false},
    });
    let (lines, entries) = run(config.clone(), &[]);
    assert!(lines.is_empty(), "{lines:?}");
    assert_eq!(entries.len(), 6);
    assert!(
        entries
            .iter()
            .all(|(_, o, _)| *o == Outcome::NeedsExtension)
    );
    let (lines, entries) = run(config, &["@gsd/core", "@gsd/security", "@gsd/ops"]);
    assert_eq!(
        lines,
        [
            "specforge settings set @gsd/security security.injection_blocking false",
            "specforge settings set @gsd/ops workflow.code_review_depth 'standard'",
            "specforge settings set @gsd/security workflow.security_asvs_level 1",
        ]
    );
    let left: Vec<&str> = entries.iter().map(|(k, ..)| k.as_str()).collect();
    assert_eq!(
        left,
        [
            "workflow.ai_integration_phase",
            "workflow.drift_action",
            "workflow.ui_phase"
        ]
    );
}

#[test]
fn settings_lines_report_a_file_that_is_no_object() {
    let (lines, entries) = settings_lines("[1]", &manifest(&[]));
    assert!(lines.is_empty());
    assert_eq!(entries[0].reason, "not a JSON object");
    let (_, entries) = settings_lines("{", &manifest(&[]));
    assert!(entries[0].reason.starts_with("not JSON"));
    // worktree.baseRef is reported once, whatever the file holds.
    assert_eq!(entries[1].section, "worktree.baseRef");
}

/// `OWNERS` is settings.md's owner table, token for token.
#[test]
fn settings_owners_are_the_specs_table() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/spec/settings.md"
    ))
    .expect("settings.md");
    let table = gsd_kit::markdown::tables(&text)
        .into_iter()
        .find(|t| t.header.first().is_some_and(|h| h == "Owner"))
        .expect("the owner table");
    let spans = |cell: &str| -> Vec<String> {
        cell.split('`')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect()
    };
    let read: Vec<(String, Vec<String>)> = table
        .rows
        .iter()
        .map(|row| (spans(&row[0]).concat(), spans(&row[1])))
        .collect();
    let ours: Vec<(String, Vec<String>)> = OWNERS
        .iter()
        .map(|(owner, tokens)| {
            (
                owner.to_string(),
                tokens.iter().map(|t| t.to_string()).collect(),
            )
        })
        .collect();
    assert_eq!(ours, read);
}
