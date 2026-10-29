//! The discuss, specify and research kinds, end to end, over `tests/projects/discuss/`.

use crate::harness::{Project, project};

/// `(source, target, label)` of every edge `specforge query <name> --depth 1` prints.
fn edges(p: &Project, name: &str) -> Vec<(String, String, String)> {
    let query = p.run(&["query", name, "--depth", "1", "--format", "json"]);
    let graph: serde_json::Value = serde_json::from_str(&query.stdout)
        .unwrap_or_else(|e| panic!("query {name} is JSON ({e}): {}", query.all()));
    let text = |e: &serde_json::Value, key: &str| e[key].as_str().unwrap_or_default().to_string();
    graph["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|e| (text(e, "source"), text(e, "target"), text(e, "label")))
        .collect()
}

fn has_edge(edges: &[(String, String, String)], source: &str, target: &str, label: &str) -> bool {
    edges.contains(&(source.into(), target.into(), label.into()))
}

/// A spec file of the `discuss` project, to edit and write back.
fn fixture(file: &str) -> String {
    let path = format!(
        "{}/tests/projects/discuss/spec/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn checks_clean(p: &Project) {
    p.check_exact(&[]);
}

#[test]
fn the_discuss_project_checks_clean() {
    checks_clean(&project("discuss"));
}

#[test]
fn a_decision_resolves_in_its_phase_and_from_the_project() {
    let p = project("discuss");
    let own = edges(&p, "v1.0/02/D-02");
    assert!(
        has_edge(&own, "v1.0/02/D-02", "v1.0/02/D-01", "superseded_by"),
        "{own:?}"
    );
    let spec = fixture("discuss.spec");
    p.write(
        "spec/discuss.spec",
        &spec.replace("superseded_by D-01", "superseded_by /D-01"),
    );
    checks_clean(&p);
    let rooted = edges(&p, "v1.0/02/D-02");
    assert!(
        has_edge(&rooted, "v1.0/02/D-02", "D-01", "superseded_by"),
        "{rooted:?}"
    );
}

#[test]
fn two_phases_may_each_have_a_d_01() {
    let p = project("discuss");
    p.write(
        "spec/v1.0.spec",
        &(fixture("v1.0.spec")
            + "\nphase 03 \"Sharing\" {\n  milestone v1.0\n  name \"Sharing\"\n  slug \"sharing\"\n  goal \"g\"\n  success_criteria [\"a\", \"b\"]\n}\n"),
    );
    p.write(
        "spec/more.spec",
        "decision D-01 \"Share by link\" {\n  phase v1.0/03\n  text \"A board is shared by link\"\n  origin context\n}\n",
    );
    checks_clean(&p);
    for name in ["v1.0/02/D-01", "v1.0/03/D-01", "D-01"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(query.code, 0, "query {name}: {}", query.all());
        assert!(query.all().contains(name), "query {name}: {}", query.all());
    }
}

#[test]
fn a_context_is_named_02_context_in_its_phase() {
    let p = project("discuss");
    for name in ["v1.0/02/02-CONTEXT", "02-CONTEXT"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(query.code, 0, "query {name}: {}", query.all());
        assert!(
            query.all().contains("v1.0/02/02-CONTEXT"),
            "query {name}: {}",
            query.all()
        );
    }
}

#[test]
fn a_canonical_ref_to_a_missing_file_is_e016() {
    let p = project("discuss");
    let spec = fixture("discuss.spec");
    p.write(
        "spec/discuss.spec",
        &spec.replace("docs/storage.md", "docs/missing.md"),
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(
        check.all().contains("E016") && check.all().contains("docs/missing.md"),
        "check: {}",
        check.all()
    );
}

#[test]
fn project_key_decisions_name_project_decisions() {
    let p = project("discuss");
    let own = edges(&p, "taskboard");
    assert!(
        has_edge(&own, "taskboard", "D-01", "key_decisions"),
        "{own:?}"
    );
}

fn codes(p: &Project) -> Vec<String> {
    p.codes()
}

/// The project with its baseline committed and `edit` applied to `spec/discuss.spec`.
fn edited(from: &str, to: &str) -> Project {
    let p = project("discuss");
    p.commit();
    let spec = fixture("discuss.spec");
    assert!(spec.contains(from), "the fixture holds `{from}`");
    p.write("spec/discuss.spec", &spec.replace(from, to));
    p
}

#[test]
fn a_rewritten_interview_answer_is_e084() {
    let p = edited("In one JSON file", "In SQLite");
    assert_eq!(codes(&p), ["E084"]);
}

#[test]
fn a_removed_interview_is_e085() {
    let p = edited(
        "interview IV1 \"Where are tasks kept?\" {\n  spec        v1.0/02/02-SPEC\n  round       1\n  perspective researcher\n  question    \"Where are tasks kept?\"\n  answer      \"In one JSON file\"\n}\n",
        "",
    );
    assert_eq!(codes(&p), ["E085"]);
}

#[test]
fn an_interview_answer_may_be_set_once() {
    let p = edited(
        "question    \"What if the file is corrupt?\"\n",
        "question    \"What if the file is corrupt?\"\n  answer      \"Start empty and say so\"\n",
    );
    assert_eq!(codes(&p), Vec::<String>::new());
    p.commit();
    let spec = fixture("discuss.spec");
    p.write(
        "spec/discuss.spec",
        &spec.replace(
            "question    \"What if the file is corrupt?\"\n",
            "question    \"What if the file is corrupt?\"\n  answer      \"Refuse to start\"\n",
        ),
    );
    assert_eq!(codes(&p), ["E084"]);
}

#[test]
fn a_score_above_100_is_refused() {
    let p = project("discuss");
    let spec = fixture("discuss.spec");
    let scored = |ambiguity: &str| {
        spec.replace(
            "goal    \"Tasks go from memory-only to saved on disk\"\n",
            &format!("goal    \"Tasks go from memory-only to saved on disk\"\n  scores  {{ goal 90 boundary 80 constraint 70 acceptance 80 ambiguity {ambiguity} }}\n"),
        )
    };
    p.write("spec/discuss.spec", &scored("10"));
    assert_eq!(codes(&p), Vec::<String>::new());
    p.write("spec/discuss.spec", &scored("101"));
    // The kernel's E061 on the scores block: declared integer, given 101.
    assert_eq!(codes(&p), ["E061"]);
}

const FIRST_ENTRY: &str = "    {\n      area     \"Storage\"\n      question \"Where are tasks kept?\"\n      options  [\"One JSON file\", \"SQLite\"]\n      chosen   \"One JSON file\"\n      mode     interactive\n    }\n";
const SECOND_ENTRY: &str = "    {\n      area     \"Safety\"\n      question \"How is the file written?\"\n      options  [\"In place\", \"Atomically\"]\n      chosen   \"Atomically\"\n      mode     interactive\n    }\n";

#[test]
fn a_removed_log_entry_is_refused() {
    let p = edited(FIRST_ENTRY, "");
    assert_eq!(codes(&p), ["E085"]);
    let run = p.run(&["check"]);
    assert!(
        run.all().contains("entries"),
        "names the field: {}",
        run.all()
    );
}

#[test]
fn a_rewritten_log_entry_is_refused() {
    let p = edited("chosen   \"Atomically\"", "chosen   \"In place\"");
    assert_eq!(codes(&p), ["E085"]);
}

#[test]
fn an_appended_log_entry_is_clean() {
    let p = edited(
        SECOND_ENTRY,
        &format!(
            "{SECOND_ENTRY}    {{ area \"Safety\" question \"Keep a backup?\" options [\"Yes\", \"No\"] chosen \"No\" mode advisor }}\n"
        ),
    );
    assert_eq!(codes(&p), Vec::<String>::new());
}

fn queried(p: &Project, name: &str) -> String {
    let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
    assert_eq!(query.code, 0, "query {name}: {}", query.all());
    query.all()
}

#[test]
fn research_items_are_named_within_their_research() {
    let p = project("discuss");
    checks_clean(&p);
    // The phase research's `F1` and the project research's `F1` coexist.
    assert!(queried(&p, "v1.0/02/02-RESEARCH/F1").contains("v1.0/02/02-RESEARCH/F1"));
    assert!(queried(&p, "RESEARCH/F1").contains("RESEARCH/F1"));
    for item in ["SRC1", "SC1", "P1", "PF1", "DH1", "OQ1", "PA1"] {
        let name = format!("v1.0/02/02-RESEARCH/{item}");
        assert!(queried(&p, &name).contains(&name), "{name}");
    }
}

#[test]
fn a_project_research_is_named_research_in_the_project_scope() {
    let p = project("discuss");
    assert!(queried(&p, "RESEARCH").contains("RESEARCH"));
    let own = edges(&p, "RESEARCH/F1");
    assert!(
        has_edge(&own, "RESEARCH/F1", "RESEARCH", "research"),
        "{own:?}"
    );
}

#[test]
fn an_open_question_names_the_decision_that_resolves_it() {
    let own = edges(&project("discuss"), "v1.0/02/02-RESEARCH/OQ1");
    assert!(
        has_edge(
            &own,
            "v1.0/02/02-RESEARCH/OQ1",
            "v1.0/02/D-02",
            "resolved_by"
        ),
        "{own:?}"
    );
}

const AUDIT: &str = "v1.0/02/02-RESEARCH/PA1";

#[test]
fn w205_inputs() {
    let p = project("discuss");
    let inputs = p.inputs(AUDIT, "W205");
    assert_eq!(inputs["verdict"]["state"], "missing", "{inputs}");
    let names: Vec<&str> = inputs["inputs"]
        .as_array()
        .expect("inputs")
        .iter()
        .filter_map(|i| i["name"].as_str())
        .collect();
    assert_eq!(names, ["audit"], "{inputs}");
    // The package has no verdict, so exactly one W205 says so.
    let found = p.report_all();
    assert_eq!(
        found.iter().filter(|(c, _)| c == "gsd/W205").count(),
        1,
        "{found:#?}"
    );
}

#[test]
fn a_hand_written_legitimacy_is_e082() {
    let p = project("discuss");
    let spec = fixture("research.spec");
    let hand = spec.replace(
        "  registry \"builtin\"\n",
        "  registry \"builtin\"\n  legitimacy legitimate\n",
    );
    assert_ne!(spec, hand);
    p.write("spec/research.spec", &hand);
    assert!(p.codes().contains(&"E082".to_string()), "{:?}", p.codes());
    // An unknown value is still the kernel's, on top.
    p.write(
        "spec/research.spec",
        &hand.replace("legitimate\n", "trusted\n"),
    );
    assert!(p.codes().contains(&"E061".to_string()), "{:?}", p.codes());
}

#[test]
fn a_legitimate_verdict_sets_legitimacy() {
    let p = project("discuss");
    assert_eq!(p.fields_of(AUDIT).get("legitimacy"), None);
    let run = p.record(AUDIT, "W205", "legitimate", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(AUDIT)["legitimacy"], "legitimate");
    let found = p.report_all();
    assert!(!found.iter().any(|(c, _)| c == "gsd/W205"), "{found:#?}");
}

#[test]
fn a_suspicious_package_fails_w205() {
    let p = project("discuss");
    let run = p.record(AUDIT, "W205", "suspicious", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(AUDIT)["legitimacy"], "suspicious");
    let found = p.report_all();
    let w205: Vec<_> = found.iter().filter(|(c, _)| c == "gsd/W205").collect();
    assert_eq!(w205.len(), 1, "{found:#?}");
    assert!(w205[0].1.contains("suspicious"), "{found:#?}");
    // Under strict W205 is an Error: the failing verdict would leave a new error, which the host refuses whole.
    let strict = project("discuss");
    strict.settings(serde_json::json!({ "profile": "gsd-strict" }));
    let run = strict.record(AUDIT, "W205", "suspicious", &[]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E080"), "{}", run.all());
}

fn with_canonical_refs(refs: Option<&str>) -> Project {
    let p = project("discuss");
    let spec = fixture("discuss.spec");
    let field = "  canonical_refs [\n    { topic \"Storage\" paths [\"docs/storage.md\"] }\n  ]\n";
    assert!(spec.contains(field), "the fixture holds the refs");
    let replacement = refs.map_or_else(String::new, |r| format!("  canonical_refs {r}\n"));
    p.write("spec/discuss.spec", &spec.replace(field, &replacement));
    p
}

#[test]
fn context_without_refs_is_e201() {
    let found = with_canonical_refs(None).findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/E201");
    assert!(found[0].1.contains("no canonical_refs"), "{found:#?}");
}

#[test]
fn context_with_empty_refs_is_e201() {
    let found = with_canonical_refs(Some("[]")).findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/E201");
    assert!(found[0].1.contains("empty canonical_refs"), "{found:#?}");
}

#[test]
fn a_context_that_says_no_external_specs_is_clean() {
    let p = with_canonical_refs(Some(
        "[\n    { topic \"No external specs — requirements are fully captured in decisions above\" paths [] }\n  ]",
    ));
    checks_clean(&p);
}

fn research_without_resolution() -> Project {
    let p = project("discuss");
    let spec = fixture("research.spec");
    let resolved = "  resolved_by D-02\n";
    assert!(spec.contains(resolved), "the fixture resolves OQ1");
    p.write("spec/research.spec", &spec.replace(resolved, ""));
    p
}

#[test]
fn open_question_without_decision_is_w204() {
    let p = research_without_resolution();
    assert_eq!(codes(&p), ["gsd/W204"], "no kernel code joins it");
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/W204");
    assert!(
        found[0].1.contains("'v1.0/02/02-RESEARCH/OQ1'"),
        "{found:#?}"
    );
}

#[test]
fn a_resolved_open_question_is_clean() {
    assert_eq!(project("discuss").findings(), []);
}

#[test]
fn w204_is_an_error_under_gsd_strict() {
    let p = research_without_resolution();
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(check.all().contains("W204"), "check: {}", check.all());
}

/// The `digest` the compute pass gave the context of phase 02.
fn context_digest(p: &Project) -> String {
    let query = p.run(&[
        "query",
        "v1.0/02/02-CONTEXT",
        "--depth",
        "0",
        "--format",
        "json",
    ]);
    let graph: serde_json::Value = serde_json::from_str(&query.stdout)
        .unwrap_or_else(|e| panic!("query is JSON ({e}): {}", query.all()));
    let node = graph["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["id"] == "v1.0/02/02-CONTEXT"))
        .unwrap_or_else(|| panic!("the context is queried: {}", query.all()));
    let digest = node["fields"]["digest"].as_str().unwrap_or_default();
    assert!(
        digest.starts_with("fnv1a64:"),
        "the context's digest: {node}"
    );
    digest.to_string()
}

/// Writes `context_digest <digest>` after `anchor` in the fixture `file`, without the ledger line.
fn write_digest(p: &Project, file: &str, anchor: &str, digest: &str) {
    let spec = std::fs::read_to_string(format!(
        "{}/tests/projects/discuss/spec/{file}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the fixture");
    assert!(spec.contains(anchor), "the fixture holds `{anchor}`");
    p.write(
        &format!("spec/{file}"),
        &spec.replace(anchor, &format!("{anchor}  context_digest {digest:?}\n")),
    );
}

/// Writes `context_digest <digest>` on the research (or the spec) and records it in the ledger.
fn record(p: &Project, file: &str, entity: &str, anchor: &str, digest: &str) {
    write_digest(p, file, anchor, digest);
    p.record_harness(entity, "context_digest", &format!("{digest:?}"));
}

const RESEARCH_ANCHOR: &str = "research 02-RESEARCH \"Persistence research\" {\n";
const SPEC_ANCHOR: &str = "phase_spec 02-SPEC \"Persistence spec\" {\n";

fn edit_decision(p: &Project) {
    let spec = p.read("spec/discuss.spec");
    let anchor = "Tasks are kept in a single JSON file";
    assert!(spec.contains(anchor), "the fixture holds `{anchor}`");
    p.write(
        "spec/discuss.spec",
        &spec.replace(anchor, "Tasks are kept in SQLite"),
    );
}

#[test]
fn context_changed_after_research_is_w406() {
    let p = project("discuss");
    let digest = context_digest(&p);
    record(
        &p,
        "research.spec",
        "v1.0/02/02-RESEARCH",
        RESEARCH_ANCHOR,
        &digest,
    );
    checks_clean(&p);
    edit_decision(&p);
    assert_eq!(codes(&p), ["gsd/W406"], "no kernel code joins it");
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/W406");
    assert!(
        found[0].1.contains("research 'v1.0/02/02-RESEARCH'")
            && found[0].1.contains("'v1.0/02/02-CONTEXT' changed since"),
        "{found:#?}"
    );
}

#[test]
fn a_spec_recorded_before_its_context_is_w406() {
    let p = project("discuss");
    let before = context_digest(&p);
    record(&p, "discuss.spec", "v1.0/02/02-SPEC", SPEC_ANCHOR, &before);
    checks_clean(&p);
    edit_decision(&p);
    assert_ne!(context_digest(&p), before, "the edit moves the digest");
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/W406");
    assert!(
        found[0].1.contains("phase spec 'v1.0/02/02-SPEC'"),
        "{found:#?}"
    );
}

#[test]
fn a_decision_of_another_origin_does_not_move_the_digest() {
    let p = project("discuss");
    let summary = |text: &str| {
        format!(
            "decision D-09 \"From a summary\" {{\n  phase  v1.0/02\n  text   \"{text}\"\n  origin summary\n}}\n"
        )
    };
    p.write("spec/summary.spec", &summary("first"));
    let digest = context_digest(&p);
    record(
        &p,
        "research.spec",
        "v1.0/02/02-RESEARCH",
        RESEARCH_ANCHOR,
        &digest,
    );
    p.write("spec/summary.spec", &summary("second"));
    checks_clean(&p);
}

#[test]
fn a_hand_written_digest_is_e083() {
    let p = project("discuss");
    let digest = context_digest(&p);
    write_digest(&p, "research.spec", RESEARCH_ANCHOR, &digest);
    assert_eq!(codes(&p), ["E083"]);
    p.record_harness(
        "v1.0/02/02-RESEARCH",
        "context_digest",
        &format!("{digest:?}"),
    );
    assert_eq!(codes(&p), Vec::<String>::new());
}

/// A project whose research was recorded against the context before a decision of the context was edited.
fn stale_research() -> Project {
    let p = project("discuss");
    let digest = context_digest(&p);
    record(
        &p,
        "research.spec",
        "v1.0/02/02-RESEARCH",
        RESEARCH_ANCHOR,
        &digest,
    );
    edit_decision(&p);
    p
}

/// `(severity, exit code)` of the W406 `specforge check` reports, `None` when it reports none.
fn w406(p: &Project) -> Option<(String, i32)> {
    let run = p.run(&["check", "--format", "json"]);
    let found: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    let severity = found
        .as_array()
        .expect("a list")
        .iter()
        .find(|d| d["code"] == "gsd/W406")
        .map(|d| d["severity"].as_str().unwrap_or_default().to_lowercase())?;
    Some((severity, run.code))
}

#[test]
fn context_drift_action_block_raises_w406_to_an_error() {
    let p = stale_research();
    p.settings(serde_json::json!({ "workflow.context_drift_action": "block" }));
    assert_eq!(w406(&p), Some(("error".to_string(), 1)));
}

#[test]
fn context_drift_action_warn_keeps_w406_a_warning() {
    let p = stale_research();
    assert_eq!(w406(&p), Some(("warning".to_string(), 0)), "the default");
    p.settings(serde_json::json!({ "workflow.context_drift_action": "warn" }));
    assert_eq!(w406(&p), Some(("warning".to_string(), 0)));
}

#[test]
fn context_drift_precheck_off_silences_w406() {
    let p = stale_research();
    assert!(
        w406(&p).is_some(),
        "the research is stale before the gate is turned off"
    );
    p.settings(serde_json::json!({ "workflow.context_drift_precheck": false }));
    assert_eq!(w406(&p), None);
    p.settings(serde_json::json!({
        "workflow.context_drift_precheck": false,
        "workflow.context_drift_action": "block"
    }));
    assert_eq!(w406(&p), None, "the gate is off whatever the action");
}

#[test]
fn w406_is_an_error_under_gsd_strict() {
    let p = stale_research();
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    assert_eq!(w406(&p), Some(("error".to_string(), 1)));
}

/// The project with `03-RESEARCH` (no `phase`) and an unresolved question of it, as a forgetful author writes them.
fn phaseless_research(name: &str) -> Project {
    let p = project("discuss");
    p.write(
        "spec/lost.spec",
        &format!("research {name} \"Forgot its phase\" {{\n  confidence high\n  summary \"x\"\n}}\n\nopen_question OQ9 \"Open\" {{\n  research {name}\n  question \"q\"\n  unclear \"u\"\n}}\n"),
    );
    p
}

#[test]
fn a_research_without_phase_that_is_not_named_research_is_e206() {
    let p = phaseless_research("03-RESEARCH");
    // E206 blames the stray research, not the project's `RESEARCH` (it names only `03-RESEARCH` as the culprit); its unresolved question is W204 as a phase's is.
    assert_eq!(codes(&p), ["gsd/E206", "gsd/W204"]);
    let found = p.findings();
    assert!(
        found[0].1.contains("research '03-RESEARCH' has no phase"),
        "{found:#?}"
    );
    assert!(found[1].1.contains("'03-RESEARCH/OQ9'"), "{found:#?}");
}

#[test]
fn a_research_without_phase_is_e206_even_when_the_project_has_none() {
    let p = phaseless_research("03-RESEARCH");
    p.write(
        "spec/research.spec",
        fixture("research.spec")
            .split("research RESEARCH ")
            .next()
            .unwrap_or_default(),
    );
    assert!(
        p.codes().contains(&"gsd/E206".to_string()),
        "{:?}",
        p.codes()
    );
}
