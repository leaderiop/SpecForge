//! The plan flow end to end on the parity twins, taskboard and ledger (ADR 0378): each runs its phase 01 as a chain of
//! two plans that share paths. Unedited, every plan runs in its declared wave; with 01-02's `depends_on` dropped (in a
//! temp copy) the two plans share wave 1 and the strict wave keeps them apart, `gsd/W526` saying where 01-02 runs. In
//! both states the phase's thread passes `specforge trace --plan`, its critical path holds, and a measured plan reads
//! its estimate error.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::critical_path::critical;
use crate::estimate_error::{error_of, report_text};
use crate::harness::{Project, ledger, taskboard};
use crate::strict_wave::{placed, w526};
use crate::thread_plan::{thread, trace};

/// One twin and what its phase 01 shares.
struct Twin {
    name: &'static str,
    open: fn() -> Project,
    /// 01-01's `files_modified` line: the anchor its estimate is written after.
    first_plan: &'static str,
    /// The paths 01-01 and 01-02 both modify, sorted.
    shared: &'static [&'static str],
    /// `gsd/W526` on 01-02 once its dependency is dropped.
    pushed: &'static str,
}

const TWINS: [Twin; 2] = [
    Twin {
        name: "taskboard",
        open: taskboard,
        first_plan: "  files_modified   [\"package.json\", \"src/tasks.js\", \"test/tasks.test.js\"]\n",
        shared: &["src/tasks.js", "test/tasks.test.js"],
        pushed: "plan 'v1.0/01/01-02' shares 'src/tasks.js', 'test/tasks.test.js' with 'v1.0/01/01-01' (tasks: \
                 v1.0/01/01-01/T1, v1.0/01/01-01/T2, v1.0/01/01-02/T1): it runs in strict wave 2, after \
                 'v1.0/01/01-01' in strict wave 1, not in its wave 1",
    },
    Twin {
        name: "ledger",
        open: ledger,
        first_plan: "  files_modified   [\"Cargo.toml\", \"Cargo.lock\", \"src/lib.rs\", \"tests/entries.rs\"]\n",
        shared: &["src/lib.rs"],
        pushed: "plan 'v1.0/01/01-02' shares 'src/lib.rs' with 'v1.0/01/01-01' (tasks: v1.0/01/01-01/T1, \
                 v1.0/01/01-02/T1): it runs in strict wave 2, after 'v1.0/01/01-01' in strict wave 1, not in its \
                 wave 1",
    },
];

/// 01-02's dependency on 01-01, the same line in both twins.
const DEPENDENCY: &str = "  depends_on       [01-01]\n";

/// The plans of both twins.
const PLANS: [&str; 3] = ["v1.0/01/01-01", "v1.0/01/01-02", "v1.0/02/02-01"];

/// Phase 01's tasks in thread and path order: 01-01's, then 01-02's.
const PHASE_01_TASKS: [&str; 4] = [
    "v1.0/01/01-01/T1",
    "v1.0/01/01-01/T2",
    "v1.0/01/01-02/T1",
    "v1.0/01/01-02/T2",
];

impl Twin {
    /// A fresh temp copy of the twin, unedited.
    fn unedited(&self) -> Project {
        (self.open)()
    }

    /// A fresh temp copy with 01-02's `depends_on` removed: 01-01 and 01-02 both declare wave 1.
    fn dropped(&self) -> Project {
        let p = (self.open)();
        p.edit("spec/plans.spec", DEPENDENCY, "");
        p
    }

