//! Each profile on a seeded violation of each SpecForge gate (ADR 0379 D1, D2): `gsd-compat`, the default, is silent;
//! `gsd-strict` reports and blocks gsd/E515, gsd/E516 and gsd/E521; `gsd-plus` does the same and adds gsd/E517. Every
//! case runs on a fresh copy per profile, so the profile is the only thing that changes, and removes the cause too, so
//! each gate is shown satisfiable.

use serde_json::{Value, json};

use super::cmd_workstreams::codes_e517;
use super::commands_support::{bytes, gsd, refused, twin, unchanged};
use super::execute_rules::{T0, T1, collect_tests, repository};
use super::harness::{Project, judged, project};
use super::prompts_gates::{failing, refused_by};

/// `(profile, blocks)`: `None` writes no settings (the default, gsd-compat).
const PROFILES: [(Option<&str>, bool); 3] = [
    (None, false),
    (Some("gsd-strict"), true),
    (Some("gsd-plus"), true),
];

const PHASE: &str = "v1.0/01";
const TRUTH: &str = "v1.0/01/01-02/TR1";
const E516: &str = "gsd/E516";

fn select(p: &Project, profile: Option<&str>) {
    if let Some(profile) = profile {
        p.settings(json!({ "profile": profile }));
    }
}

/// Records the verifier's verdict on 01-02/TR1 over what it cites.
fn verify_truth(t: &Project) {
    let run = t.record(TRUTH, "E501", "verified", &["--cited", &t.cites(TRUTH)]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(t.inputs(TRUTH, "E501")["verdict"]["state"], "fresh");
}

/// The twin with 01-02/TR1's verdict made stale: recorded, then a file it cites edited.
fn stale_truth() -> Project {
    let t = twin();
    verify_truth(&t);
    t.write(
        "src/tasks.js",
        &format!("{}\n// edited\n", t.read("src/tasks.js")),
    );
    assert_eq!(t.inputs(TRUTH, "E501")["verdict"]["state"], "stale");
    t
}

#[test]
fn e516_blocks_ship_under_strict_and_plus_and_not_by_default() {
    for (profile, blocks) in PROFILES {
        let t = stale_truth();
        select(&t, profile);
        if blocks {
            refused_by(&t, "ship", &[PHASE], "E516");
            // The remedy: the verifier judges again every verdict the edit staled, as re-verification lists them.
            assert_eq!(t.re_verify(PHASE, true)["not_fresh"], 6);
            assert_eq!(t.verify_phase(PHASE), 6);
            assert_eq!(t.re_verify(PHASE, true)["not_fresh"], 0);
        }
        let codes = failing(&t, "ship", &[PHASE]);
        assert!(!codes.contains(&E516.to_string()), "{profile:?}: {codes:?}");
    }
    // An explicit setting beats the preset: the way out for a project that will not judge again.
    let t = stale_truth();
    t.settings(json!({ "profile": "gsd-strict", "specforge.verdict_gate": false }));
    let codes = failing(&t, "ship", &[PHASE]);
    assert!(!codes.contains(&E516.to_string()), "{codes:?}");
}

#[test]
fn e515_blocks_close_milestone_under_strict_and_plus_until_the_requirement_traces() {
    for (profile, blocks) in PROFILES {
        let p = repository();
        assert_eq!(p.fields_of("TASK-01")["status"], "complete");
        select(&p, profile);
        let before = bytes(&p);
        let run = gsd(&p, "close-milestone", &["v1.0"]);
        if blocks {
            refused(&run, "gate_failed", &["gsd/E515"]);
            unchanged(&p, &before);
        } else {
            assert!(!run.all().contains("gsd/E515"), "{}", run.all());
            continue;
        }
        // A commit and a passing test linked to the task trace it.
        p.git_commit("feat(01-01): Store module", &[("src/tasks.js", "a\n")], T0);
        let run = p.collect(&["--runner", "gsd-git"]);
        assert_eq!(run.code, 0, "{}", run.all());
        collect_tests(&p, &[("[gsd:v1.0/01/01-01/T1] adds a task", true)]);
        let after = gsd(&p, "close-milestone", &["v1.0"]).all();
        assert!(!after.contains("gsd/E515"), "{profile:?}: {after}");
    }
}

/// `specforge check`'s exit and its gsd/E521 errors.
fn e521_errors(p: &Project) -> (i32, Vec<Value>) {
    let run = p.run(&["check", "--format", "json"]);
    let found: Value =
        serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("{e}: {}", run.all()));
    let errors = found
        .as_array()
        .expect("a list")
        .iter()
        .filter(|d| d["code"] == "gsd/E521" && d["severity"] == "Error")
        .cloned()
        .collect();
    (run.code, errors)
}

/// The `execute` project with one task commit `message`, a plan's metadata commit, a git collect, passing tests, its
/// plans checked and its phase verified over the committed files: gsd/E521 is the only error it can hold.
fn committed(message: &str) -> Project {
    let p = repository();
    p.git_commit(message, &[("src/tasks.js", "a\n")], T0);
    p.git_commit("docs(01-01): complete the task store plan", &[], T1);
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    // The coverage check's test, and one that proves the truth the verifier verifies.
    collect_tests(
        &p,
        &[
            ("adds a task", true),
            ("[gsd:v1.0/01/01-01/TR1] lists an added task", true),
        ],
    );
    // The plan checker's verdicts, which gsd-strict makes errors when they are missing.
    for (code, plan) in judged(&p.report_all()) {
        if ["gsd/W311", "gsd/W313", "gsd/W315", "gsd/W316"].contains(&code.as_str()) {
            let run = p.record(&plan, &code["gsd/".len()..], "passed", &[]);
            assert_eq!(run.code, 0, "{plan} {code}: {}", run.all());
        }
    }
    assert_eq!(p.verify_phase(PHASE), 3);
    p
}

#[test]
fn e521_is_an_error_under_strict_and_plus_until_the_commit_carries_its_trailer() {
    for (profile, blocks) in PROFILES {
        let p = committed("feat(01-01): Store module");
        select(&p, profile);
        let (code, errors) = e521_errors(&p);
        assert_eq!(errors.len(), usize::from(blocks), "{profile:?}: {errors:?}");
        assert_eq!(code, i32::from(blocks), "{profile:?}: {errors:?}");
        // The trailer gsd-core's executor writes links the commit to its task.
        let p = committed("feat(01-01): Store module\n\nGsd-Task: 01-01/T1\n");
        select(&p, profile);
        assert_eq!(e521_errors(&p), (0, Vec::new()), "{profile:?}");
    }
}

/// The task's full id, as `specforge query` prints it, is a trailer the collector reads: gsd/E521 does not ask for it.
#[test]
fn e521_is_silent_under_strict_and_plus_for_a_trailer_naming_the_task_by_its_full_id() {
    for (profile, _) in PROFILES.into_iter().filter(|&(_, blocks)| blocks) {
        let p = committed("feat(01-01): Store module\n\nGsd-Task: v1.0/01/01-01/T1\n");
        select(&p, profile);
        assert_eq!(e521_errors(&p), (0, Vec::new()), "{profile:?}");
    }
}

#[test]
fn e517_is_reported_under_plus_only() {
    for (profile, fires) in [
        (None, false),
        (Some("gsd-strict"), false),
        (Some("gsd-plus"), true),
    ] {
        let p = project("workstreams");
        select(&p, profile);
        let found = codes_e517(&p);
        assert_eq!(found.len(), usize::from(fires), "{profile:?}: {found:?}");
        if fires {
            assert!(found[0].contains("src/tasks.js"), "{found:?}");
        }
    }
}
