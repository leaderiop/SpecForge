//! The GSD A/B protocol (`evaluation/gsd-ab/`) as files: its scripts' self-tests pass, and what it says about
//! itself holds.

use std::process::{Command, Stdio};

use super::harness::HOW_TO_BUILD;
use super::harness::ab::{ab_dir, ab_tasks, python};

/// Runs `script --self-test` and answers its standard output once it exits 0 saying `self-test: ok`.
fn passes(script: &str) -> String {
    let run = python(script, &["--self-test"]);
    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    assert!(
        run.status.success() && stdout.lines().any(|l| l == "self-test: ok"),
        "{script} --self-test: {stdout}{}",
        String::from_utf8_lossy(&run.stderr)
    );
    stdout
}

/// The scorer's self-test passes: it reads its synthetic runs back as files and pairs all of them, refuses an
/// unpaired run, refuses exactly fourteen bad run files, and reports H3.
#[test]
fn the_scorer_passes_its_self_test_pairing_twelve_runs_and_refusing_fourteen_run_files() {
    let stdout = passes("score.py");
    assert!(
        stdout.lines().any(|l| l == "B - A: 12 pairs over 4 tasks"),
        "{stdout}"
    );
    assert!(
        stdout
            .lines()
            .any(|l| l == "B - Bv1: 12 pairs over 4 tasks"),
        "{stdout}"
    );
    let count = |prefix: &str| stdout.lines().filter(|l| l.starts_with(prefix)).count();
    assert_eq!(
        (count("refused: "), count("refused run file: ")),
        (1, 14),
        "{stdout}"
    );
    assert!(count("H3: ") > 0, "{stdout}");
}

/// Every acceptance row is a command to run or a rubric to judge by, never both, never a bare string.
#[test]
fn every_acceptance_row_is_run_or_judged() {
    let tasks = ab_tasks();
    for task in tasks["tasks"].as_array().expect("tasks") {
        for row in task["acceptance"].as_array().expect("acceptance") {
            let has = |key: &str| row[key].as_str().is_some_and(|s| !s.is_empty());
            assert!(
                has("text") && (has("run") != has("judge")),
                "{}: {row}",
                task["id"]
            );
        }
    }
}

/// `seed.py --self-test` passes, its `spawns` cases among them.
#[test]
fn the_seeding_script_passes_its_self_test() {
    let stdout = passes("seed.py");
    assert!(
        stdout.lines().any(|l| l.starts_with("spawns: ")),
        "{stdout}"
    );
}

/// Claude Code's name for an MCP tool: `mcp__specforge__` and the name with every character outside
/// `[A-Za-z0-9_-]` as `_`.
fn claude_name(mcp: &str) -> String {
    let safe: String = mcp
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => c,
            _ => '_',
        })
        .collect();
    format!("mcp__specforge__{safe}")
}

/// The writing commands of `@gsd/core` and `@gsd/security` as the host serves them: the `specforge.gsd.*` MCP tools of a
/// copy that loads both whose annotations say they write, each with its CLI name. Core's are its `gsd help` writers.
fn writing_commands() -> Vec<(String, String)> {
    let p = super::harness::ab::seeded("vault-execute-01-02", "B", &["--unseeded"])
        .expect("a vault copy");
    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    ];
    let run = p.run_stdin(&["mcp", "."], &(messages.join("\n") + "\n"));
    let listed = run
        .stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .find(|m| m["id"] == 2)
        .unwrap_or_else(|| panic!("no tools/list answer: {}", run.all()));
    let mut out: Vec<(String, String)> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter(|t| t["annotations"]["readOnlyHint"] == false)
        .filter_map(|t| {
            t["name"]
                .as_str()?
                .strip_prefix("specforge.gsd.")
                .map(String::from)
        })
        .map(|name| (format!("specforge.gsd.{name}"), name.replace('_', "-")))
        .collect();
    out.sort();
    let help = p.run(&["gsd", "help", "--format", "json"]);
    let help: serde_json::Value = serde_json::from_str(&help.stdout).expect("help answers JSON");
    for command in help["commands"].as_array().expect("commands") {
        let name = command["command"].as_str().expect("a name");
        assert_eq!(
            command["writes"] == true,
            out.iter().any(|(_, cli)| cli == name),
            "{name}: core's writers are the MCP tools that write"
        );
    }
    assert!(
        out.iter().any(|(_, cli)| cli == "record-security-audit"),
        "security's writers are served: {out:?}"
    );
    out
}

