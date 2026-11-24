//! What `import` refuses before it plans (ADR 0391 D4, D9, D10), and what it drops and reports, through the command in
//! process.

mod common;

use serde_json::{Value, json};
use specforge_extension_sdk::PlanOp;
use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::testing::MockHost;

use common::{fact, feed, import, manifest, payload};
use gsd_import::kinds::{CORE_KINDS, REQUIRED, required};
use gsd_import::limits::{MAX_ENTRIES, MAX_OPS};
use gsd_import::records::{Record, Records, Source};
use gsd_import::report::Report;

/// A `REQUIREMENTS.md` of one v1 category holding `bullets`, each `- [ ] **<id>**: <text>`; bullet `i` (from 0) is on
/// line `6 + i`.
fn requirements(bullets: &[(&str, &str)]) -> String {
    let mut text = "# Requirements\n\n## v1 Requirements\n\n### Core\n".to_string();
    for (id, words) in bullets {
        text.push_str(&format!("- [ ] **{id}**: {words}\n"));
    }
    text
}

fn planning(text: &str) -> Vec<Value> {
    vec![fact(".planning/REQUIREMENTS.md", text)]
}

/// `import` on an empty graph, the default manifest and `planning`.
fn run(planning: Vec<Value>) -> CommandOutput {
    import(
        &CommandGraph::default(),
        &feed(planning, false, Some(&manifest(&["@gsd/core"]))),
    )
}

fn created(out: &CommandOutput) -> Vec<String> {
    out.plan
        .as_ref()
        .expect("a plan")
        .ops
        .iter()
        .map(|op| match op {
            PlanOp::Create { id, .. } => id.clone(),
            other => panic!("import answers creates only, not {other:?}"),
        })
        .collect()
}

fn entries(out: &CommandOutput) -> Vec<Value> {
    payload(out)["entries"].as_array().expect("entries").clone()
}

fn refused(out: &CommandOutput, says: &str) {
    assert_eq!(out.exit_code, 1, "{}", out.stdout);
    assert!(out.plan.is_none(), "a refusal answers no plan");
    assert!(out.stderr.contains(says), "{says:?} not in {}", out.stderr);
}

fn node(id: &str, kind: &str, file: &str) -> GraphNode {
    GraphNode {
        id: id.into(),
        kind: kind.into(),
        file: Some(file.into()),
        ..GraphNode::default()
    }
}

#[test]
fn reimport_is_refused_naming_one_core_entity() {
    let graph = CommandGraph::new(
        vec![
            node("hello", "spec", "spec/hello.spec"),
            node("TASK-01", "requirement", "spec/gsd/requirements.spec"),
        ],
        Vec::new(),
    );
    let out = import(
        &graph,
        &feed(
            planning(&requirements(&[("TASK-02", "b")])),
            false,
            Some(&manifest(&[])),
        ),
    );
    refused(&out, "requirement TASK-01 in spec/gsd/requirements.spec");
}

#[test]
fn reimport_refusal_ignores_an_entity_of_another_extension() {
    let graph = CommandGraph::new(vec![node("hello", "spec", "spec/hello.spec")], Vec::new());
    let out = import(
        &graph,
        &feed(
            planning(&requirements(&[("TASK-02", "b")])),
            false,
            Some(&manifest(&[])),
        ),
    );
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(created(&out), ["TASK-02"]);
}

#[test]
fn a_spec_root_other_than_spec_is_refused() {
    let text = json!({"name": "p", "spec_root": "specs"}).to_string();
    let out = import(
        &CommandGraph::default(),
        &feed(planning(&requirements(&[])), false, Some(&text)),
    );
    refused(&out, "the spec root 'specs'");
}

#[test]
fn a_spec_root_absent_is_spec() {
    let text = json!({"name": "p"}).to_string();
    let out = import(
        &CommandGraph::default(),
        &feed(
            planning(&requirements(&[("A-01", "a")])),
            false,
            Some(&text),
        ),
    );
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
}

