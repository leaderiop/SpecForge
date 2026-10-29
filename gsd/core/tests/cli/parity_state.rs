//! The taskboard's SpecForge twin of phase 1's execution against what gsd-tools 1.18.0 says (parity suite, "state"):
//! `progress json`, the two `SUMMARY.md` files, and the tests `npm test` recorded. What verification decides (a
//! phase `Complete`, state json's `completed_phases` and `percent`) holds once the twin's phase 01 has its E501 verdicts, which the tests record.

use std::path::Path;

use serde_json::{Value, json};

use crate::harness::{Project, taskboard};

fn parity(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity")
        .join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display()))
}

fn golden(name: &str) -> Value {
    serde_json::from_str(&parity(&format!("goldens/taskboard/{name}.json")))
        .unwrap_or_else(|e| panic!("{name} is JSON: {e}"))
}

/// The label `progress json` gives a phase status (`phase-status.cts`).
fn label(status: &str) -> &'static str {
    match status {
        "not_started" => "Pending",
        "planned" => "Planned",
        "in_progress" => "In Progress",
        "executed" => "Executed",
        "needs_review" => "Needs Review",
        "complete" => "Complete",
        other => panic!("no label for {other}"),
    }
}

/// The twin with the verifier's `verified` verdict on every item of phase 01 (its verification then passes).
fn verified_taskboard() -> Project {
    let p = taskboard();
    // Every truth, artifact and key link of phase 01 (the exact count, so a dropped item is noticed).
    assert_eq!(p.verify_phase("v1.0/01"), 7);
    p
}

#[test]
fn taskboard_progress_matches_gsd_tools() {
    let p = verified_taskboard();
    let progress = golden("progress-json");
    let phases = progress["phases"].as_array().expect("phases");
    for phase in phases {
        let number = phase["number"].as_str().expect("a number");
        let fields = p.fields_of(&format!("v1.0/{number}"));
        let counts = format!("{}/{}", phase["summaries"], phase["plans"]);
        assert_eq!(
            fields["plans_complete"], counts,
            "phase {number}: summaries/plans"
        );
        let status = fields["status"].as_str().unwrap_or_default();
        assert_eq!(label(status), phase["status"], "phase {number}");
    }
    // The totals, and the plan-based percent, from `milestone.progress`.
    let milestone = p.fields_of("v1.0")["progress"].clone();
    assert_eq!(milestone["plans"], progress["total_plans"]);
    assert_eq!(milestone["plans_complete"], progress["total_summaries"]);
    let plans = milestone["plans"].as_f64().expect("plans");
    let done = milestone["plans_complete"]
        .as_f64()
        .expect("plans_complete");
    assert_eq!(
        (done / plans * 100.0).round(),
        progress["percent"].as_f64().expect("percent")
    );
    // `state json`: the counts equal.
    let state = golden("state-json")["progress"].clone();
    assert_eq!(milestone["phases"], state["total_phases"]);
    assert_eq!(milestone["plans"], state["total_plans"]);
    assert_eq!(milestone["plans_complete"], state["completed_plans"]);
    assert_eq!(milestone["phases_complete"], state["completed_phases"]);
    assert_eq!(milestone["percent"], state["percent"]);
}

#[test]
fn phase_01_is_complete_as_upstream_says() {
    let progress = golden("progress-json");
    assert_eq!(progress["phases"][0]["status"], "Complete");
    let p = taskboard();
    // Without the verifier's verdicts the phase stops at Executed.
    assert_eq!(
        label(
            p.fields_of("v1.0/01")["status"]
                .as_str()
                .unwrap_or_default()
        ),
        "Executed"
    );
    let p = verified_taskboard();
    assert_eq!(
        label(
            p.fields_of("v1.0/01")["status"]
                .as_str()
                .unwrap_or_default()
        ),
        "Complete"
    );
}

#[test]
fn completed_phases_and_percent_match_upstream() {
    let p = verified_taskboard();
    let state = golden("state-json")["progress"].clone();
    let milestone = p.fields_of("v1.0")["progress"].clone();
    assert_eq!(
        (&milestone["phases_complete"], &milestone["percent"]),
        (&json!(1), &json!(25))
    );
    assert_eq!(
        (&state["completed_phases"], &state["percent"]),
        (&json!(1), &json!(25))
    );
}

/// A `SUMMARY.md`'s frontmatter and bold one-liner, read by the subset of YAML GSD's summaries use.
struct Summary {
    status: String,
    subsystem: String,
    one_liner: String,
    requirements: Vec<String>,
    deliverables: Vec<Deliverable>,
}

#[derive(Default)]
struct Deliverable {
    id: String,
    description: String,
    requirement: Option<String>,
    human_judgment: bool,
    rationale: Option<String>,
    /// `(kind, ref, status)`.
    checks: Vec<(String, String, String)>,
}

fn unquote(text: &str) -> String {
    text.trim().trim_matches('"').to_string()
}

fn inline_list(text: &str) -> Vec<String> {
    text.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(unquote)
        .filter(|s| !s.is_empty())
        .collect()
}

