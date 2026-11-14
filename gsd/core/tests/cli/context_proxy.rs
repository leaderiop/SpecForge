//! The executor's context, measured in CI: on arm B's copy of each A/B fixture's execute task, the `gsd/executor`
//! pack, given as much room as the reads it replaces, costs at most seven tenths of them. Both sides are counted by
//! the host's own token estimate (`specforge_extension_sdk::tokens`).
//!
//! The read set is frozen from the executor's `<required_reading>` as `execute-phase.md` held it before the context
//! routes: the plan, the project, the computed state, the settings, `CLAUDE.md` and the project skills; with a context
//! window of 500,000 tokens or more, also the phase's context, its research and the earlier waves' summaries. Each read
//! is the command that served it then. A guard against a pack that grows, not the design's answer: the A/B's M7
//! measures the context a spawn really starts on.

use specforge_extension_sdk::tokens::estimate_tokens;

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

/// The pack's own estimate when it may use `room` tokens: `budget.estimated_tokens` of `specforge context`.
fn pack(p: &Project, row: &Row, room: usize) -> usize {
    let room = room.to_string();
    let run = p.run(&[
        "context",
        "gsd/executor",
        row.plan,
        "--max-tokens",
        &room,
        "--pick",
        ".budget.estimated_tokens",
    ]);
    assert_eq!(
        run.code,
        0,
        "{}: the gsd/executor pack failed: {}",
        row.task,
        run.all()
    );
    // a missing key answers empty with exit 0: an empty pick is a renamed answer, never a pack of 0
    run.stdout.trim().parse().unwrap_or_else(|_| {
        panic!(
            "{}: `--pick .budget.estimated_tokens` printed no count: {:?}",
            row.task, run.stdout
        )
    })
}

#[test]
fn the_executor_pack_is_at_most_seven_tenths_of_the_reads_it_replaces() {
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
        for (window, large) in [("standard", false), ("large", true)] {
            let reads = reads(&p, row, large);
            let v = reads.total();
            let k = pack(&p, row, v);
            let fixture = row.task.split('-').next().unwrap_or(row.task);
            eprintln!(
                "{fixture} {window} {v} {k} {:.2} (files {}; {})",
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