#[test]
fn a_manifest_that_is_not_json_or_missing_is_refused() {
    let out = import(
        &CommandGraph::default(),
        &feed(planning(&requirements(&[])), false, Some("{ not json")),
    );
    refused(&out, "specforge.json is not JSON");
    let out = import(
        &CommandGraph::default(),
        &feed(planning(&requirements(&[])), false, None),
    );
    refused(&out, "specforge.json was not read");
}

#[test]
fn security_is_a_text_check_of_the_manifest() {
    let loaded = |extensions: &[&str]| {
        let out = import(
            &CommandGraph::default(),
            &feed(
                planning(&requirements(&[("A-01", "a")])),
                false,
                Some(&manifest(extensions)),
            ),
        );
        let security = payload(&out)["security"].clone();
        assert!(
            security["check"]
                .as_str()
                .expect("check")
                .contains("text check"),
            "{security}"
        );
        security["loaded"].clone()
    };
    assert_eq!(loaded(&["@gsd/core", "@gsd/security"]), true);
    assert_eq!(
        loaded(&["@gsd/core", "@gsd/security=../security.wasm"]),
        true
    );
    assert_eq!(loaded(&["@gsd/core", "@gsd/securityx"]), false);
    assert_eq!(loaded(&[]), false);
}

#[test]
fn no_planning_folder_is_refused() {
    refused(&run(Vec::new()), "no .planning/ at the project root");
}

#[test]
fn a_truncated_read_is_refused() {
    let out = import(
        &CommandGraph::default(),
        &feed(
            planning(&requirements(&[("A-01", "a")])),
            true,
            Some(&manifest(&[])),
        ),
    );
    refused(&out, &format!("truncated at {FILE_READ_MAX}"));
}

#[test]
fn a_read_error_on_a_mapped_file_is_refused() {
    for error in [
        "budget",
        "too_large",
        "not_utf8",
        "unreadable",
        "outside_project",
    ] {
        let failed = json!({"path": ".planning/phases/01-core/01-01-PLAN.md", "exists": true, "error": error});
        let out = run(vec![
            fact(".planning/REQUIREMENTS.md", &requirements(&[("A-01", "a")])),
            failed,
        ]);
        refused(
            &out,
            &format!(".planning/phases/01-core/01-01-PLAN.md was not read ({error})"),
        );
    }
}

