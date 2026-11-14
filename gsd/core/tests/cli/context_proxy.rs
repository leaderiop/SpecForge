//! The executor's context, measured in CI: on arm B's copy of each A/B fixture's execute task, the `gsd/executor`
//! pack, given as much room as the reads it replaces, costs at most seven tenths of them. Both sides are counted by
//! the host's own token estimate (`specforge_extension_sdk::tokens`).
//!
//! The read set is frozen from the executor's `<required_reading>` as `execute-phase.md` held it before the context
//! routes: the plan, the project, the computed state, the settings, `CLAUDE.md` and the project skills; with a context
//! window of 500,000 tokens or more, also the phase's context, its research and the earlier waves' summaries. Each read
//! is the command that served it then. A guard against a pack that grows, not the design's answer: the A/B's M7
//! measures the context a spawn really starts on.
//!
//! The pack's side counts what the executor still loads before its first write besides the pack: the `specforge`
//! reads of its body's `load_project_state` step, run as the body holds them. A pack must serve its core sections
//! (the plan, its body and its phase), so an empty or header-only answer cannot pass.

use specforge_extension_sdk::tokens::estimate_tokens;

use super::assets_shell::{Shell, asset, block, code};
use super::harness::Project;
use super::harness::ab::seeded;

/// One fixture's execute task and the entities its executor read.
struct Row {
    task: &'static str,
    plan: &'static str,
    project: &'static str,
    context: Option<&'static str>,
    research: Option<&'static str>,
    summaries: &'static [&'static str],
}

const ROWS: &[Row] = &[
    Row {
        task: "taskboard-execute-02-01",
        plan: "v1.0/02/02-01",
        project: "taskboard",
        context: Some("v1.0/02/02-CONTEXT"),
        research: Some("v1.0/02/02-RESEARCH"),
        summaries: &[],
    },
    Row {
        task: "vault-execute-01-02",
        plan: "v1.0/01/01-02",
        project: "vault",
        context: None,
        research: None,
        summaries: &["v1.0/01/01-01/01-01-SUMMARY"],
    },
    Row {
        task: "ledger-execute-02-01",
        plan: "v1.0/02/02-01",
        project: "ledger",
        context: None,
        research: None,
        summaries: &[],
    },
];

/// What a window read: each read's command and cost, and the project files (`CLAUDE.md`, the skills) apart.
struct Reads {
    each: Vec<(String, usize)>,
    files: usize,
}

impl Reads {
    fn total(&self) -> usize {
        self.each.iter().map(|(_, n)| n).sum::<usize>() + self.files
    }
}

/// The cost of `specforge <args>` on `p`, which must exit 0: a failed read fails the test, never counts as 0.
fn read(p: &Project, task: &str, args: &[&str]) -> (String, usize) {
    let run = p.run(args);
    let line = args.join(" ");
    assert_eq!(
        run.code,
        0,
        "{task}: `specforge {line}` failed: {}",
        run.all()
    );
    (line, estimate_tokens(&run.stdout))
}

/// The cost of the project's instructions and skills, as the executor read them: `CLAUDE.md`, and for each skills
/// folder that exists its listing and each skill's `SKILL.md`.
fn project_files(p: &Project) -> usize {
    let root = p.root();
    let text =
        |path: &std::path::Path| std::fs::read_to_string(path).map_or(0, |t| estimate_tokens(&t));
    let mut cost = text(&root.join("CLAUDE.md"));
    for folder in [".claude/skills", ".agents/skills"] {
        let Ok(entries) = std::fs::read_dir(root.join(folder)) else {
            continue;
        };
        let mut names: Vec<std::path::PathBuf> =
            entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        names.sort();
        let listing: Vec<String> = names
            .iter()
            .filter_map(|n| n.file_name().map(|f| f.to_string_lossy().into_owned()))
            .collect();
        cost += estimate_tokens(&listing.join("\n"));
        cost += names
            .iter()
            .map(|n| text(&n.join("SKILL.md")))
            .sum::<usize>();
    }
    cost
}

/// The reads of `row`'s executor at the standard window, or with `large` at 500,000 tokens or more.
fn reads(p: &Project, row: &Row, large: bool) -> Reads {
    let mut each = vec![
        read(
            p,
            row.task,
            &["query", row.plan, "--depth", "1", "--format", "context"],
        ),
        read(
            p,
            row.task,
            &["query", row.project, "--depth", "0", "--format", "context"],
        ),
        read(p, row.task, &["gsd", "progress-report"]),
        read(
            p,
            row.task,
            &["settings", "--format", "json", "--extension", "@gsd/core"],
        ),
    ];
    if large {
        for id in row.context.iter().chain(row.research.iter()) {
            each.push(read(
                p,
                row.task,
                &["query", id, "--depth", "1", "--format", "context"],
            ));
        }
        for id in row.summaries {
            each.push(read(
                p,
                row.task,
                &["query", id, "--depth", "0", "--format", "context"],
            ));
        }
    }
    Reads {
        each,
        files: project_files(p),
    }
}

