//! `quick`, `quick_batch` and `fast` (T2): a quick task is a plan with no phase, made, started and completed through
//! the typed tools the prompt names.

use super::commands_support::{gsd, twin};
use super::prompts_support::{command, payload, prompt, render_three_hosts, text_of};

/// Today's quick id, `<yymmdd>-<3 base-36>`.
fn quick_id() -> String {
    let day = std::process::Command::new("date")
        .args(["-u", "+%y%m%d"])
        .output()
        .expect("date");
    format!("{}-a1b", String::from_utf8_lossy(&day.stdout).trim())
}

fn run(p: &super::harness::Project, args: &[String]) -> super::harness::Run {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    p.run(&refs)
}

#[test]
fn quick_creates_a_quick_plan() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "quick",
        &["--task", "trim titles"],
        "claude-code",
    )));
    let id = quick_id();
    p.write("src/tasks.js", "export const add = (t) => t.trim();\n");
    let fill = [
        ("<title>", "Trim titles"),
        ("<paths>", "src/tasks.js"),
        ("<objective>", "Titles are trimmed"),
        ("<what must be true>", "A padded title is trimmed"),
        ("<file>", "src/tasks.js"),
        ("<name>", "Task store"),
        ("<what it provides>", "add() that trims"),
        ("<one line>", "Trimmed"),
        ("<area>", "tasks"),
    ];
    for prefix in ["specforge new plan", "specforge new truth"] {
        let args = command(&text, prefix, &id, &fill);
        let made = run(&p, &args);
        assert_eq!(made.code, 0, "{args:?}: {}", made.all());
    }
    // The artifact goes through `specforge.edit`; the file is what that tool writes.
    let quick = p.read("spec/quick.spec");
    p.write(
        "spec/quick.spec",
        &format!(
            "{quick}\nartifact AR1 \"Task store\" {{\n  plan     {id}\n  path     \"src/tasks.js\"\n  provides \"add() that trims\"\n}}\n"
        ),
    );
    let codes = p.codes();
    for code in ["W323", "E302", "E401"] {
        assert!(!codes.contains(&code.to_string()), "{code}: {codes:?}");
    }
    // P4: a plan with no phase starts and completes through the typed tools.
    let started = gsd(&p, "start-plan", &[&id]);
    assert_eq!(started.code, 0, "{}", started.all());
    let ts = started
        .stdout
        .split("started ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .expect("a time")
        .to_string();
    let summary = command(&text, "specforge new summary", &id, &fill);
    let made = run(&p, &summary);
    assert_eq!(made.code, 0, "{summary:?}: {}", made.all());
    let done = gsd(
        &p,
        "complete-plan",
        &[&format!("{id}/{id}-SUMMARY"), "--started", &ts],
    );
    assert_eq!(done.code, 0, "{}", done.all());
    let fields = p.fields_of(&format!("{id}/{id}-SUMMARY"));
    assert_eq!(fields["started"], ts.as_str());
    assert!(fields["completed"].as_str().is_some(), "{fields}");
}

#[test]
fn quick_renders_for_three_hosts() {
    let args = [
        "--full",
        "--validate",
        "--discuss",
        "--research",
        "--task",
        "trim titles",
    ];
    render_three_hosts(
        &twin(),
        "quick",
        &args,
        "--full --validate --discuss --research --task \"trim titles\"",
        "<purpose>",
    );
}

#[test]
fn quick_batch_renders_for_three_hosts() {
    let out = render_three_hosts(
        &twin(),
        "quick_batch",
        &["--tasks", "a; b", "--jobs", "2"],
        "--jobs \"2\" --tasks \"a; b\"",
        "<purpose>",
    );
    assert!(text_of(&out[0].1).contains("The quick plan"));
}

#[test]
fn fast_renders_for_three_hosts() {
    let out = render_three_hosts(
        &twin(),
        "fast",
        &["--task", "fix the typo"],
        "--task \"fix the typo\"",
        "<purpose>",
    );
    assert!(
        out[0].1["payload"]["steps"]
            .as_array()
            .is_none_or(|s| s.is_empty())
    );
}

