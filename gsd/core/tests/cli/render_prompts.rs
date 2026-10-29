//! Each lifecycle prompt rendered for the three hosts (T12): one golden per prompt and host (D-r42-21: the payload,
//! the head of the text, its counts and its digest, never the whole text), every gate proved both ways, and the
//! listing of the slash commands.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::cmd_execute::stale_plan_check;
use super::commands_support::{gsd, json, plus, twin};
use super::harness::{Project, project, taskboard};
use super::prompts_support::{HOSTS, payload, prompt, text_of};

const PHASE: &str = "v1.0/02";
const PLAN: &str = "v1.0/02/02-01";

/// A prompt, what it is asked about and the project that makes it render.
struct Row {
    id: &'static str,
    args: &'static [&'static str],
    setup: fn() -> Project,
}

fn discuss() -> Project {
    project("discuss")
}
fn empty() -> Project {
    project("empty")
}
fn shipped() -> Project {
    let p = taskboard();
    assert!(p.verify_phase("v1.0/01") > 0);
    p
}

const ROWS: [Row; 28] = [
    Row {
        id: "new_project",
        args: &["--auto"],
        setup: empty,
    },
    Row {
        id: "new_milestone",
        args: &["--name", "Next"],
        setup: empty,
    },
    Row {
        id: "discuss_phase",
        args: &[PHASE],
        setup: discuss,
    },
    Row {
        id: "spec_phase",
        args: &[PHASE],
        setup: discuss,
    },
    Row {
        id: "mvp_phase",
        args: &[PHASE],
        setup: twin,
    },
    Row {
        id: "plan_phase",
        args: &[PHASE],
        setup: discuss,
    },
    Row {
        id: "plan_review_convergence",
        args: &[PHASE],
        setup: twin,
    },
    Row {
        id: "ultraplan_phase",
        args: &[PHASE],
        setup: discuss,
    },
    Row {
        id: "execute_phase",
        args: &[PHASE],
        setup: twin,
    },
    Row {
        id: "execute_plan",
        args: &[PLAN],
        setup: twin,
    },
    Row {
        id: "verify_work",
        args: &[PHASE],
        setup: twin,
    },
    Row {
        id: "validate_phase",
        args: &[PHASE],
        setup: twin,
    },
    Row {
        id: "add_tests",
        args: &[PHASE],
        setup: twin,
    },
    Row {
        id: "audit_uat",
        args: &[],
        setup: twin,
    },
    Row {
        id: "audit_milestone",
        args: &["v1.0"],
        setup: twin,
    },
    Row {
        id: "complete_milestone",
        args: &["v1.0"],
        setup: super::cmd_milestone::ready,
    },
    Row {
        id: "ship",
        args: &["v1.0/01"],
        setup: shipped,
    },
    Row {
        id: "next",
        args: &[],
        setup: twin,
    },
    Row {
        id: "progress",
        args: &["--next", "--auto"],
        setup: twin,
    },
    Row {
        id: "manager",
        args: &[],
        setup: twin,
    },
    Row {
        id: "quick",
        args: &["--task", "trim titles"],
        setup: twin,
    },
    Row {
        id: "quick_batch",
        args: &["--tasks", "a; b"],
        setup: twin,
    },
    Row {
        id: "fast",
        args: &["--task", "fix the typo"],
        setup: twin,
    },
    Row {
        id: "autonomous",
        args: &["--from", "01", "--to", "02", "--converge"],
        setup: twin,
    },
    Row {
        id: "pause_work",
        args: &["--report"],
        setup: twin,
    },
    Row {
        id: "resume_work",
        args: &[],
        setup: twin,
    },
    Row {
        id: "review",
        args: &[PHASE, "--codex"],
        setup: twin,
    },
    Row {
        id: "review_backlog",
        args: &[],
        setup: twin,
    },
];

fn golden(host: &str, id: &str, actual: &Value) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/prompts")
        .join(host);
    let path = dir.join(format!("{id}.json"));
    let text = serde_json::to_string_pretty(actual).expect("JSON prints") + "\n";
    if std::env::var_os("GSD_BLESS").is_some() {
        std::fs::create_dir_all(&dir).expect("a golden directory");
        std::fs::write(&path, &text).expect("write the golden");
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{} is missing: GSD_BLESS=1 writes it", path.display()));
    assert!(
        expected == text,
        "{host}/{id} differs: review the change, then GSD_BLESS=1"
    );
}

#[test]
fn render_goldens() {
    for row in &ROWS {
        let p = (row.setup)();
        for host in HOSTS {
            let rendered = payload(&prompt(&p, row.id, row.args, host));
            let text = text_of(&rendered);
            let head: Vec<&str> = text.lines().take(80).collect();
            let digest = format!("{:x}", Sha256::digest(text.as_bytes()));
            golden(
                host,
                row.id,
                &json!({
                    "payload": rendered["payload"],
                    "head": head,
                    "lines": text.lines().count(),
                    "bytes": text.len(),
                    "sha256": digest,
                }),
            );
        }
    }
}

/// The gated prompts that render on the twin: under `gsd-plus` a stale verdict inside the subject refuses (E516),
/// under the default preset the same project renders. The other guards have their own tests (`prompts_*`).
#[test]
fn every_gated_prompt_refuses_and_allows() {
    let gated = [
        "mvp_phase",
        "plan_phase",
        "plan_review_convergence",
        "ultraplan_phase",
        "execute_phase",
        "execute_plan",
        "verify_work",
        "validate_phase",
        "add_tests",
        "audit_milestone",
    ];
    for id in gated {
        let row = ROWS.iter().find(|r| r.id == id).expect("a row");
        let p = (row.setup)();
        // The `discuss` project is not the twin: the stale verdict is planted on the twin's plan.
        let p = if id == "plan_phase" || id == "ultraplan_phase" {
            twin()
        } else {
            p
        };
        stale_plan_check(&p);
        plus(&p);
        let run = prompt(&p, id, row.args, "claude-code");
        assert_ne!(run.code, 0, "{id}: {}", run.all());
        assert!(run.all().contains("E516"), "{id}: {}", run.all());
        p.settings(json!({}));
        let rendered = payload(&prompt(&p, id, row.args, "claude-code"));
        assert!(
            rendered["payload"]["prompt"]
                .as_str()
                .is_some_and(|s| s.ends_with(id)),
            "{id}"
        );
    }
}

#[test]
fn help_lists_the_prompt_slash_commands() {
    let p = twin();
    let found = json(&gsd(&p, "help", &["--format", "json"]));
    let listed: Vec<&str> = found["slash_commands"]
        .as_array()
        .expect("slash commands")
        .iter()
        .filter_map(|s| s["slash"].as_str())
        .collect();
    for row in ROWS.iter().filter(|r| r.id != "execute_plan") {
        let slash = format!("/gsd-{}", row.id.replace('_', "-"));
        assert!(
            listed.contains(&slash.as_str()),
            "{slash} is not listed: {listed:?}"
        );
    }
}
