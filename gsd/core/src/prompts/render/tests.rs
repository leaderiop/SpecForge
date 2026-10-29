use super::*;
use crate::prompts::preamble::{self, FILES, MARKER};

const EMBEDDED: &str = "references/verification-patterns.md";

fn input<'a>(host: &'a str, arguments: &'a str) -> Input<'a> {
    Input {
        command: "x",
        host,
        arguments,
        settings: "{}",
        tail: "",
    }
}

#[test]
fn render_order_repoints_before_the_host_conversion() {
    let command =
        format!("Read @~/.claude/gsd-core/{EMBEDDED} and ~/.claude/gsd-core/{EMBEDDED}.\n");
    let out = assemble(&command, &input("codex", ""));
    assert!(
        out.contains(&format!("`specforge gsd reference gsd-core/{EMBEDDED}`")),
        "{out}"
    );
    assert!(
        !out.contains(".codex/gsd-core") && !out.contains(".claude/gsd-core"),
        "{out}"
    );
}

#[test]
fn the_execution_context_is_inlined_once_and_wrapped() {
    let include = format!("@~/.claude/gsd-core/{EMBEDDED}");
    let command = format!(
        "<execution_context>\n{include}\n{include}\n</execution_context>\nafter {include}\n"
    );
    let out = assemble(&command, &input("claude-code", ""));
    assert_eq!(
        out.matches("<file path=\"gsd-core/references/verification-patterns.md\">")
            .count(),
        1,
        "{out}"
    );
    assert!(
        out.contains("# Verification Patterns"),
        "the file's text is inlined"
    );
    assert!(
        !out.contains("Ported from"),
        "the attribution line is not text"
    );
    assert!(
        out.contains(&format!(
            "after `specforge gsd reference gsd-core/{EMBEDDED}`"
        )),
        "an include outside the context stays a reference: {out}"
    );
}

#[test]
fn arguments_replace_every_dollar_arguments() {
    let out = assemble(
        "Parse $ARGUMENTS for --auto; $ARGUMENTS_X stays.\n$ARGUMENTS",
        &input("claude-code", "02 --auto"),
    );
    assert!(
        out.contains("Parse 02 --auto for --auto; $ARGUMENTS_X stays.\n02 --auto"),
        "{out}"
    );
    let none = assemble("Parse $ARGUMENTS.", &input("claude-code", ""));
    assert!(none.contains("Parse (no arguments)."), "{none}");
}

#[test]
fn the_text_has_the_preamble_the_header_and_the_tail() {
    let mut i = input("codex", "02");
    i.tail = "TAIL";
    let out = assemble("# Heading\n", &i);
    assert!(out.starts_with("<codex_skill_adapter>"), "{out}");
    assert!(
        out.contains(MARKER) && out.contains("# Heading") && out.ends_with("TAIL"),
        "{out}"
    );
    let claude = assemble("# Heading\n", &input("claude-code", ""));
    assert!(claude.starts_with(MARKER), "{claude}");
    assert!(
        claude.contains("Task(subagent_type=\"gsd-<name>\""),
        "{claude}"
    );
    assert!(assemble("x", &input("cursor", "")).contains("as the adapter header above says"));
}

/// The rows of `spec/README.md`'s "Every GSD file maps to a kind" table whose extension is `core`.
fn readme_core_rows() -> Vec<(String, String)> {
    include_str!("../../../spec/README.md")
        .lines()
        .filter(|l| l.starts_with("| ") && l.ends_with("| core |"))
        .map(|l| {
            let cells: Vec<&str> = l.trim_matches('|').split(" | ").map(str::trim).collect();
            (cells[0].to_string(), cells[1].to_string())
        })
        .collect()
}

#[test]
fn the_preamble_table_matches_spec_readme() {
    let rows = readme_core_rows();
    assert!(rows.len() > 15, "{} core rows", rows.len());
    let ours: Vec<(String, String)> = FILES
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    assert_eq!(ours, rows);
    let text = preamble::text("claude-code", "{}");
    for forbidden in [".planning", "gsd-tools", "gsd_run", "gsd-sdk", "gsd.cjs"] {
        assert!(!text.contains(forbidden), "the preamble names {forbidden}");
    }
}

#[test]
fn the_preamble_settings_block_lists_the_gsd_settings() {
    let block = crate::settings::block_of(|name| match name {
        "workflow.discuss_mode" => Some(serde_json::json!("assumptions")),
        "workflow.research" => Some(serde_json::json!(true)),
        _ => None,
    });
    assert_eq!(block, format!("{{\n  {}\n}}", ordered(&block)));
    let parsed: serde_json::Value = serde_json::from_str(&block).expect("a JSON object");
    assert_eq!(parsed["workflow.discuss_mode"], "assumptions");
    assert_eq!(parsed.as_object().map(|o| o.len()), Some(2));
    assert!(preamble::text("claude-code", &block).contains(&block));
}

/// The two rows in the roster's order, whatever that is.
fn ordered(block: &str) -> String {
    let research = "\"workflow.research\": true";
    let mode = "\"workflow.discuss_mode\": \"assumptions\"";
    if block.find(research) < block.find(mode) {
        format!("{research},\n  {mode}")
    } else {
        format!("{mode},\n  {research}")
    }
}

#[test]
fn the_argument_text_rebuilds_upstreams_order() {
    use crate::prompts::args::{Arg, render_text};
    const ARGS: &[Arg] = &[
        Arg::reference("phase", "phase"),
        Arg::flag("auto"),
        Arg::flag("skip_research"),
        Arg::integer("wave"),
        Arg::one_of("granularity", &["coarse", "standard", "fine"]),
        Arg::text("instructions"),
    ];
    let values = |name: &str| match name {
        "phase" => Some(serde_json::json!("v1.0/02")),
        "skip_research" => Some(serde_json::json!(true)),
        "auto" => Some(serde_json::json!(false)),
        "wave" => Some(serde_json::json!(2)),
        "granularity" => Some(serde_json::json!("fine")),
        "instructions" => Some(serde_json::json!("cover \"the\" empty list")),
        _ => None,
    };
    assert_eq!(
        render_text(ARGS, values),
        "02 --skip-research --wave 2 --granularity fine --instructions \"cover \\\"the\\\" empty list\""
    );
    assert_eq!(render_text(ARGS, |_| None), "");
}