#[test]
fn quick_refuses_a_bare_task_word() {
    // The K19 tail: a free-text task is one quoted option, so a bare word is not an argument.
    let run = prompt(&twin(), "quick", &["fix", "the", "bug"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("invalid_input"), "{}", run.all());
}

#[test]
fn quick_steps_name_the_judges_and_record_verdict() {
    let rendered = payload(&prompt(&twin(), "quick", &[], "claude-code"));
    let steps = rendered["payload"]["steps"].as_array().expect("steps");
    let ids: Vec<&str> = steps
        .iter()
        .map(|s| s["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        ids,
        [
            "research",
            "plan",
            "check",
            "record_verdict",
            "start",
            "execute",
            "complete",
            "verify"
        ]
    );
    let cli = |id: &str| {
        steps
            .iter()
            .find(|s| s["id"] == id)
            .and_then(|s| s["cli"].as_str())
    };
    assert_eq!(cli("record_verdict"), Some("specforge gsd record-verdict"));
    assert_eq!(cli("start"), Some("specforge gsd start-plan"));
    assert_eq!(cli("complete"), Some("specforge gsd complete-plan"));
}

#[test]
fn quick_skips_the_code_reviewer() {
    let rendered = payload(&prompt(&twin(), "quick", &["--full"], "claude-code"));
    let text = text_of(&rendered);
    assert!(text.contains("gsd-code-reviewer"), "{text}");
    assert!(text.contains("skip the code review"), "{text}");
    let steps = rendered["payload"]["steps"].as_array().expect("steps");
    assert!(steps.iter().all(|s| s["id"] != "code_review"));
}

/// REVIEW-r43 M12: each batch item gets its own quick id (`<yymmdd>-<3 base-36>`, what `gsd/W323` checks) and keeps its
/// task text without the list marker; a batch of fewer than two items is refused.
#[test]
fn the_batch_makes_one_quick_id_per_item() {
    // the step file is read on demand by the agent, so it is not part of the render: read the asset itself
    let step = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/gsd-core/workflows/quick-batch/steps/batch-init.md"
    ))
    .expect("the batch-init step");
    let line = step
        .lines()
        .find(|l| l.starts_with("QB_ITEMS="))
        .expect("the batch-init line")
        .to_string();
    let run = |items: &str| {
        let dir = tempfile::tempdir().expect("a dir");
        std::fs::write(dir.path().join("tasks.txt"), items).expect("write");
        std::process::Command::new("bash")
            .args([
                "-c",
                &format!("TASK_FILE=tasks.txt\n{line}\necho \"$QB_CREATE_JSON\""),
            ])
            .current_dir(dir.path())
            .output()
            .expect("bash")
    };
    let out = run("- Fix login\n* Add search\n\n1. Trim titles\n");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)));
    let items = json["manifest"]["items"].as_array().expect("items");
    let tasks: Vec<&str> = items
        .iter()
        .map(|i| i["task"].as_str().expect("a task"))
        .collect();
    assert_eq!(tasks, ["Fix login", "Add search", "Trim titles"]);
    let id = regex::Regex::new(r"^[0-9]{6}-[a-z0-9]{3}$").expect("a regex");
    for item in items {
        assert!(
            id.is_match(item["quickId"].as_str().expect("an id")),
            "{item}"
        );
    }
    let one = run("- Only one\n");
    assert!(
        String::from_utf8_lossy(&one.stdout).trim().is_empty(),
        "{one:?}"
    );
}

/// REVIEW-r43 M7: the typed arguments render as options (`--list`, `--status "abc"`), so the dispatch reads options, and
/// `--task` / `--tasks` are named as the description and the task list.
#[test]
fn quick_dispatches_on_the_option_forms() {
    let p = twin();
    for (args, shown) in [
        (vec!["--list"], "<arguments>--list</arguments>"),
        (
            vec!["--status", "abc"],
            "<arguments>--status \"abc\"</arguments>",
        ),
        (
            vec!["--resume", "abc"],
            "<arguments>--resume \"abc\"</arguments>",
        ),
        (
            vec!["--task", "trim"],
            "<arguments>--task \"trim\"</arguments>",
        ),
    ] {
        let text = text_of(&payload(&prompt(&p, "quick", &args, "claude-code")));
        assert!(text.contains(shown), "{args:?}: no {shown}");
        for word in [
            "holds `--list`",
            "holds `--status \"<slug>\"`",
            "holds `--resume \"<slug>\"`",
            "`--task \"...\"` carries the description",
            "`--task \"<text>\"` → use its value as `$DESCRIPTION`",
        ] {
            assert!(text.contains(word), "{args:?}: no {word}");
        }
        assert!(!text.contains("starts with \"list\""));
    }
    let batch = text_of(&payload(&prompt(
        &p,
        "quick_batch",
        &["--tasks", "a; b"],
        "claude-code",
    )));
    assert!(batch.contains("the value of `--tasks \"<text>\"`"));
    let fast = text_of(&payload(&prompt(
        &p,
        "fast",
        &["--task", "x"],
        "claude-code",
    )));
    assert!(fast.contains("the value of `--task \"<text>\"`"));
}
