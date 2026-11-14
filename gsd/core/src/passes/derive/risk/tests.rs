use serde_json::json;

use super::*;

const NO_POST: Post = Post {
    severity: None,
    occurrence: None,
    detection: None,
};

fn sod(severity: i64, occurrence: i64, detection: i64) -> Sod {
    Sod {
        severity,
        occurrence,
        detection,
    }
}

/// A risk with its three ratings as written, and its `post` block's members when `post` is `Some`.
fn risk(id: &str, ratings: [&str; 3], post: Option<serde_json::Value>) -> PassEntity {
    let mut fields: std::collections::BTreeMap<String, String> =
        ["severity", "occurrence", "detection"]
            .into_iter()
            .zip(ratings)
            .filter(|(_, written)| !written.is_empty())
            .map(|(name, written)| (name.to_string(), written.to_string()))
            .collect();
    let mut values = std::collections::BTreeMap::new();
    if let Some(post) = post {
        let keys: Vec<&String> = post.as_object().expect("a block").keys().collect();
        fields.insert(
            "post".into(),
            keys.iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
        values.insert("post".to_string(), post);
    }
    PassEntity {
        id: id.into(),
        kind: "risk".into(),
        fields,
        values,
        ..Default::default()
    }
}

/// The values `values_and_findings` answers under the default threshold, with no commit and no plan status.
fn rated_values(input: &PassInput) -> Vec<PassValue> {
    values_and_findings(input, &[], &BTreeMap::new(), 100).0
}

fn found(values: &[PassValue], entity: &str, field: &str) -> Option<serde_json::Value> {
    values
        .iter()
        .find(|v| v.entity == entity && v.field == field)
        .map(|v| v.value.clone())
}

#[test]
fn rpn_is_the_product_of_severity_occurrence_and_detection() {
    assert_eq!(rate(sod(7, 5, 5), NO_POST, &[], 100).rpn, 175);
    assert_eq!(rate(sod(1, 1, 1), NO_POST, &[], 100).rpn, 1);
    assert_eq!(rate(sod(10, 10, 10), NO_POST, &[], 100).rpn, 1000);
    assert_eq!(
        rate(sod(7, 5, 5), NO_POST, &[], 100).post_rpn,
        175,
        "no post block"
    );
}

#[test]
fn a_post_rating_left_out_takes_its_pre_value() {
    let occurrence = Post {
        occurrence: Some(1),
        ..NO_POST
    };
    assert_eq!(rate(sod(7, 5, 5), occurrence, &[], 100).post_rpn, 35);
    let two = Post {
        severity: Some(3),
        detection: Some(2),
        ..NO_POST
    };
    assert_eq!(rate(sod(5, 5, 5), two, &[], 100).post_rpn, 30);
    assert_eq!(
        rate(sod(5, 5, 5), two, &[], 100).rpn,
        125,
        "the pre rpn is the pre ratings'"
    );
}

#[test]
fn a_risk_with_a_rating_missing_or_outside_one_to_ten_gets_no_value() {
    for (ratings, post) in [
        (["", "5", "5"], None),
        (["7", "5", "11"], None),
        (["0", "5", "5"], None),
        (["seven", "5", "5"], None),
        (["7", "5", "5"], Some(json!({ "occurrence": 0 }))),
        (["7", "5", "5"], Some(json!({ "detection": 11 }))),
    ] {
        let input = PassInput {
            entities: vec![risk("RK1", ratings, post.clone())],
            ..Default::default()
        };
        assert_eq!(rated_values(&input), Vec::new(), "{ratings:?} {post:?}");
    }
}

#[test]
fn values_answers_rpn_and_post_rpn_for_each_rated_risk() {
    let input = PassInput {
        entities: vec![
            risk("RK1", ["7", "5", "5"], Some(json!({ "occurrence": 1 }))),
            risk("RK2", ["2", "3", "1"], None),
            risk("RK3", ["10", "", "7"], None),
        ],
        ..Default::default()
    };
    let out = rated_values(&input);
    assert_eq!(found(&out, "RK1", "rpn"), Some(json!(175)));
    assert_eq!(found(&out, "RK1", "post_rpn"), Some(json!(35)));
    assert_eq!(found(&out, "RK2", "rpn"), Some(json!(6)));
    assert_eq!(found(&out, "RK2", "post_rpn"), Some(json!(6)));
    assert_eq!(
        found(&out, "RK3", "rpn"),
        None,
        "a required rating is missing"
    );
    assert_eq!(
        out.len(),
        8,
        "rpn, post_rpn, status and blocking of RK1 and RK2"
    );
}

const PLAN: &str = "v1.0/01/01-01";

fn rate_7_5_5(done: &[bool]) -> Rating {
    rate(
        sod(7, 5, 5),
        Post {
            occurrence: Some(1),
            ..NO_POST
        },
        done,
        100,
    )
}

/// A phase with plan 01-01, its task T1 and truth TR1, the phase's verification, and `risks`; each risk's
/// `mitigated_by` resolves to the full ids it lists that exist, as the host's edges would.
fn project(risks: Vec<serde_json::Value>, truth_judgements: Vec<serde_json::Value>) -> PassInput {
    let mut entities = vec![
        json!({ "id": "v1.0/01", "kind": "phase" }),
        json!({ "id": PLAN, "kind": "plan", "fields": { "phase": "v1.0/01" } }),
        json!({ "id": format!("{PLAN}/T1"), "kind": "task", "fields": { "plan": PLAN } }),
        json!({ "id": format!("{PLAN}/TR1"), "kind": "truth", "fields": { "plan": PLAN },
                "judgements": truth_judgements }),
        json!({ "id": "v1.0/01/01-VERIFICATION", "kind": "verification", "fields": { "phase": "v1.0/01" } }),
    ];
    let ids: Vec<String> = entities
        .iter()
        .map(|e| e["id"].as_str().expect("an id").to_string())
        .collect();
    let mut edges = vec![json!({ "source": PLAN, "target": "v1.0/01", "label": "phase" })];
    for risk in &risks {
        for target in risk["values"]["mitigated_by"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let target = target.as_str().expect("an id");
            if ids.iter().any(|id| id == target) {
                edges.push(
                    json!({ "source": risk["id"], "target": target, "label": "mitigated_by" }),
                );
            }
        }
    }
    entities.extend(risks);
    serde_json::from_value(crate::passes::plans::with_field_edges(
        json!({ "entities": entities, "edges": edges }),
    ))
    .expect("a pass input")
}

/// A risk rated 7/5/5 with `post { occurrence 1 }` (rpn 175, post_rpn 35) naming `mitigated_by`.
fn named(id: &str, mitigated_by: &[&str]) -> serde_json::Value {
    let joined = mitigated_by.join(", ");
    json!({ "id": id, "kind": "risk",
            "fields": { "severity": "7", "occurrence": "5", "detection": "5", "post": "occurrence",
                        "mitigated_by": joined },
            "values": { "post": { "occurrence": 1 }, "mitigated_by": mitigated_by } })
}

fn verdict(outcome: &str, passed: bool) -> serde_json::Value {
    json!({ "rule": "gsd/E501", "state": "fresh", "passed": passed, "outcome": outcome,
            "at": "2026-10-29T10:00:00Z", "files": [] })
}

fn status_of(
    input: &PassInput,
    commits: &[Commit],
    statuses: &[(&str, &'static str)],
    id: &str,
) -> String {
    let statuses: BTreeMap<&str, &'static str> = statuses.iter().copied().collect();
    let (values, _) = values_and_findings(input, commits, &statuses, 100);
    found(&values, id, "status")
        .and_then(|s| s.as_str().map(String::from))
        .unwrap_or_else(|| panic!("{id} has no status: {values:?}"))
}

#[test]
fn a_risk_naming_no_mitigation_is_open() {
    assert_eq!(rate_7_5_5(&[]).status, Status::Open);
    assert_eq!(Status::Open.as_str(), "open");
}

#[test]
fn a_risk_is_mitigated_once_every_mitigation_is_done() {
    assert_eq!(rate_7_5_5(&[true, true]).status, Status::Mitigated);
    assert_eq!(rate_7_5_5(&[true, false]).status, Status::Mitigating);
    assert_eq!(rate_7_5_5(&[false]).status, Status::Mitigating);
    assert_eq!(Status::Mitigating.as_str(), "mitigating");
    assert_eq!(Status::Mitigated.as_str(), "mitigated");
}

#[test]
fn an_unresolved_mitigation_is_never_done() {
    let task = format!("{PLAN}/T1");
    let commit = Commit {
        task: Some(task.clone()),
        ..Default::default()
    };
    let input = project(
        vec![
            named("RK1", &[&task]),
            named("RK2", &[&task, "v1.0/01/01-01/T9"]),
        ],
        Vec::new(),
    );
    let commits = [commit];
    assert_eq!(status_of(&input, &commits, &[], "RK1"), "mitigated");
    assert_eq!(status_of(&input, &commits, &[], "RK2"), "mitigating");
}

#[test]
fn a_mitigated_risk_blocks_on_its_post_rpn_and_another_on_its_rpn() {
    let mitigated = rate_7_5_5(&[true]);
    assert_eq!((mitigated.current, mitigated.blocking), (35, false));
    let mitigating = rate_7_5_5(&[false]);
    assert_eq!((mitigating.current, mitigating.blocking), (175, true));
    let open = rate_7_5_5(&[]);
    assert_eq!(
        (open.current, open.blocking, open.unmitigated),
        (175, true, true)
    );
    let residual = rate(
        sod(10, 10, 10),
        Post {
            occurrence: Some(5),
            ..NO_POST
        },
        &[true],
        100,
    );
    assert_eq!(
        (residual.current, residual.blocking),
        (500, true),
        "a residual at the threshold blocks"
    );
    let at = rate(sod(4, 5, 5), NO_POST, &[], 100);
    assert_eq!(
        (at.blocking, at.unmitigated),
        (true, true),
        "at the threshold"
    );
    let below = rate(sod(3, 5, 5), NO_POST, &[], 100);
    assert_eq!(
        (below.blocking, below.unmitigated),
        (false, false),
        "one below"
    );
    assert!(
        !rate_7_5_5(&[false]).unmitigated,
        "naming a mitigation is not unmitigated"
    );
}

#[test]
fn a_task_mitigation_is_done_by_its_commit_or_its_complete_plan() {
    let task = format!("{PLAN}/T1");
    let input = project(vec![named("RK1", &[&task])], Vec::new());
    assert_eq!(status_of(&input, &[], &[], "RK1"), "mitigating");
    let by_commit = Commit {
        task: Some(task.clone()),
        ..Default::default()
    };
    assert_eq!(status_of(&input, &[by_commit], &[], "RK1"), "mitigated");
    assert_eq!(
        status_of(&input, &[], &[(PLAN, "complete")], "RK1"),
        "mitigated"
    );
    assert_eq!(
        status_of(&input, &[], &[(PLAN, "executing")], "RK1"),
        "mitigating"
    );
}

#[test]
fn a_truth_mitigation_is_done_once_its_verification_verifies_it() {
    let truth = format!("{PLAN}/TR1");
    let unverified = project(vec![named("RK1", &[&truth])], Vec::new());
    assert_eq!(status_of(&unverified, &[], &[], "RK1"), "mitigating");
    let failed = project(
        vec![named("RK1", &[&truth])],
        vec![verdict("failed", false)],
    );
    assert_eq!(status_of(&failed, &[], &[], "RK1"), "mitigating");
    let verified = project(
        vec![named("RK1", &[&truth])],
        vec![verdict("verified", true)],
    );
    assert_eq!(status_of(&verified, &[], &[], "RK1"), "mitigated");
    let (values, _) = values_and_findings(&verified, &[], &BTreeMap::new(), 100);
    assert_eq!(
        found(&values, "RK1", "blocking"),
        Some(json!(false)),
        "post_rpn 35"
    );
}