/// `tasks.json#state_writes`, the state changes M7 stops at, is a checked copy: its MCP names and CLI lines are the
/// catalogue's mutating engine tools and the extensions' writing commands, and its files the host's file tools.
#[test]
fn the_state_writes_are_the_mutating_tools_and_the_writing_commands() {
    use specforge_extension_sdk::engine_tools::{ENGINE_TOOLS, EngineEffect};
    let tasks = ab_tasks();
    let listed = |key: &str| -> Vec<String> {
        let mut all: Vec<String> = tasks["state_writes"][key]
            .as_array()
            .unwrap_or_else(|| panic!("state_writes.{key}"))
            .iter()
            .map(|v| v.as_str().expect("a name").to_string())
            .collect();
        all.sort();
        all
    };
    let mutating: Vec<_> = ENGINE_TOOLS
        .iter()
        .filter(|t| t.effect == EngineEffect::Mutates)
        .collect();
    let commands = writing_commands();
    let mut mcp: Vec<String> = mutating
        .iter()
        .map(|t| claude_name(&t.mcp_name()))
        .chain(commands.iter().map(|(name, _)| claude_name(name)))
        .collect();
    mcp.sort();
    let mut cli: Vec<String> = mutating
        .iter()
        .flat_map(|t| t.cli.iter().map(|l| l.to_string()))
        .collect();
    cli.sort();
    let mut gsd: Vec<String> = commands.iter().map(|(_, c)| format!("gsd {c}")).collect();
    gsd.sort();
    assert_eq!(listed("mcp"), mcp);
    assert_eq!(listed("cli"), cli);
    assert_eq!(listed("commands"), gsd);
    assert_eq!(
        listed("files"),
        ["Edit", "MultiEdit", "NotebookEdit", "Write"]
    );
}

/// `git check-ignore`'s exit code for a run file: 0 when it is ignored.
fn run_files_ignored() -> i32 {
    Command::new("git")
        .args(["check-ignore", "-q", "evaluation/gsd-ab/runs/A/t/1.json"])
        .current_dir(ab_dir().join("../.."))
        .stdin(Stdio::null())
        .status()
        .expect("run git")
        .code()
        .unwrap_or(-1)
}

#[test]
fn run_files_are_ignored() {
    assert_eq!(run_files_ignored(), 0);
}

/// The corpus is version 2: arms A, B, B0 and Bv1, where Bv1 is arm B as the tag `gsd-ab-v1` froze it (B's tools,
/// configuration and drops); three ready fixtures whose paths exist, twelve tasks, each listed defect seeded in arm A
/// and arm B with at least one `b_signals` row, each row about a seeded defect, and no status `pending` left in it.
#[test]
fn the_corpus_is_version_2() {
    let tasks = ab_tasks();
    assert_eq!(tasks["version"], "gsd-ab-v2");
    let arms = tasks["arms"].as_array().expect("arms");
    let ids: Vec<&str> = arms.iter().filter_map(|a| a["id"].as_str()).collect();
    assert_eq!(ids, ["A", "B", "B0", "Bv1"]);
    let (b, v1) = (&arms[1], &arms[3]);
    assert_eq!(v1["tag"], "gsd-ab-v1");
    for key in ["config", "tools", "drop"] {
        assert_eq!(v1[key], b[key], "Bv1's {key} is B's");
    }
    let fixtures = tasks["fixtures"].as_array().expect("fixtures");
    assert_eq!(fixtures.len(), 3);
    for fixture in fixtures {
        assert_eq!(fixture["status"], "ready", "{fixture}");
        let path = fixture["path"].as_str().expect("a path");
        assert!(ab_dir().join("../..").join(path).is_dir(), "{path}");
    }
    let all = tasks["tasks"].as_array().expect("tasks");
    assert_eq!(all.len(), 12);
    for task in all {
        let id = task["id"].as_str().expect("an id");
        let seeded: Vec<&str> = task["seeded"]
            .as_array()
            .expect("seeded")
            .iter()
            .map(|d| d.as_str().expect("a defect"))
            .collect();
        for defect in &seeded {
            for arm in ["A", "B"] {
                let edits = task["seeds"][defect][arm].as_array();
                assert!(
                    edits.is_some_and(|e| !e.is_empty()),
                    "{id} {defect} has no {arm} edits"
                );
            }
        }
        let signalled: Vec<&str> = task["b_signals"]
            .as_array()
            .expect("b_signals")
            .iter()
            .map(|row| row["defect"].as_str().expect("a defect"))
            .collect();
        for defect in &signalled {
            assert!(
                seeded.contains(defect),
                "{id}: a signal for {defect}, which it does not seed"
            );
        }
        for defect in &seeded {
            assert!(
                signalled.contains(defect),
                "{id}: {defect} is seeded with no b_signals row"
            );
        }
    }
    let text = std::fs::read_to_string(ab_dir().join("tasks.json")).expect("tasks.json");
    // `pending` is also a word of the states' vocabularies; a placeholder is a fixture or task whose status says it.
    assert_eq!(
        text.matches("\"status\": \"pending\"").count(),
        0,
        "a frozen corpus has nothing pending"
    );
}

