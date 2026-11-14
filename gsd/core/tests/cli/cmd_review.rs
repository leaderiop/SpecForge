//! `record_review`: a cross-AI review round and its concerns, written in one call.

use super::commands_support::*;

const PHASE: &str = "v1.0/01";
const ROUND: &str = "v1.0/01/RV1";

/// A round of two reviewers over phase 01's plans, with a high and a medium concern.
fn round() -> serde_json::Value {
    serde_json::json!({
        "plans": ["01-01", "01-02"],
        "reviewers": [
            { "name": "claude", "model": "opus", "source": "cli" },
            { "name": "codex", "model": "gpt", "source": "cli" }
        ],
        "lanes": [
            { "reviewer": "claude", "summary": "Sound", "strengths": ["small tasks"], "risk": "low" },
            { "reviewer": "codex", "summary": "One gap", "suggestions": ["test the reload"], "risk": "medium" }
        ],
        "coverage": [{ "reviewer": "claude", "total": 2 }, { "reviewer": "codex", "total": 2, "missing": [] }],
        "consensus": "Ready once the reload is tested",
        "strengths": ["small tasks"],
        "concerns": [
            { "plan": "01-02", "reviewers": ["claude", "codex"], "severity": "high", "text": "The reload is untested", "actionable": true },
            { "plan": "01-01", "reviewers": ["codex"], "severity": "medium", "text": "Name the store module", "actionable": true }
        ]
    })
}

fn record(
    p: &crate::harness::Project,
    review: &serde_json::Value,
    extra: &[&str],
) -> crate::harness::Run {
    let text = review.to_string();
    let mut args = vec![PHASE, "--review", text.as_str()];
    args.extend_from_slice(extra);
    gsd(p, "record-review", &args)
}