fn read_summary(path: &str) -> Summary {
    let text = parity(path);
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("---"), "{path}: frontmatter");
    let front: Vec<&str> = lines.by_ref().take_while(|l| *l != "---").collect();
    let one_liner = lines
        .find(|l| l.starts_with("**"))
        .map(|l| l.trim_matches('*').to_string())
        .expect("a bold one-liner");
    let top = |key: &str| {
        front
            .iter()
            .find_map(|l| l.strip_prefix(&format!("{key}:")))
            .map(unquote)
            .unwrap_or_default()
    };
    let mut deliverables: Vec<Deliverable> = Vec::new();
    let mut in_coverage = false;
    for line in &front {
        if !line.starts_with(' ') {
            in_coverage = line.starts_with("coverage:");
            continue;
        }
        if !in_coverage {
            continue;
        }
        let item = line.trim();
        if let Some(id) = item.strip_prefix("- id:") {
            deliverables.push(Deliverable {
                id: unquote(id),
                ..Default::default()
            });
            continue;
        }
        let Some(current) = deliverables.last_mut() else {
            continue;
        };
        if let Some(kind) = item.strip_prefix("- kind:") {
            current
                .checks
                .push((unquote(kind), String::new(), String::new()));
        } else if let Some(reference) = item.strip_prefix("ref:") {
            current.checks.last_mut().expect("a check").1 = unquote(reference);
        } else if let Some(status) = item.strip_prefix("status:") {
            current.checks.last_mut().expect("a check").2 = unquote(status);
        } else if let Some(v) = item.strip_prefix("description:") {
            current.description = unquote(v);
        } else if let Some(v) = item.strip_prefix("requirement:") {
            current.requirement = Some(unquote(v));
        } else if let Some(v) = item.strip_prefix("human_judgment:") {
            current.human_judgment = unquote(v) == "true";
        } else if let Some(v) = item.strip_prefix("rationale:") {
            current.rationale = Some(unquote(v));
        }
    }
    Summary {
        status: top("status"),
        subsystem: top("subsystem"),
        one_liner,
        requirements: front
            .iter()
            .find_map(|l| l.strip_prefix("requirements-completed:"))
            .map(inline_list)
            .unwrap_or_default(),
        deliverables,
    }
}

const SUMMARIES: [(&str, &str); 2] = [
    (
        "fixtures/taskboard/.planning/phases/01-core/01-01-SUMMARY.md",
        "v1.0/01/01-01/01-01-SUMMARY",
    ),
    (
        "fixtures/taskboard/.planning/phases/01-core/01-02-SUMMARY.md",
        "v1.0/01/01-02/01-02-SUMMARY",
    ),
];

#[test]
fn taskboard_summaries_match_planning() {
    let p = taskboard();
    for (path, id) in SUMMARIES {
        let summary = read_summary(path);
        let fields = p.fields_of(id);
        assert_eq!(fields["status"], summary.status.as_str(), "{id}: status");
        assert_eq!(
            fields["subsystem"],
            summary.subsystem.as_str(),
            "{id}: subsystem"
        );
        assert_eq!(
            fields["one_liner"],
            summary.one_liner.as_str(),
            "{id}: one_liner"
        );
        assert_eq!(
            fields["requirements_completed"],
            json!(summary.requirements),
            "{id}: requirements"
        );
        for d in &summary.deliverables {
            let fields = p.fields_of(&format!("{id}/{}", d.id));
            let at = format!("{id}/{}", d.id);
            assert_eq!(
                fields["description"],
                d.description.as_str(),
                "{at}: description"
            );
            assert_eq!(
                fields.get("requirement").and_then(Value::as_str),
                d.requirement.as_deref(),
                "{at}: requirement"
            );
            assert_eq!(
                fields["human_judgment"], d.human_judgment,
                "{at}: human_judgment"
            );
            assert_eq!(
                fields.get("rationale").and_then(Value::as_str),
                d.rationale.as_deref(),
                "{at}: rationale"
            );
            let checks: Vec<(String, String)> = fields
                .get("checks")
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .map(|c| {
                            (
                                c["kind"].as_str().unwrap_or_default().to_string(),
                                c["ref"].as_str().unwrap_or_default().to_string(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let expected: Vec<(String, String)> = d
                .checks
                .iter()
                .map(|(k, r, _)| (k.clone(), r.clone()))
                .collect();
            assert_eq!(checks, expected, "{at}: checks");
        }
        // No other deliverable than the SUMMARY's.
        let written: Vec<String> = p
            .nodes_of("deliverable")
            .iter()
            .filter_map(|n| n["id"].as_str())
            .filter(|n| n.starts_with(&format!("{id}/")))
            .map(String::from)
            .collect();
        assert_eq!(
            written.len(),
            summary.deliverables.len(),
            "{id}: {written:?}"
        );
    }
}

fn statuses(p: &Project, id: &str) -> Vec<String> {
    p.fields_of(id)
        .get("checks")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|c| c["status"].as_str().unwrap_or("-").to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn taskboard_coverage_matches_recorded_tests() {
    let p = taskboard();
    // Before any report: no status.
    assert_eq!(statuses(&p, "v1.0/01/01-01/01-01-SUMMARY/D1"), ["-"]);
    let report =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/goldens/taskboard/junit.xml");
    let run = p.collect(&[
        "--runner",
        "gsd-junit",
        "--report",
        &report.display().to_string(),
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    for (path, id) in SUMMARIES {
        for d in read_summary(path).deliverables {
            let expected: Vec<String> = d.checks.iter().map(|(_, _, s)| s.clone()).collect();
            let given = statuses(&p, &format!("{id}/{}", d.id));
            assert_eq!(given, expected, "{id}/{}", d.id);
        }
    }
    p.check_exact(&[]);
    let check = p.run(&["check"]);
    assert!(!check.all().contains("I205"), "{}", check.all());
}

#[test]
fn the_twin_reports_only_unrecorded_verdicts_with_and_without_a_report() {
    let p = taskboard();
    p.check_exact(&[]);
    let report =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/goldens/taskboard/junit.xml");
    let run = p.collect(&[
        "--runner",
        "gsd-junit",
        "--report",
        &report.display().to_string(),
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    p.check_exact(&[]);
    // With a report the harness hides I205 for every project: a stray one would be invisible, so count it raw.
    let check = p.run(&["check"]);
    assert!(!check.all().contains("I205"), "{}", check.all());
}