/// A verify task's seeds break the proofs of the truths its `b_signals` rows name: each of those truths is `not
/// verified` in the task's ground truth, so a run that leaves it verified drifts.
#[test]
fn a_verify_tasks_seeded_truths_are_not_verified() {
    let tasks = ab_tasks();
    let mut checked = 0;
    for task in tasks["tasks"].as_array().expect("tasks") {
        if task["workflow"] != "verify" {
            continue;
        }
        for row in task["b_signals"].as_array().expect("b_signals") {
            if let Some(entity) = row["entity"].as_str() {
                assert_eq!(
                    task["ground_truth"][entity], "not verified",
                    "{}: {entity}",
                    task["id"]
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked >= 5,
        "the verify tasks name their seeded truths: {checked}"
    );
}

/// Every ground truth key is of exactly one kind of `states` (its `id` pattern), with a value of that kind's
/// vocabulary and a read for arms A and B; every task's workflow has its RESULT values, one or more of them claiming
/// the work done.
#[test]
fn every_ground_truth_key_has_one_kind_and_a_read_per_arm() {
    let tasks = ab_tasks();
    let kinds = tasks["states"]["kinds"].as_array().expect("states.kinds");
    for kind in kinds {
        for arm in ["A", "B"] {
            let read = &kind[arm];
            assert!(
                ["read", "at"]
                    .iter()
                    .all(|k| read[k].as_str().is_some_and(|s| !s.is_empty()))
                    && read["map"].as_object().is_some_and(|m| !m.is_empty()),
                "{}: arm {arm} has no read, at and map",
                kind["kind"]
            );
        }
    }
    let mut keys = 0;
    for task in tasks["tasks"].as_array().expect("tasks") {
        let id = task["id"].as_str().expect("an id");
        for (key, value) in task["ground_truth"].as_object().expect("ground_truth") {
            let of: Vec<&serde_json::Value> = kinds
                .iter()
                .filter(|k| {
                    regex::Regex::new(k["id"].as_str().expect("an id pattern"))
                        .expect("a pattern")
                        .is_match(key)
                })
                .collect();
            assert_eq!(of.len(), 1, "{id}: {key} is of {} kinds", of.len());
            let values = of[0]["values"].as_array().expect("values");
            assert!(
                values.contains(value),
                "{id}: {key} = {value}, none of {values:?}"
            );
            keys += 1;
        }
        let results = tasks["results"][task["workflow"].as_str().expect("a workflow")]
            .as_array()
            .unwrap_or_else(|| panic!("{id}: its workflow has no results"));
        assert!(results.iter().any(|r| r["claims"] == true), "{id}");
    }
    assert!(keys >= 30, "{keys} ground truth keys");
}

/// The tree git would commit for `path` (relative to the repository) from the working copy, built in a scratch index
/// so the repository's own index is left as it is.
fn working_tree_of(path: &str) -> String {
    let index = tempfile::tempdir().expect("a temp dir");
    let git = |args: &[&str]| {
        let run = Command::new("git")
            .args(args)
            .current_dir(ab_dir().join("../.."))
            .env("GIT_INDEX_FILE", index.path().join("index"))
            .stdin(Stdio::null())
            .output()
            .expect("run git");
        assert!(
            run.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        String::from_utf8_lossy(&run.stdout).trim().to_string()
    };
    git(&["read-tree", "HEAD"]);
    git(&["add", "-A", "--", path]);
    git(&["write-tree", &format!("--prefix={path}/")])
}

/// Each fixture is pinned in `tasks.json` by its git tree, the one `seed.py` copies: a fixture edited after its pin
/// fails here until the pin moves with it, and a moved pin is a new protocol version.
#[test]
fn every_fixture_is_its_pinned_tree() {
    let tasks = ab_tasks();
    for fixture in tasks["fixtures"].as_array().expect("fixtures") {
        let path = fixture["path"].as_str().expect("a path");
        assert_eq!(
            fixture["tree"].as_str(),
            Some(working_tree_of(path).as_str()),
            "{path}: its tree is not the pinned one"
        );
        assert!(
            fixture["baseline"]
                .as_array()
                .is_some_and(|b| !b.is_empty() && b.iter().all(|l| l.is_string())),
            "{path}: no preflight baseline"
        );
    }
}

/// OPERATOR.md builds the binary and the components with the commands the tests' build hint names.
#[test]
fn the_operator_builds_what_the_tests_load() {
    let path = ab_dir().join("OPERATOR.md");
    let operator =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let commands: Vec<&str> = HOW_TO_BUILD
        .split("(cd ")
        .skip(1)
        .filter_map(|part| part.split_once(" && ")?.1.split_once(')'))
        .map(|(command, _)| command)
        .collect();
    assert_eq!(commands.len(), 2, "{HOW_TO_BUILD}");
    for command in commands {
        assert!(
            operator.contains(command),
            "OPERATOR.md does not build with `{command}`"
        );
    }
}

#[test]
fn no_task_is_a_placeholder() {
    let tasks = ab_tasks();
    let placeholders = tasks["tasks"]
        .as_array()
        .expect("tasks")
        .iter()
        .filter(|t| t.get("status").is_some())
        .count();
    assert_eq!(placeholders, 0);
}

/// `prompts/<arm>.md` split at its commands block (`<!-- commands -->` to `<!-- /commands -->`): the text around the
/// block, which every arm shares, and the block.
fn prompt(arm: &str) -> (String, String) {
    let path = ab_dir().join("prompts").join(format!("{arm}.md"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(
        text.matches("{task.prompt}").count(),
        1,
        "{arm}: the task's prompt goes in once"
    );
    let last = text
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default();
    assert_eq!(
        last, "End your final message with one line: {task.results}",
        "{arm}: the prompt ends with the RESULT instruction, its values the task's workflow's"
    );
    let (before, rest) = text
        .split_once("<!-- commands -->\n")
        .unwrap_or_else(|| panic!("{arm}: no commands block"));
    let (commands, after) = rest
        .split_once("<!-- /commands -->\n")
        .unwrap_or_else(|| panic!("{arm}: the commands block is not closed"));
    assert!(
        !commands.trim().is_empty(),
        "{arm}: the commands block names the arm's tools"
    );
    (format!("{before}{after}"), commands.to_string())
}

/// Every arm of `tasks.json` has a prompt; the prompts differ only in their commands block, B0's is B's, Bv1's file is
/// B's byte for byte, and each holds the task's prompt once and ends with the same RESULT instruction.
#[test]
fn the_arms_prompts_differ_only_in_their_commands() {
    let tasks = ab_tasks();
    let arms: Vec<&str> = tasks["arms"]
        .as_array()
        .expect("arms")
        .iter()
        .map(|a| a["id"].as_str().expect("an arm id"))
        .collect();
    assert_eq!(arms, ["A", "B", "B0", "Bv1"]);
    let (shared, a) = prompt("A");
    let (b_shared, b) = prompt("B");
    let (b0_shared, b0) = prompt("B0");
    let (v1_shared, v1) = prompt("Bv1");
    assert_eq!(
        (&b_shared, &b0_shared, &v1_shared),
        (&shared, &shared, &shared),
        "every arm's prompt is A's outside its commands"
    );
    let read = |arm: &str| {
        std::fs::read(ab_dir().join("prompts").join(format!("{arm}.md"))).expect("a prompt")
    };
    assert_eq!(read("Bv1"), read("B"), "Bv1's prompt is B's, byte for byte");
    assert_eq!(v1, b, "Bv1 has B's tools");
    assert_ne!(a, b, "A and B name their own tools");
    assert_eq!(
        b0, b,
        "B0 has B's tools: its difference is its configuration"
    );
    let mut files: Vec<String> = std::fs::read_dir(ab_dir().join("prompts"))
        .expect("prompts/")
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    files.sort();
    assert_eq!(
        files,
        ["A.md", "B.md", "B0.md", "Bv1.md"],
        "one prompt per arm, nothing else"
    );
}