#[test]
fn a_round_is_recorded_with_its_lanes_and_concerns() {
    let p = twin();
    let run = record(&p, &round(), &["--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let answer = json(&run)["output"].clone();
    assert_eq!(answer["review"], ROUND);
    assert_eq!(
        answer["concerns"],
        serde_json::json!([format!("{ROUND}/RC1"), format!("{ROUND}/RC2")])
    );
    let review = p.fields_of(ROUND);
    assert_eq!(review["lanes"][1]["suggestions"][0], "test the reload");
    assert_eq!(review["open"]["high"], 1);
    assert_eq!(review["open"]["actionable"], 1);
    assert!(
        review["reviewed_at"]
            .as_str()
            .is_some_and(|t| t.ends_with('Z')),
        "{review}"
    );
    let concern = p.fields_of(&format!("{ROUND}/RC1"));
    assert_eq!(concern["status"], "open");
    assert_eq!(concern["severity"], "high");
    assert!(
        !p.codes().iter().any(|c| c.starts_with('E')),
        "{:?}",
        p.codes()
    );
}

#[test]
fn a_round_names_only_the_phases_plans() {
    let p = twin();
    let before = bytes(&p);
    let mut review = round();
    review["plans"] = serde_json::json!(["01-01", "02-01"]);
    refused(
        &record(&p, &review, &[]),
        "invalid_input",
        &["02-01", PHASE],
    );
    review["plans"] = serde_json::json!([]);
    refused(&record(&p, &review, &[]), "invalid_input", &["plans"]);
    let mut review = round();
    review["concerns"][0]["plan"] = serde_json::json!("01-03");
    refused(&record(&p, &review, &[]), "invalid_input", &["01-03"]);
    unchanged(&p, &before);
}

#[test]
fn a_concern_names_a_reviewer_of_the_round() {
    let p = twin();
    let before = bytes(&p);
    let mut review = round();
    review["concerns"][1]["reviewers"] = serde_json::json!(["gemini"]);
    refused(
        &record(&p, &review, &[]),
        "invalid_input",
        &["gemini", "concerns[1]"],
    );
    let mut review = round();
    review["concerns"][1]["reviewers"] = serde_json::json!([]);
    refused(
        &record(&p, &review, &[]),
        "invalid_input",
        &["concerns[1].reviewers"],
    );
    let mut review = round();
    review["reviewers"][1]["name"] = serde_json::json!("claude");
    refused(
        &record(&p, &review, &[]),
        "invalid_input",
        &["claude", "twice"],
    );
    let mut review = round();
    review["lanes"][0]["reviewer"] = serde_json::json!("gemini");
    refused(
        &record(&p, &review, &[]),
        "invalid_input",
        &["lanes[0]", "gemini"],
    );
    unchanged(&p, &before);
    // a drift finding has no reviewer
    let mut review = round();
    review["concerns"][1]["reviewers"] = serde_json::json!([]);
    review["concerns"][1]["drift"] = serde_json::json!(true);
    assert_eq!(record(&p, &review, &[]).code, 0);
}

#[test]
fn a_second_call_with_round_appends_unchecked_and_concerns() {
    let p = twin();
    assert_eq!(record(&p, &round(), &[]).code, 0);
    let more = serde_json::json!({
        "unchecked": [{ "symbol": "loadTasks", "why": "no test names it" }],
        "concerns": [{ "plan": "01-02", "severity": "low", "text": "Drifted from the plan", "drift": true, "actionable": true }]
    });
    let run = record(&p, &more, &["--round", ROUND, "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(json(&run)["output"]["concerns"][0], format!("{ROUND}/RC3"));
    let review = p.fields_of(ROUND);
    assert_eq!(review["unchecked"][0]["symbol"], "loadTasks");
    // a drift finding counts in neither
    assert_eq!(review["open"]["actionable"], 1);
    assert_eq!(review["reviewers"][0]["name"], "claude");
}

#[test]
fn round_takes_only_unchecked_and_concerns() {
    let p = twin();
    assert_eq!(record(&p, &round(), &[]).code, 0);
    let before = bytes(&p);
    let more = serde_json::json!({ "consensus": "changed", "concerns": [] });
    refused(
        &record(&p, &more, &["--round", ROUND]),
        "invalid_input",
        &["consensus"],
    );
    let more = serde_json::json!({
        "concerns": [{ "severity": "low", "text": "x", "reviewers": ["gemini"] }]
    });
    refused(
        &record(&p, &more, &["--round", ROUND]),
        "invalid_input",
        &["gemini"],
    );
    unchanged(&p, &before);
}

const PLAN: &str = "v1.0/01/01-02";

fn conflict(p: &crate::harness::Project, dimension: &str) -> crate::harness::Run {
    gsd(
        p,
        "record-conflict",
        &[
            "01-02",
            "--dimension",
            dimension,
            "--property",
            "titles survive a reload",
            "--conflicts-with",
            "D-02",
            "--alternatives",
            "keep the id",
            "--format",
            "json",
        ],
    )
}

#[test]
fn a_conflict_is_recorded_once() {
    let p = twin();
    let run = conflict(&p, "task_completeness");
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(json(&run)["output"]["conflict"], "v1.0/01/CF1");
    assert_eq!(json(&run)["output"]["reused"], false);
    let before = bytes(&p);
    let again = conflict(&p, "task_completeness");
    assert_eq!(again.code, 0, "{}", again.all());
    assert_eq!(json(&again)["conflict"], "v1.0/01/CF1");
    assert_eq!(json(&again)["reused"], true);
    unchanged(&p, &before);
    let fields = p.fields_of("v1.0/01/CF1");
    assert_eq!(fields["status"], "open");
    assert_eq!(fields["plan"], PLAN);
    assert!(
        !p.codes().iter().any(|c| c.starts_with('E')),
        "{:?}",
        p.codes()
    );
}

#[test]
fn newlines_in_a_conflict_field_become_one_space() {
    let p = twin();
    assert_eq!(conflict(&p, "task\n\t completeness\n").code, 0);
    assert_eq!(
        p.fields_of("v1.0/01/CF1")["dimension"],
        "task  completeness"
    );
    let before = bytes(&p);
    refused(
        &conflict(&p, "\n\t"),
        "invalid_input",
        &["dimension", "empty"],
    );
    unchanged(&p, &before);
}

#[test]
fn a_quick_plan_has_no_conflict_channel() {
    let p = twin();
    plant(
        &p,
        "spec/quick.spec",
        "plan 261112-abc \"Trim titles\" {\n  type           execute\n  files_modified []\n  objective      \"Trim titles\"\n}\n",
    );
    let before = bytes(&p);
    let run = gsd(
        &p,
        "record-conflict",
        &[
            "261112-abc",
            "--dimension",
            "d",
            "--property",
            "p",
            "--conflicts-with",
            "c",
        ],
    );
    refused(&run, "invalid_input", &["261112-abc", "quick"]);
    unchanged(&p, &before);
}

#[test]
fn resolve_by_plan_closes_its_open_conflicts() {
    let p = twin();
    assert_eq!(conflict(&p, "one").code, 0);
    assert_eq!(conflict(&p, "two").code, 0);
    let run = gsd(
        &p,
        "resolve-conflict",
        &[
            "adopted the alternative",
            "--plan",
            "01-02",
            "--format",
            "json",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        json(&run)["output"]["resolved"],
        serde_json::json!(["v1.0/01/CF1", "v1.0/01/CF2"])
    );
    for id in ["v1.0/01/CF1", "v1.0/01/CF2"] {
        let fields = p.fields_of(id);
        assert_eq!(fields["status"], "resolved");
        assert_eq!(fields["resolved"]["how"], "adopted the alternative");
    }
    // none left: an empty answer, exit 0
    let run = gsd(
        &p,
        "resolve-conflict",
        &["x", "--plan", "01-02", "--format", "json"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(json(&run)["resolved"], serde_json::json!([]));
}

#[test]
fn a_resolved_conflict_is_refused() {
    let p = twin();
    assert_eq!(conflict(&p, "one").code, 0);
    let args = ["amended D-02", "--conflict", "v1.0/01/CF1"];
    assert_eq!(gsd(&p, "resolve-conflict", &args).code, 0);
    let before = bytes(&p);
    refused(
        &gsd(&p, "resolve-conflict", &args),
        "invalid_input",
        &["CF1", "already resolved"],
    );
    refused(
        &gsd(&p, "resolve-conflict", &["x"]),
        "invalid_input",
        &["--conflict", "--plan"],
    );
    unchanged(&p, &before);
}

#[test]
fn a_conflict_no_command_raised_has_no_status() {
    let p = twin();
    let run = p.run(&[
        "new",
        "--in",
        "spec/v1.0.spec",
        "--field",
        "phase=v1.0/01",
        "--field",
        "plan=v1.0/01/01-02",
        "--field",
        "dimension=d",
        "--field",
        "property=p",
        "--field",
        "conflicts_with=c",
        "--",
        "revision_conflict",
        "CF1",
        "Hand-made",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of("v1.0/01/CF1");
    assert!(fields.get("status").is_none(), "{fields}");
}