    /// Both states, named.
    fn states(&self) -> [(&'static str, Project); 2] {
        [("unedited", self.unedited()), ("dropped", self.dropped())]
    }
}

/// The messages of the findings of `code`, sorted.
fn messages(findings: &[(String, String)], code: &str) -> Vec<String> {
    findings
        .iter()
        .filter(|(c, _)| c == code)
        .map(|(_, message)| message.clone())
        .collect()
}

#[test]
fn the_unedited_twins_run_in_their_declared_waves() {
    for twin in &TWINS {
        let p = twin.unedited();
        assert_eq!(
            placed(&p, "v1.0/01/01-01"),
            (Some(1), Some(1)),
            "{}",
            twin.name
        );
        assert_eq!(
            placed(&p, "v1.0/01/01-02"),
            (Some(2), Some(2)),
            "{}",
            twin.name
        );
        assert_eq!(
            placed(&p, "v1.0/02/02-01"),
            (Some(1), Some(1)),
            "{}",
            twin.name
        );
        let codes = p.codes();
        for code in ["gsd/W305", "gsd/W526"] {
            assert!(
                !codes.contains(&code.to_string()),
                "{}: {codes:?}",
                twin.name
            );
        }
        for plan in PLANS {
            assert_eq!(error_of(&p, plan), Value::Null, "{}: {plan}", twin.name);
        }
    }
}

#[test]
fn a_dropped_dependency_on_the_twins_is_a_wave_collision() {
    for twin in &TWINS {
        let p = twin.dropped();
        assert_eq!(
            placed(&p, "v1.0/01/01-01"),
            (Some(1), Some(1)),
            "{}",
            twin.name
        );
        assert_eq!(
            placed(&p, "v1.0/01/01-02"),
            (Some(1), Some(2)),
            "{}",
            twin.name
        );
        assert_eq!(
            placed(&p, "v1.0/02/02-01"),
            (Some(1), Some(1)),
            "{}",
            twin.name
        );
        let findings = p.findings();
        let overlaps: Vec<String> = twin
            .shared
            .iter()
            .map(|path| {
                format!(
                    "plans 'v1.0/01/01-01' and 'v1.0/01/01-02' of wave 1 both modify '{path}': under gsd-compat \
                     the wave runs sequentially"
                )
            })
            .collect();
        assert_eq!(messages(&findings, "gsd/W305"), overlaps, "{}", twin.name);
        assert_eq!(
            messages(&findings, "gsd/W526"),
            [twin.pushed],
            "{}",
            twin.name
        );
        assert_eq!(w526(&p), [twin.pushed], "{}", twin.name);
    }
}

/// A task of a placed plan, from the exported graph.
struct Task {
    id: String,
    plan: String,
    phase: String,
    /// Its plan's strict wave.
    strict_wave: i64,
    files: Vec<String>,
}

/// Every task of a plan that has a phase and a strict wave (a quick plan has neither).
fn placed_tasks(p: &Project) -> Vec<Task> {
    let plans: BTreeMap<String, (String, i64)> = p
        .nodes_of("plan")
        .iter()
        .filter_map(|plan| {
            let fields = &plan["fields"];
            let placed = (
                fields["phase"].as_str()?.to_string(),
                fields["strict_wave"].as_i64()?,
            );
            Some((plan["id"].as_str()?.to_string(), placed))
        })
        .collect();
    p.nodes_of("task")
        .iter()
        .filter_map(|task| {
            let fields = &task["fields"];
            let plan = fields["plan"].as_str()?.to_string();
            let (phase, strict_wave) = plans.get(&plan)?.clone();
            let files = fields["files"]
                .as_array()?
                .iter()
                .map(|f| f.as_str().expect("a path").to_string())
                .collect();
            Some(Task {
                id: task["id"].as_str()?.to_string(),
                plan,
                phase,
                strict_wave,
                files,
            })
        })
        .collect()
}

/// Every two tasks of different plans of one phase whose `files` share a path, with the first such path.
fn sharing_pairs(tasks: &[Task]) -> Vec<(&Task, &Task, &str)> {
    let mut pairs = Vec::new();
    for (i, task) in tasks.iter().enumerate() {
        for other in &tasks[i + 1..] {
            if task.phase != other.phase || task.plan == other.plan {
                continue;
            }
            if let Some(path) = task.files.iter().find(|f| other.files.contains(f)) {
                pairs.push((task, other, path.as_str()));
            }
        }
    }
    pairs
}

#[test]
fn two_tasks_sharing_a_path_never_share_a_strict_wave() {
    for twin in &TWINS {
        for (state, p) in twin.states() {
            let tasks = placed_tasks(&p);
            let pairs = sharing_pairs(&tasks);
            // Not vacuous: 01-01's and 01-02's first tasks share a path in either state.
            assert!(
                !pairs.is_empty(),
                "{} {state}: no two tasks share a path",
                twin.name
            );
            for (task, other, path) in pairs {
                assert_ne!(
                    task.strict_wave, other.strict_wave,
                    "{} {state}: {} and {} both touch '{path}'",
                    twin.name, task.id, other.id
                );
            }
        }
    }
}

#[test]
fn the_twins_thread_passes_trace_before_and_after() {
    let expected = json!({
        "plan_id": "v1.0/01",
        "entries": [
            { "entity_id": "v1.0/01/01-01", "action": "execute" },
            { "entity_id": PHASE_01_TASKS[0], "action": "implement" },
            { "entity_id": PHASE_01_TASKS[1], "action": "implement" },
            { "entity_id": "v1.0/01/01-02", "action": "execute" },
            { "entity_id": PHASE_01_TASKS[2], "action": "implement" },
            { "entity_id": PHASE_01_TASKS[3], "action": "implement" },
        ]
    });
    for twin in &TWINS {
        for (state, p) in twin.states() {
            let plan = thread(&p, "v1.0/01");
            assert_eq!(plan, expected, "{} {state}", twin.name);
            let (run, outcome) = trace(&p, &plan);
            assert_eq!(run.code, 0, "{} {state}: {}", twin.name, run.all());
            assert_eq!(outcome["ok"], true, "{} {state}: {outcome:#}", twin.name);
        }
    }
}

fn path(ids: &[&str]) -> Option<Vec<String>> {
    Some(ids.iter().map(|id| (*id).to_string()).collect())
}

#[test]
fn the_twins_critical_path_holds_through_the_shared_path() {
    for twin in &TWINS {
        for (state, p) in twin.states() {
            // Unedited, 01-02 runs after 01-01 by its dependency; dropped, by the paths they share.
            assert_eq!(
                critical(&p, "v1.0/01"),
                (path(&PHASE_01_TASKS), Some(4)),
                "{} {state}",
                twin.name
            );
            assert_eq!(
                critical(&p, "v1.0/02"),
                (path(&["v1.0/02/02-01/T1", "v1.0/02/02-01/T2"]), Some(2)),
                "{} {state}",
                twin.name
            );
        }
    }
}

#[test]
fn a_measured_plan_of_the_twin_reads_its_estimate_error() {
    for twin in &TWINS {
        let p = twin.unedited();
        assert!(!report_text(&p).contains("estimate error"), "{}", twin.name);
        // 01-01 is complete with two tasks: two estimated held. No harness records its tokens, so that member is
        // absent, not zero.
        p.edit(
            "spec/plans.spec",
            twin.first_plan,
            &format!(
                "{}  estimate         {{ tokens 60000 tasks 2 }}\n",
                twin.first_plan
            ),
        );
        assert_eq!(
            error_of(&p, "v1.0/01/01-01"),
            json!({ "tasks": 0 }),
            "{}",
            twin.name
        );
        for plan in &PLANS[1..] {
            assert_eq!(error_of(&p, plan), Value::Null, "{}: {plan}", twin.name);
        }
        let text = report_text(&p);
        let lines: Vec<&str> = text.lines().collect();
        let phase = lines
            .iter()
            .position(|l| l.starts_with("01 "))
            .unwrap_or_else(|| panic!("{}: phase 01's line: {text}", twin.name));
        assert_eq!(
            lines[phase + 1],
            "  estimate error 01-01: tasks 0%",
            "{}: {text}",
            twin.name
        );
        assert_eq!(
            text.matches("estimate error").count(),
            1,
            "{}: {text}",
            twin.name
        );
    }
}