#[test]
fn a_read_error_on_another_file_is_reported_and_the_plan_answered() {
    // The host answers a secret-looking name with a fact that holds no text (13-02).
    let denied = json!({"path": ".planning/.env", "exists": true, "error": "denied"});
    let large = json!({"path": ".planning/notes/big.txt", "exists": true, "error": "too_large"});
    let out = run(vec![
        denied,
        large,
        fact(".planning/REQUIREMENTS.md", &requirements(&[("A-01", "a")])),
    ]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(created(&out), ["A-01"]);
    let entries = entries(&out);
    for (path, error) in [
        (".planning/.env", "denied"),
        (".planning/notes/big.txt", "too_large"),
    ] {
        let entry = entries
            .iter()
            .find(|e| e["file"] == path)
            .unwrap_or_else(|| panic!("{path} is reported"));
        assert_eq!(entry["outcome"], "unmapped");
        assert!(
            entry["reason"].as_str().expect("a reason").contains(error),
            "{entry}"
        );
    }
}

#[test]
fn a_plan_past_max_ops_is_truncated() {
    let ids: Vec<String> = (1..=MAX_OPS + 1).map(|n| format!("A-{n:04}")).collect();
    let at_cap: Vec<(&str, &str)> = ids[..MAX_OPS].iter().map(|id| (id.as_str(), "x")).collect();
    let out = run(planning(&requirements(&at_cap)));
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(created(&out).len(), MAX_OPS);

    let past: Vec<(&str, &str)> = ids.iter().map(|id| (id.as_str(), "x")).collect();
    refused(
        &run(planning(&requirements(&past))),
        &format!("truncated at {MAX_OPS}"),
    );
}

#[test]
fn a_report_past_max_entries_is_truncated() {
    let mut files: Vec<Value> = (0..=MAX_ENTRIES)
        .map(|n| fact(&format!(".planning/notes/{n:05}.txt"), ""))
        .collect();
    files.push(fact(".planning/REQUIREMENTS.md", &requirements(&[])));
    refused(&run(files), &format!("truncated at {MAX_ENTRIES}"));
}

#[test]
fn a_refused_id_is_reported_at_its_line_and_the_rest_imported() {
    let bad = [
        "TASK 01",
        "A/B-01",
        "..-01",
        "T\u{7}-01",
        "TASK-\u{661}",
        "task-01",
    ];
    let mut bullets: Vec<(&str, &str)> = bad.iter().map(|id| (*id, "words")).collect();
    bullets.push(("TASK-02", "kept"));
    let out = run(planning(&requirements(&bullets)));
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(created(&out), ["TASK-02"]);
    let entries = entries(&out);
    for (i, _) in bad.iter().enumerate() {
        let line = 6 + i;
        let entry = entries
            .iter()
            .find(|e| e["line"] == line && e["outcome"] == "unmapped")
            .unwrap_or_else(|| panic!("line {line} is reported: {entries:#?}"));
        let reason = entry["reason"].as_str().expect("a reason");
        assert!(reason.contains("not an id a requirement takes"), "{reason}");
        assert!(
            !reason.contains('\u{7}'),
            "a control character is escaped: {reason:?}"
        );
    }
}

#[test]
fn a_second_record_of_one_id_is_dropped_and_reported() {
    let out = run(planning(&requirements(&[
        ("TASK-01", "first"),
        ("TASK-01", "second"),
    ])));
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(created(&out), ["TASK-01"]);
    let entries = entries(&out);
    assert!(
        entries.iter().any(|e| e["line"] == 7
            && e["reason"]
                .as_str()
                .expect("a reason")
                .contains("given twice (first at .planning/REQUIREMENTS.md:6)")),
        "{entries:#?}"
    );
}

/// A record of `kind` holding each of its required fields.
fn record(kind: &'static str, id: &str, refs: &[&str], line: usize) -> Record {
    Record {
        kind,
        id: id.into(),
        title: id.into(),
        file: "spec/gsd/x.spec".into(),
        fields: required(kind)
            .iter()
            .map(|f| (f.to_string(), json!("x")))
            .collect(),
        refs: refs.iter().map(|r| r.to_string()).collect(),
        source: Source {
            file: ".planning/x.md".into(),
            line,
        },
    }
}

#[test]
fn a_reference_to_nothing_drops_the_record_and_its_children() {
    let mut report = Report::default();
    let mut records = Records::default();
    // The plan names a phase nobody creates; its task names the plan; a requirement names a graph node.
    records.add(record("plan", "09-01", &["09"], 1), &mut report);
    records.add(record("task", "T1", &["09-01"], 2), &mut report);
    records.add(record("truth", "TR1", &["T1"], 3), &mut report);
    records.add(record("requirement", "A-01", &["outside"], 4), &mut report);
    // An id the graph already holds is the host's duplicate: dropped too.
    records.add(record("requirement", "B-01", &[], 5), &mut report);
    let graph = CommandGraph::new(
        vec![
            node("B-01", "spec_item", "spec/x.spec"),
            node("outside", "spec_item", "spec/x.spec"),
        ],
        Vec::new(),
    );
    let plan = PlanBuilder::new(Default::default(), Default::default());
    let (plan, counts, _) =
        gsd_import::emit::emit(&records, &graph, plan, &mut report).expect("not refused");
    let ids: Vec<String> = plan
        .finish()
        .ops
        .iter()
        .map(|op| match op {
            PlanOp::Create { id, .. } => id.clone(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(ids, ["A-01"]);
    assert_eq!(counts.get("requirement"), Some(&1));
    let reason = |line: usize| {
        report
            .entries()
            .into_iter()
            .find(|e| e.line == Some(line))
            .map(|e| e.reason.clone())
            .unwrap_or_default()
    };
    assert!(
        reason(1).contains("names 09, which neither the import nor the graph holds"),
        "{}",
        reason(1)
    );
    assert!(
        reason(2).contains("parent not imported (09-01)"),
        "{}",
        reason(2)
    );
    assert!(
        reason(3).contains("parent not imported (T1)"),
        "{}",
        reason(3)
    );
    assert!(
        reason(5).contains("the graph already holds spec_item B-01"),
        "{}",
        reason(5)
    );
}

#[test]
fn the_plan_creates_kinds_in_order_and_ids_within_a_kind() {
    let mut report = Report::default();
    let mut records = Records::default();
    records.add(record("task", "T2", &["01-01"], 1), &mut report);
    records.add(record("task", "T1", &["01-01"], 2), &mut report);
    records.add(record("plan", "01-01", &["01"], 3), &mut report);
    records.add(record("phase", "01", &["v1.0"], 4), &mut report);
    records.add(record("milestone", "v1.0", &[], 5), &mut report);
    records.add(record("requirement", "A-01", &[], 6), &mut report);
    let plan = PlanBuilder::new(Default::default(), Default::default());
    let (plan, _, _) =
        gsd_import::emit::emit(&records, &CommandGraph::default(), plan, &mut report)
            .expect("not refused");
    let ids: Vec<String> = plan
        .finish()
        .ops
        .iter()
        .map(|op| match op {
            PlanOp::Create { id, .. } => id.clone(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(ids, ["v1.0", "A-01", "01", "01-01", "T1", "T2"]);
}

#[test]
fn core_kinds_are_what_gsd_core_declares() {
    let host = MockHost::new(gsd_core::build());
    let entities: Value =
        serde_json::from_str(&host.describe_json("entities").expect("the entities"))
            .expect("describe answers JSON");
    let mut declared: Vec<&str> = entities["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|kind| kind["name"].as_str().expect("a name"))
        .collect();
    declared.sort_unstable();
    assert_eq!(declared, CORE_KINDS);
}

/// The ids of a plan's creates, in order.
fn create_ids(plan: PlanBuilder) -> Vec<String> {
    plan.finish()
        .ops
        .iter()
        .map(|op| match op {
            PlanOp::Create { id, .. } => id.clone(),
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn required_fields_are_what_the_kinds_declare() {
    let mut declared: Vec<(String, Vec<String>)> = Vec::new();
    for build in [gsd_core::build(), gsd_security::build()] {
        let entities: Value = serde_json::from_str(
            &MockHost::new(build)
                .describe_json("entities")
                .expect("the entities"),
        )
        .expect("describe answers JSON");
        for kind in entities["items"].as_array().expect("items") {
            let fields: Vec<String> = kind["fields"]
                .as_array()
                .expect("fields")
                .iter()
                .filter(|f| {
                    f["required"] == true
                        && !matches!(f["writer"].as_str(), Some("computed" | "harness"))
                })
                .map(|f| f["name"].as_str().expect("a name").to_string())
                .collect();
            if !fields.is_empty() {
                declared.push((kind["name"].as_str().expect("a name").to_string(), fields));
            }
        }
    }
    let listed: Vec<(String, Vec<String>)> = REQUIRED
        .iter()
        .map(|(k, f)| (k.to_string(), f.iter().map(|f| f.to_string()).collect()))
        .collect();
    assert_eq!(listed, declared);
}

#[test]
fn a_record_missing_a_required_field_is_dropped_with_its_children() {
    let mut report = Report::default();
    let mut records = Records::default();
    let mut phase = record("phase", "v1.0/01", &["v1.0"], 2);
    phase.fields.remove("goal");
    records.add(record("milestone", "v1.0", &[], 1), &mut report);
    records.add(phase, &mut report);
    records.add(
        record("plan", "v1.0/01/01-01", &["v1.0/01"], 3),
        &mut report,
    );
    let plan = PlanBuilder::new(Default::default(), Default::default());
    let (plan, _, _) =
        gsd_import::emit::emit(&records, &CommandGraph::default(), plan, &mut report)
            .expect("not refused");
    assert_eq!(create_ids(plan), ["v1.0"]);
    let reasons: Vec<(Option<usize>, String)> = report
        .entries()
        .into_iter()
        .map(|e| (e.line, e.reason.clone()))
        .collect();
    assert!(
        reasons.contains(&(
            Some(2),
            "phase v1.0/01: required goal has no source: not imported".to_string()
        )),
        "{reasons:#?}"
    );
    assert!(
        reasons.contains(&(
            Some(3),
            "plan v1.0/01/01-01: parent not imported (v1.0/01): not imported".to_string()
        )),
        "{reasons:#?}"
    );
}

#[test]
fn a_scoped_record_is_created_with_its_own_id() {
    let mut report = Report::default();
    let mut records = Records::default();
    records.add(record("milestone", "v1.0", &[], 1), &mut report);
    records.add(record("phase", "v1.0/02.1", &["v1.0"], 2), &mut report);
    records.add(record("phase", "v1.0/0 2", &["v1.0"], 3), &mut report);
    let plan = PlanBuilder::new(Default::default(), Default::default());
    let (plan, _, _) =
        gsd_import::emit::emit(&records, &CommandGraph::default(), plan, &mut report)
            .expect("not refused");
    assert_eq!(create_ids(plan), ["v1.0", "02.1"]);
    assert!(
        report
            .entries()
            .iter()
            .any(|e| e.line == Some(3) && e.reason.contains("not an id a phase takes"))
    );
}

#[test]
fn a_record_holding_text_no_spec_file_holds_is_dropped_with_its_children() {
    let mut report = Report::default();
    let mut records = Records::default();
    records.add(record("milestone", "v1.0", &[], 1), &mut report);
    let mut phase = record("phase", "v1.0/01", &["v1.0"], 2);
    phase.title = "Core\u{1b}[31m".into();
    records.add(phase, &mut report);
    records.add(
        record("plan", "v1.0/01/01-01", &["v1.0/01"], 3),
        &mut report,
    );
    let mut bidi = record("requirement", "A-01", &[], 4);
    bidi.fields
        .insert("text".into(), json!(["fine", {"note": "x\u{202e}y"}]));
    records.add(bidi, &mut report);
    // A tab and a line break, CRLF included, are a .spec file's own.
    let mut lines = record("requirement", "A-02", &[], 5);
    lines.title = "a\tb".into();
    lines
        .fields
        .insert("text".into(), json!("one\r\ntwo\nthree"));
    records.add(lines, &mut report);
    let plan = PlanBuilder::new(Default::default(), Default::default());
    let (plan, _, _) =
        gsd_import::emit::emit(&records, &CommandGraph::default(), plan, &mut report)
            .expect("not refused");
    assert_eq!(create_ids(plan), ["v1.0", "A-02"]);
    let reason = |line: usize| {
        report
            .entries()
            .into_iter()
            .find(|e| e.line == Some(line))
            .map(|e| e.reason.clone())
            .unwrap_or_default()
    };
    let unwritable = "holds a control or bidirectional character, which no .spec file holds";
    assert_eq!(
        reason(2),
        format!("phase v1.0/01: its title {unwritable}: not imported")
    );
    assert!(
        reason(3).contains("parent not imported (v1.0/01)"),
        "{}",
        reason(3)
    );
    assert_eq!(
        reason(4),
        format!("requirement A-01: its text {unwritable}: not imported")
    );
}
