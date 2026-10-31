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
        "trim titles",
    ];
    render_three_hosts(
        &twin(),
        "quick",
        &args,
        "trim titles --full --validate --discuss --research",
        "<purpose>",
    );
}

#[test]
fn quick_batch_renders_for_three_hosts() {
    let out = render_three_hosts(
        &twin(),
        "quick_batch",
        &["--jobs", "2", "a; b"],
        "--jobs \"2\" a; b",
        "<purpose>",
    );
    assert!(text_of(&out[0].1).contains("The quick plan"));
}

#[test]
fn fast_renders_for_three_hosts() {
    let out = render_three_hosts(
        &twin(),
        "fast",
        &["fix", "the", "typo"],
        "fix the typo",
        "<purpose>",
    );
    assert!(
        out[0].1["payload"]["steps"]
            .as_array()
            .is_none_or(|s| s.is_empty())
    );
}

#[test]
fn quick_takes_its_task_bare() {
    // The free-text tail (ADR 0314): the words after the options are the task, as upstream's hint says.
    render_three_hosts(
        &twin(),
        "quick",
        &["fix", "the", "login", "bug"],
        "<arguments>fix the login bug</arguments>",
        "<purpose>",
    );
    render_three_hosts(
        &twin(),
        "quick",
        &["--full", "fix", "it"],
        "<arguments>fix it --full</arguments>",
        "<purpose>",
    );
    // by name it is the same tail
    render_three_hosts(
        &twin(),
        "quick",
        &["--task", "fix it"],
        "<arguments>fix it</arguments>",
        "<purpose>",
    );
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
    assert_eq!(
        cli("record_verdict"),
        Some("specforge gsd record-verdict --host claude-code")
    );
    assert_eq!(
        cli("start"),
        Some("specforge gsd start-plan --host claude-code")
    );
    assert_eq!(
        cli("complete"),
        Some("specforge gsd complete-plan --host claude-code")
    );
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
        .map(|i| i["description"].as_str().expect("a description"))
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

/// The subcommands are words, as upstream's hint says: the tail carries `list`, `status <slug>` and `resume <slug>`, and
/// the command file dispatches on what the block starts with.
#[test]
fn quick_dispatches_on_the_subcommand_words() {
    let p = twin();
    for (args, shown) in [
        (vec!["list"], "<arguments>list</arguments>"),
        (vec!["status", "abc"], "<arguments>status abc</arguments>"),
        (vec!["resume", "abc"], "<arguments>resume abc</arguments>"),
        (vec!["trim", "it"], "<arguments>trim it</arguments>"),
    ] {
        let text = text_of(&payload(&prompt(&p, "quick", &args, "claude-code")));
        assert!(text.contains(shown), "{args:?}: no {shown}");
        for word in [
            "If the `<arguments>` block starts with \"list\": SUBCMD=list",
            "starts with \"status \"",
            "starts with \"resume \"",
            "Remaining text → use as `$DESCRIPTION`",
        ] {
            assert!(text.contains(word), "{args:?}: no {word}");
        }
        assert!(!text.contains("holds `--list`"));
    }
    let run = prompt(&p, "quick", &["--list"], "claude-code");
    assert_ne!(run.code, 0, "list is a word, not an option: {}", run.all());
}

/// REVIEW-k20b m9: a flag does not move the subcommand words, so the block still starts with `resume`.
#[test]
fn quick_dispatches_on_a_subcommand_with_a_flag() {
    let p = twin();
    for (args, shown) in [
        (
            vec!["resume", "abc", "--research"],
            "<arguments>resume abc --research</arguments>",
        ),
        (
            vec!["status", "abc", "--full"],
            "<arguments>status abc --full</arguments>",
        ),
    ] {
        let text = text_of(&payload(&prompt(&p, "quick", &args, "claude-code")));
        assert!(text.contains(shown), "{args:?}: no {shown}");
    }
}

/// REVIEW-k20b m9: a task that names an option is text after the prompt's own `--` (the second one, the first being
/// the host's), and is refused without it.
#[test]
fn a_task_naming_an_option_is_taken_after_the_prompt_boundary() {
    let p = twin();
    let run = |words: &[&str]| prompt(&p, "quick", words, "claude-code");
    let text = text_of(&payload(&run(&["add", "a", "--", "--dry-run", "flag"])));
    assert!(
        text.contains("<arguments>add a --dry-run flag</arguments>"),
        "{text}"
    );
    let refused = run(&["add", "a", "--dry-run", "flag"]);
    assert_eq!(refused.code, 2, "{}", refused.all());
    assert!(
        refused.stderr.contains("no such option"),
        "{}",
        refused.all()
    );
}

/// REVIEW-r43 Minor 7: the quick id is made once (the init block's), and a `git.quick_branch_template` has its `{num}`
/// expanded to it (`{slug}` is the one the agent writes later).
#[test]
fn the_quick_id_is_made_once_and_the_branch_template_is_expanded() {
    let p = twin();
    p.settings(serde_json::json!({ "git.quick_branch_template": "quick/{num}-{slug}" }));
    let text = text_of(&payload(&prompt(&p, "quick", &[], "claude-code")));
    assert_eq!(
        text.matches("quick_id=\"$(date").count(),
        1,
        "the quick id is generated twice"
    );
    let line = text
        .lines()
        .find(|l| l.starts_with("quick_id=\"$(date"))
        .expect("the init line");
    let out = std::process::Command::new("bash")
        .args(["-c", &format!("{line}\necho \"$quick_id $branch_name\"")])
        .output()
        .expect("bash");
    let said = String::from_utf8_lossy(&out.stdout);
    let (id, branch) = said.trim().split_once(' ').expect("two words");
    assert_eq!(branch, format!("quick/{id}-{{slug}}"), "{said}");
}

/// REVIEW-r43 post-fix N2: the steps read each item's `description`, and quick-batch has no init block to make the
/// quick id: its tail says `$quick_id` is the item's `quickId`.
#[test]
fn quick_batch_names_where_its_quick_id_comes_from() {
    let p = twin();
    let text = text_of(&payload(&prompt(&p, "quick_batch", &[], "claude-code")));
    assert!(text.contains("$quick_id is each manifest item's `quickId`"));
    assert!(!text.contains("init block above"));
    let quick = text_of(&payload(&prompt(&p, "quick", &[], "claude-code")));
    assert!(quick.contains("init block above"));
}

/// F8: quick's worktree steps run under bash in the project's git repository, where the quick plan is the entity
/// `plan <id>` in `spec/quick.spec` and no `<id>-PLAN.md` exists.
mod worktree {
    use std::path::Path;
    use std::process::{Command, Output};

    use super::super::commands_support::{gsd, twin};
    use super::super::harness::{Project, project};
    use super::super::prompts_support::{payload, prompt, text_of};

    const ID: &str = "251031-abc";
    const AT: &str = "2026-10-31T08:00:00Z";
    const PLAN: &str = "plan 251031-abc \"Trim titles\" {\n  objective \"Titles are trimmed\"\n}\n";

    fn no_bash() -> bool {
        if Command::new("bash").arg("--version").output().is_ok() {
            return false;
        }
        eprintln!("skipped: bash is absent");
        true
    }

    /// `script` under bash in `dir`, with git's identity pinned and none of the developer's configuration.
    fn bash(dir: &Path, script: &str, env: &[(&str, &str)]) -> Output {
        Command::new("bash")
            .args(["-c", script])
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .envs([
                ("GIT_AUTHOR_NAME", "gsd"),
                ("GIT_AUTHOR_EMAIL", "gsd@example.com"),
                ("GIT_COMMITTER_NAME", "gsd"),
                ("GIT_COMMITTER_EMAIL", "gsd@example.com"),
            ])
            .envs(env.iter().copied())
            .output()
            .expect("bash")
    }

    /// The served pre-dispatch step's bash block.
    fn pre_dispatch(p: &Project) -> String {
        let served = gsd(
            p,
            "reference",
            &["gsd-core/workflows/quick/steps/worktree-pre-dispatch-commit.md"],
        );
        assert_eq!(served.code, 0, "{}", served.all());
        served
            .stdout
            .split("```bash\n")
            .nth(1)
            .and_then(|b| b.split("\n```").next())
            .expect("the pre-dispatch block")
            .to_string()
    }

    /// Runs the pre-dispatch block in `p` and answers `(QUICK_PLAN_COMMIT, QUICK_PLAN_PARENT)`.
    fn dispatch(p: &Project, block: &str) -> (String, String) {
        let out = bash(
            p.root(),
            &format!(
                "{block}\necho \"commit=$QUICK_PLAN_COMMIT\"\necho \"parent=$QUICK_PLAN_PARENT\""
            ),
            &[
                ("USE_WORKTREES", "true"),
                ("quick_id", ID),
                ("QUICK_DIR", ID),
                ("DESCRIPTION", "trim titles"),
                ("DISCUSS_MODE", "false"),
                ("RESEARCH_MODE", "false"),
            ],
        );
        let said = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{said}{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let value = |key: &str| {
            said.lines()
                .find_map(|l| l.strip_prefix(key))
                .unwrap_or_default()
                .to_string()
        };
        (value("commit="), value("parent="))
    }

    fn rev(p: &Project, spec: &str) -> String {
        p.git(&["rev-parse", spec], AT).trim().to_string()
    }

    #[test]
    #[ignore = "F8: plan 02 T2"]
    fn the_pre_dispatch_commit_holds_the_plan_and_the_base_is_its_parent() {
        if no_bash() {
            return;
        }
        let p = project("empty");
        let prior = p.git_commit("chore: start", &[("README.md", "taskboard\n")], AT);
        let block = pre_dispatch(&p);
        p.write("spec/quick.spec", PLAN);
        let (commit, parent) = dispatch(&p, &block);
        assert_eq!(
            p.git(&["log", "-1", "--format=%s"], AT).trim(),
            format!("docs({ID}): pre-dispatch artifacts for trim titles"),
            "the plan is not committed before dispatch"
        );
        assert_eq!(commit, rev(&p, "HEAD"));
        assert_eq!(
            parent, prior,
            "the alternate base is the plan commit's parent"
        );
        // Already committed, with an unrelated commit on top: no new commit, and the base is still the plan's.
        let plan_at = rev(&p, "HEAD");
        let top = p.git_commit("chore: unrelated", &[("README.md", "taskboard 2\n")], AT);
        let (commit, parent) = dispatch(&p, &block);
        assert_eq!(rev(&p, "HEAD"), top, "a re-run made a commit");
        assert_eq!(commit, top);
        assert_eq!(parent, prior, "not HEAD^ ({plan_at})");
    }

    #[test]
    #[ignore = "F8: plan 02 T2"]
    fn without_a_plan_there_is_no_alternate_base() {
        if no_bash() {
            return;
        }
        let p = project("empty");
        p.git_commit("chore: start", &[("README.md", "taskboard\n")], AT);
        p.git_commit(
            "docs: another plan",
            &[("spec/quick.spec", "plan 251031-xyz \"Other\" {\n}\n")],
            AT,
        );
        let (_, parent) = dispatch(&p, &pre_dispatch(&p));
        assert_eq!(parent, "", "a base was found for a plan no commit wrote");
    }

    #[test]
    #[ignore = "F8: plan 02 T2"]
    fn the_executor_finds_the_plan_in_its_worktree_and_stops_without_it() {
        if no_bash() {
            return;
        }
        let text = text_of(&payload(&prompt(&twin(), "quick", &[], "claude-code")));
        let lines: Vec<&str> = text.lines().collect();
        let first = lines
            .iter()
            .position(|l| l.starts_with("FIRST ACTION after the worktree branch check"))
            .expect("the executor's first action");
        let open = first
            + lines[first..]
                .iter()
                .position(|l| *l == "\\`\\`\\`bash")
                .expect("its block");
        let block = lines[open + 1..]
            .iter()
            .take_while(|l| !l.starts_with("${DISCUSS_MODE"))
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        let p = project("empty");
        p.git_commit("chore: start", &[("README.md", "taskboard\n")], AT);
        let with = p.git_commit("docs: the plan", &[("spec/quick.spec", PLAN)], AT);
        let trees = tempfile::tempdir().expect("a temp dir");
        let tree = trees.path().join("agent");
        let tree_arg = tree.to_string_lossy().into_owned();
        p.git(
            &["worktree", "add", "-q", "-b", "agent", &tree_arg, &with],
            AT,
        );
        let first_action = |commit: &str| {
            bash(
                &tree,
                &block,
                &[
                    ("QUICK_PLAN_COMMIT", commit),
                    ("quick_id", ID),
                    ("QUICK_DIR", ID),
                ],
            )
        };
        let passed = first_action(&with);
        assert_eq!(
            passed.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&passed.stderr)
        );
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&tree)
            .output()
            .expect("git");
        assert_eq!(
            String::from_utf8_lossy(&status.stdout),
            "",
            "a file was written"
        );
        let without = p.git_commit("docs: no plan", &[("spec/quick.spec", "")], AT);
        let refused = first_action(&without);
        assert_eq!(refused.status.code(), Some(42), "{refused:?}");
    }
}