/// What the executor's body loads at its start besides the pack: each `specforge` answer of the bash fences of
/// `agents/gsd-executor.md`'s `load_project_state` step, run with the plan's phase and milestone, as the agent
/// receives it.
fn start_loads(sh: &Shell, p: &Project, row: &Row) -> usize {
    let body = asset("agents/gsd-executor.md");
    let step = block(&body, "<step name=\"load_project_state\"", "</step>");
    let fences: Vec<String> = gsd_kit::markdown::fences(&step)
        .into_iter()
        .filter(|f| f.language == "bash")
        .map(|f| code(&f.text).to_string())
        .collect();
    assert!(!fences.is_empty(), "load_project_state runs no fence");
    let log = p.root().join(".start-loads.log");
    let phase = row
        .plan
        .rsplit_once('/')
        .map_or(row.plan, |(phase, _)| phase);
    let milestone = row.plan.split('/').next().unwrap_or(row.plan);
    // each answer goes to the log as well as to the step, which may hold it in a variable
    let script = format!(
        "PHASE={phase}\nMILESTONE={milestone}\nspecforge() {{ command specforge \"$@\" | tee -a {}; }}\n{}",
        log.display(),
        fences.join("\n")
    );
    let (out, err, status) = sh.run_in(p.root(), &script);
    assert_eq!(status, 0, "{}: load_project_state: {out}{err}", row.task);
    let loaded = std::fs::read_to_string(&log).unwrap_or_default();
    std::fs::remove_file(&log).ok();
    assert!(
        !loaded.is_empty(),
        "{}: load_project_state read nothing",
        row.task
    );
    estimate_tokens(&loaded)
}

/// The pack's own estimate when it may use `room` tokens (`budget.estimated_tokens` of `specforge context`), once it
/// is shown to serve its core sections: the plan, its body and its phase, each with members.
fn pack(p: &Project, row: &Row, room: usize) -> usize {
    let room = room.to_string();
    let run = p.run(&[
        "context",
        "gsd/executor",
        row.plan,
        "--max-tokens",
        &room,
        "--pick",
        ".",
    ]);
    assert_eq!(
        run.code,
        0,
        "{}: the gsd/executor pack failed: {}",
        row.task,
        run.all()
    );
    let answer: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("{}: the pack is not JSON ({e}): {}", row.task, run.stdout));
    for name in ["plan", "body", "phase"] {
        let served = answer["sections"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|s| s["name"] == name)
            .and_then(|s| s["members"].as_array())
            .is_some_and(|m| !m.is_empty());
        assert!(served, "{}: the pack serves no {name}: {answer}", row.task);
    }
    // a renamed answer reads as no count, never as a pack of 0
    answer["budget"]["estimated_tokens"]
        .as_u64()
        .and_then(|k| usize::try_from(k).ok())
        .unwrap_or_else(|| {
            panic!(
                "{}: the pack's budget holds no estimate: {answer}",
                row.task
            )
        })
}

#[test]
fn the_executor_pack_is_at_most_seven_tenths_of_the_reads_it_replaces() {
    let Some(sh) = Shell::only(&["tee"]) else {
        return;
    };
    eprintln!("fixture window reads pack ratio");
    for row in ROWS {
        let p =
            seeded(row.task, "B", &["--unseeded"]).unwrap_or_else(|e| panic!("{}: {e}", row.task));
        p.collect_seeded(row.task, false);
        let ids = [row.plan, row.project]
            .into_iter()
            .chain(row.context)
            .chain(row.research)
            .chain(row.summaries.iter().copied());
        for id in ids {
            let run = p.run(&["query", id, "--depth", "0"]);
            assert_eq!(run.code, 0, "{}: no entity {id}: {}", row.task, run.all());
        }
        let start = start_loads(&sh, &p, row);
        for (window, large) in [("standard", false), ("large", true)] {
            let reads = reads(&p, row, large);
            let v = reads.total();
            let k = pack(&p, row, v) + start;
            let fixture = row.task.split('-').next().unwrap_or(row.task);
            eprintln!(
                "{fixture} {window} {v} {k} {:.2} (start loads {start}; files {}; {})",
                k as f64 / v as f64,
                reads.files,
                reads
                    .each
                    .iter()
                    .map(|(c, n)| format!("{c}={n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            assert!(
                0 < k && k <= v,
                "{fixture} {window}: the pack costs {k} of {v} tokens of room"
            );
            assert!(
                10 * k <= 7 * v,
                "{fixture} {window}: the pack costs {k} tokens, over seven tenths of the {v} the reads cost: {:?} \
                 (files {})",
                reads.each,
                reads.files
            );
        }
    }
}
