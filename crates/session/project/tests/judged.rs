//! Judged rules (ADR 0290): what a project with none reads, and the pins of the ones it will.

use std::sync::Arc;

use specforge_project::testing::GSD_JUDGED_SPEC;
use specforge_project::{SharedRuntime, UpdateKind, Watched};
use specforge_test::prelude::*;

use crate::check_passes::{SPEC, passes_extension, project};

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a project with no judged rule reads no report and stamps no file"
)]
fn a_project_without_judged_rules_reads_no_report_and_stamps_no_file() {
    let dir = project(SPEC).file("specforge-report.json", "{}");
    let ext = Arc::new(passes_extension());
    let session = dir.session_in(Arc::clone(&ext) as SharedRuntime);
    let report = dir.root().join("specforge-report.json");
    let watched = session.inputs().watched();
    assert!(!watched.contains(&Watched::File(report)), "{watched:?}");
    let codes: Vec<_> = session
        .project()
        .diagnostics()
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        !codes.iter().any(|c| c == "W168" || c == "I205"),
        "{codes:?}"
    );
}

// ---- The pins of K7 (ADR 0290): each is ignored with the ticket that flips it. ----

use serde_json::json;
use specforge_common::Diagnostic;
use specforge_project::judged::Cited;
use specforge_project::testing::{GSD_JUDGED, TestProject, gsd_judged_project, gsd_judged_runtime};
use specforge_project::{CompiledProject, harness};
use specforge_registry::PlainDisk;

fn compile(project: &TestProject) -> CompiledProject {
    project.compile_in(Arc::new(gsd_judged_runtime()) as SharedRuntime)
}

fn found<'a>(diagnostics: &'a [Diagnostic], code: &str) -> Vec<&'a Diagnostic> {
    diagnostics.iter().filter(|d| d.code == code).collect()
}

/// Records a verdict of `rule` on `entity` (the entity's block in `a.spec`) as `set_verdict` would: the statement
/// with the inputs the host computes now, and its line in `specforge-harness`.
fn record(project: &TestProject, entity: &str, rule: &str, outcome: &str) {
    let compiled = compile(project);
    let env = compiled.environment();
    let judged = env.registries().judged_rule(rule).expect("a judged rule");
    let node = compiled
        .graph()
        .node(entity)
        .expect("the entity is declared");
    let now =
        specforge_project::judged::inputs(&compiled, &PlainDisk, entity, judged, &Cited::default())
            .expect("the rule judges the entity");
    let digests: String = now
        .digests
        .iter()
        .map(|(name, digest)| format!("      {name} \"{digest}\"\n"))
        .collect();
    let statement = format!(
        "  verdict {rule} {outcome} {{\n    judge \"j\"\n    model \"m\"\n    at \"2026-10-26\"\n    inputs {{\n{digests}    }}\n  }}\n"
    );
    let head = format!(
        "{} {} ",
        node.kind.raw,
        entity.rsplit('/').next().unwrap_or(entity)
    );
    let mut text = project.read("a.spec");
    let start = text.find(&head).expect("the entity's block");
    let mut close = start + text[start..].find("\n}\n").expect("a block") + 1;
    // Recording again replaces the verdict of the rule.
    if let Some(at) = text[start..close].find(&format!("\n  verdict {rule} ")) {
        let from = start + at + 1;
        let to = from + text[from..].find("\n  }\n").expect("a verdict") + 6;
        text.replace_range(from..to, "");
        close -= to - from;
    }
    text.insert_str(close, &statement);
    project.write("a.spec", text);
    let compiled = compile(project);
    let node = compiled
        .graph()
        .node(entity)
        .expect("the entity is declared");
    let verdict = node
        .verdicts
        .iter()
        .find(|v| v.rule == rule)
        .expect("the verdict");
    let key = format!("verdict:{rule}");
    let bytes = specforge_graph::wire::verdict_bytes(compiled.graph(), node, verdict);
    let file = project.root().join("specforge-harness");
    let mut ledger =
        harness::Ledger::parse(&std::fs::read_to_string(&file).unwrap_or_default()).ledger;
    ledger.insert(harness::Line {
        path: entity.to_string(),
        digest: harness::digest(entity, &key, &bytes),
        key,
    });
    std::fs::write(file, ledger.canonical()).expect("the ledger is written");
}

/// The state and the changed inputs of `rule`'s judgement of `entity`.
fn judgement(compiled: &CompiledProject, entity: &str, rule: &str) -> (String, Vec<String>) {
    let judgement = compiled
        .judgements()
        .get(entity, rule)
        .unwrap_or_else(|| panic!("{rule} judges {entity}"));
    (judgement.state.clone(), judgement.changed.clone())
}

/// The digest of `entity` over its resolved authored values. K7 T5.
fn digest_of(compiled: &CompiledProject, entity: &str) -> String {
    let graph = compiled.graph();
    let node = graph.node(entity).expect("the entity is declared");
    let forms = compiled.environment().registries().field_forms();
    specforge_project::judged::entity_digest(graph, forms, node)
}

const PLAN: &str = "v1.0/03/03-02";
const W311: &str = "gsd/W311";

#[specforge_test(
    behavior = "declare_judged_rule",
    verify = "every input of the model's judged rules parses and resolves"
)]
fn every_gsd_judged_input_parses_and_resolves() {
    let compiled = compile(&gsd_judged_project(""));
    let diagnostics = compiled.diagnostics();
    assert!(found(&diagnostics, "W112").is_empty(), "{diagnostics:?}");
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a hand-written verdict is E083"
)]
fn a_hand_written_verdict_is_e083() {
    let compiled = compile(&gsd_judged_project(&hand_written()));
    let diagnostics = compiled.diagnostics();
    assert_eq!(found(&diagnostics, "E083").len(), 1, "{diagnostics:?}");
}

fn hand_written() -> String {
    format!(
        "plan 03-04 \"Hand\" {{\n  phase 03\n  verdict {W311} passed {{\n    judge \"j\"\n    model \"m\"\n    at \"2026-10-26T14:02:11Z\"\n    inputs {{ plan \"sha256:0\" }}\n  }}\n}}\n"
    )
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a hand-written verdict counts as missing"
)]
fn a_hand_written_verdict_counts_as_missing() {
    let compiled = compile(&gsd_judged_project(&hand_written()));
    let diagnostics = compiled.diagnostics();
    let on_hand: Vec<_> = found(&diagnostics, W311)
        .into_iter()
        .filter(|d| d.message.contains("v1.0/03/03-04"))
        .collect();
    assert_eq!(on_hand.len(), 1, "{diagnostics:?}");
    assert!(on_hand[0].message.contains("not recorded"), "{on_hand:?}");
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "an entity digest ignores layout, comments, verdicts and derived values"
)]
fn an_entity_digest_ignores_layout_comments_verdicts_and_derived_values() {
    let project = gsd_judged_project("");
    let before = digest_of(&compile(&project), PLAN);
    project.write(
        "a.spec",
        GSD_JUDGED_SPEC.replace("plan 03-02", "// a comment\nplan   03-02"),
    );
    assert_eq!(before, digest_of(&compile(&project), PLAN));
    project.write(
        "a.spec",
        GSD_JUDGED_SPEC.replace("\"Session tokens\"", "\"Other\""),
    );
    assert_ne!(before, digest_of(&compile(&project), PLAN));
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a respelling stales nothing and a rename stales the verdicts that read it"
)]
fn a_respelling_stales_nothing_and_a_rename_stales_its_readers() {
    let project = gsd_judged_project("");
    record(&project, PLAN, W311, "passed");
    // Spelling the phase with its full path is the same reference.
    project.write(
        "a.spec",
        project.read("a.spec").replace(
            "phase 03\n  requirements [AUTH-07]\n  files",
            "phase v1.0/03\n  requirements [AUTH-07]\n  files",
        ),
    );
    assert_eq!(judgement(&compile(&project), PLAN, W311).0, "fresh");
    // Renaming the requirement changes the full path every reader digests.
    project.write(
        "a.spec",
        project.read("a.spec").replace("AUTH-07", "AUTH-08"),
    );
    let (state, changed) = judgement(&compile(&project), PLAN, W311);
    assert_eq!(state, "stale");
    assert!(changed.contains(&"requirements".to_string()), "{changed:?}");
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a judged rule reports missing, stale and failed verdicts"
)]
fn a_judged_rule_reports_missing_stale_and_failed_verdicts() {
    let project = gsd_judged_project("");
    let missing = compile(&project).diagnostics();
    let on_plan = |d: &Diagnostic| d.message.contains(PLAN);
    assert!(
        found(&missing, W311)
            .iter()
            .any(|d| on_plan(d) && d.message.contains("no verdict")),
        "{missing:?}"
    );
    record(&project, PLAN, W311, "issues");
    let failed = compile(&project).diagnostics();
    assert!(
        found(&failed, W311)
            .iter()
            .any(|d| on_plan(d) && d.message.contains("is 'issues'")),
        "{failed:?}"
    );
    record(&project, PLAN, W311, "passed");
    assert!(
        found(&compile(&project).diagnostics(), W311)
            .iter()
            .all(|d| !on_plan(d))
    );
    project.write(
        "a.spec",
        project.read("a.spec").replace("Use tokens", "Use cookies"),
    );
    let stale = compile(&project).diagnostics();
    assert!(
        found(&stale, W311)
            .iter()
            .any(|d| on_plan(d) && d.message.contains("stale")),
        "{stale:?}"
    );
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a stale verdict names the inputs that changed"
)]
fn a_stale_verdict_names_the_inputs_that_changed() {
    let project = gsd_judged_project("");
    record(&project, PLAN, W311, "passed");
    project.write(
        "a.spec",
        project.read("a.spec").replace("Use tokens", "Use cookies"),
    );
    let (state, changed) = judgement(&compile(&project), PLAN, W311);
    assert_eq!(state, "stale");
    assert_eq!(changed, ["decisions"]);
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a file input reads from its field's base and an absent file digests as absent"
)]
fn a_file_input_reads_from_its_fields_base_and_absent_digests_absent() {
    let project = gsd_judged_project("");
    record(&project, "v1.0/03/03-02/TR1", "gsd/E501", "passed");
    let truth = "v1.0/03/03-02/TR1";
    assert_eq!(judgement(&compile(&project), truth, "gsd/E501").0, "fresh");
    // CRLF against LF stales nothing; another content does.
    project.write("src/auth/session.ts", "export const session = 1;\r\n");
    assert_eq!(judgement(&compile(&project), truth, "gsd/E501").0, "fresh");
    project.write("src/auth/session.ts", "export const session = 2;\n");
    let (state, changed) = judgement(&compile(&project), truth, "gsd/E501");
    assert_eq!(
        (state.as_str(), changed.as_slice()),
        ("stale", ["code".to_string()].as_slice())
    );
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a file input that is a spec source digests as what it declares"
)]
fn a_spec_source_file_input_ignores_its_verdicts() {
    let project = gsd_judged_project("");
    let truth = "v1.0/03/03-02/TR1";
    project.write(
        "a.spec",
        project
            .read("a.spec")
            .replace("\"src/auth/session.ts\"", "\"a.spec\""),
    );
    record(&project, truth, "gsd/E501", "passed");
    // Recording a verdict into the source changes its bytes but not what it declares.
    record(&project, PLAN, W311, "passed");
    assert_eq!(judgement(&compile(&project), truth, "gsd/E501").0, "fresh");
    project.write(
        "a.spec",
        project
            .read("a.spec")
            .replace("Tokens survive", "Tokens do not survive"),
    );
    assert_eq!(judgement(&compile(&project), truth, "gsd/E501").0, "stale");
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "test inputs are not judged without a report"
)]
fn test_inputs_are_not_judged_without_a_report() {
    let project = gsd_judged_project("");
    let truth = "v1.0/03/03-02/TR1";
    record(&project, truth, "gsd/E501", "passed");
    let diagnostics = compile(&project).diagnostics();
    assert_eq!(found(&diagnostics, "I205").len(), 1, "{diagnostics:?}");
    assert_eq!(judgement(&compile(&project), truth, "gsd/E501").0, "fresh");
    project.write("specforge-report.json", "not json");
    let diagnostics = compile(&project).diagnostics();
    assert!(!found(&diagnostics, "E045").is_empty(), "{diagnostics:?}");
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a re-run that agrees on every status stales nothing"
)]
fn a_rerun_that_agrees_stales_nothing() {
    let project = gsd_judged_project("");
    let truth = "v1.0/03/03-02/TR1";
    let report = |ms: u64, status: &str| {
        json!({"results": {"v1.0/03/03-02/TR1": {"tests": [{"name": "survives", "status": status, "duration_ms": ms}]}}}).to_string()
    };
    project.write("specforge-report.json", report(5, "passed"));
    record(&project, truth, "gsd/E501", "passed");
    project.write("specforge-report.json", report(900, "passed"));
    assert_eq!(judgement(&compile(&project), truth, "gsd/E501").0, "fresh");
    project.write("specforge-report.json", report(5, "failed"));
    let (state, changed) = judgement(&compile(&project), truth, "gsd/E501");
    assert_eq!(state, "stale");
    assert_eq!(changed, ["tests"]);
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a gated judged rule computes and reports nothing"
)]
fn a_gated_judged_rule_computes_and_reports_nothing() {
    let project = gsd_judged_project("");
    let open = compile(&project).diagnostics();
    assert!(!found(&open, W311).is_empty(), "{open:?}");
    project.write(
        "specforge.json",
        json!({"name": "p", "extensions": [GSD_JUDGED], "settings": {GSD_JUDGED: {"plan_check": false}}}).to_string(),
    );
    let gated = compile(&project).diagnostics();
    assert!(found(&gated, W311).is_empty(), "{gated:?}");
    // Nothing is read for a gated rule: `CLAUDE.md` is no input of any other rule.
    let session = project.session_in(Arc::new(gsd_judged_runtime()) as SharedRuntime);
    let watched = session.inputs().watched();
    assert!(
        !watched.contains(&Watched::File(project.root().join("CLAUDE.md"))),
        "{watched:?}"
    );
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a verdict that cannot be read is W168"
)]
fn an_unknown_rules_verdict_is_w168() {
    let spec = "plan 03-05 \"Odd\" {\n  phase 03\n  verdict gsd/W999 passed {\n    judge \"j\"\n    model \"m\"\n    at \"t\"\n    inputs { plan \"sha256:0\" }\n  }\n}\n";
    let diagnostics = compile(&gsd_judged_project(spec)).diagnostics();
    assert_eq!(found(&diagnostics, "W168").len(), 1, "{diagnostics:?}");
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a changed file input rechecks the session"
)]
fn a_changed_file_input_rechecks_the_session() {
    let project = gsd_judged_project("");
    let truth = "v1.0/03/03-02/TR1";
    record(&project, truth, "gsd/E501", "passed");
    let mut session = project.session_in(Arc::new(gsd_judged_runtime()) as SharedRuntime);
    project.write("src/auth/session.ts", "export const session = 2;\n");
    let update = session
        .apply(&session.changes([project.root().join("src/auth/session.ts").as_path()]))
        .expect("a check input changed");
    assert_eq!(update.kind, UpdateKind::Checks);
    let diagnostics = session.project().diagnostics();
    assert!(
        found(&diagnostics, "gsd/E501")
            .iter()
            .any(|d| d.message.contains("stale")),
        "{diagnostics:?}"
    );
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "incremental digests equal cold digests"
)]
fn incremental_digests_equal_cold_digests() {
    use proptest::prelude::*;
    // A random sequence of edits to the fixture: after each, a verifying session's kept digests equal a cold
    // evaluation's (`Update.verification`), and so do the judgements of a fresh compile.
    let edit = (0usize..6, 0usize..3);
    let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig::with_cases(24));
    runner
        .run(&proptest::collection::vec(edit, 1..6), |edits| {
            let project = gsd_judged_project("");
            record(&project, PLAN, W311, "passed");
            let mut session = project.session_in(Arc::new(gsd_judged_runtime()) as SharedRuntime);
            session.set_verify_incremental(true);
            for (which, round) in edits {
                let text = project.read("a.spec");
                let next = match which {
                    0 => text.replace("Use tokens", &format!("Use tokens {round}")),
                    1 => text.replace("\"Context\"", &format!("\"Context {round}\"")),
                    2 => text.replace("AUTH-07", &format!("AUTH-0{}", 7 + round)),
                    3 => text.replace(
                        "plan 03-03 \"Another\"",
                        &format!("plan 03-03 \"Another {round}\""),
                    ),
                    4 => text.replace("Tokens survive", &format!("Tokens survive {round}")),
                    _ => {
                        project.write(
                            "src/auth/session.ts",
                            format!("export const session = {round};\n"),
                        );
                        text
                    }
                };
                project.write("a.spec", next);
                let paths = [
                    project.root().join("a.spec"),
                    project.root().join("src/auth/session.ts"),
                ];
                let update = session.apply(&session.changes(paths.iter().map(|p| p.as_path())));
                if let Some(update) = update {
                    prop_assert!(
                        update.verification.as_ref().is_none_or(Result::is_ok),
                        "{:?}",
                        update.verification
                    );
                }
                let cold = compile(&project);
                for entity in [PLAN, "v1.0/03/03-03"] {
                    prop_assert_eq!(
                        session
                            .project()
                            .judgements()
                            .get(entity, W311)
                            .map(|j| (&j.state, &j.changed)),
                        cold.judgements()
                            .get(entity, W311)
                            .map(|j| (&j.state, &j.changed))
                    );
                }
            }
            Ok(())
        })
        .expect("the incremental digests agree with a cold evaluation");
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a memo hit still watches its file"
)]
fn a_memo_hit_still_watches_its_file() {
    let project = gsd_judged_project("");
    record(&project, PLAN, W311, "passed");
    let session = project.session_in(Arc::new(gsd_judged_runtime()) as SharedRuntime);
    let watched = session.inputs().watched();
    assert!(
        watched.contains(&Watched::File(project.root().join("CLAUDE.md"))),
        "{watched:?}"
    );
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a file change the session has not applied misses the memo"
)]
fn a_change_not_yet_applied_misses_the_memo() {
    let project = gsd_judged_project("");
    let truth = "v1.0/03/03-02/TR1";
    record(&project, truth, "gsd/E501", "passed");
    let mut session = project.session_in(Arc::new(gsd_judged_runtime()) as SharedRuntime);
    project.write("src/auth/session.ts", "export const session = 3;\n");
    // A .spec edit runs the checks before the watcher reports the source file.
    project.write(
        "a.spec",
        project
            .read("a.spec")
            .replace("Session tokens", "Session cookies"),
    );
    let _ = session.apply(&session.changes([project.root().join("a.spec").as_path()]));
    let diagnostics = session.project().diagnostics();
    assert!(
        found(&diagnostics, "gsd/E501")
            .iter()
            .any(|d| d.message.contains("stale")),
        "{diagnostics:?}"
    );
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a compute pass reads judgements and a V field is its fresh verdict's value"
)]
fn a_compute_pass_reads_judgements_and_a_v_field_is_its_fresh_value() {
    let project = gsd_judged_project("");
    record(&project, "PA", "gsd/W205", "suspicious");
    let compiled = compile(&project);
    let (record, _) = compiled.entities().get("PA").unwrap();
    let legitimacy = record
        .fields
        .iter()
        .find(|f| f.key == "legitimacy")
        .expect("a V field");
    assert_eq!(legitimacy.text, "suspicious");
    // A pass is handed the judgements of each entity.
    let input = specforge_project::passes::pass_input(&specforge_project::passes::PassContext {
        judgements: Some(compiled.judgements()),
        ..specforge_project::passes::PassContext::of(compiled.entities())
    });
    let audit = input.entities.iter().find(|e| e.id == "PA").unwrap();
    assert!(
        audit.judgements.iter().any(|j| j.rule == "gsd/W205"
            && j.state == "fresh"
            && j.outcome.as_deref() == Some("suspicious")),
        "{:?}",
        audit.judgements
    );
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a V field written by hand is E082"
)]
fn a_v_field_written_by_hand_is_e082() {
    let spec = "package_audit PB \"B\" {\n  legitimacy suspicious\n}\n";
    let diagnostics = compile(&gsd_judged_project(spec)).diagnostics();
    let e082 = found(&diagnostics, "E082");
    assert_eq!(e082.len(), 1, "{diagnostics:?}");
    assert!(e082[0].message.contains("gsd/W205"), "{e082:?}");
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "re-verification reads the baseline's judgements"
)]
fn re_verification_reads_the_baselines_judgements() {
    use specforge_project::baseline::Baseline;
    use specforge_project::history::{History, Located, Sha};
    use specforge_project::testing::FakeHistory;
    use std::path::{Path, PathBuf};
    // A truth passing at HEAD (fresh over that commit's files) and failing now is a regression; one stale at HEAD is
    // not "passing there".
    let project = gsd_judged_project("");
    record(&project, PLAN, W311, "passed");
    let compiled = compile(&project);
    let id = Sha::new("a".repeat(40));
    // The baseline at a commit holding the spec, the ledger and `claude` as the instructions file.
    let at = |claude: &str| {
        let mut fake = FakeHistory::repo(project.root(), id.as_str(), Vec::new());
        fake.blobs = vec![
            (PathBuf::from("CLAUDE.md"), claude.as_bytes().to_vec()),
            (
                PathBuf::from("src/auth/session.ts"),
                b"export const session = 1;\n".to_vec(),
            ),
        ];
        let Located::Repo(place) = History::locate(&fake, project.root()) else {
            panic!("a repository")
        };
        let fetch = |keys: &[String]| {
            let asked: Vec<PathBuf> = keys.iter().map(PathBuf::from).collect();
            let found = History::blobs(&fake, &place, &id, &asked).unwrap();
            keys.iter()
                .cloned()
                .zip(found.into_iter().map(|(_, bytes)| bytes))
                .collect::<Vec<_>>()
        };
        let files = vec![
            (PathBuf::from("a.spec"), project.read("a.spec").into_bytes()),
            (
                PathBuf::from("specforge-harness"),
                project.read("specforge-harness").into_bytes(),
            ),
        ];
        Baseline::compile(
            compiled.environment(),
            id.clone(),
            Path::new(""),
            files,
            &fetch,
        )
    };
    // The files as they were: the verdict holds at that commit.
    let fresh = at("# Instructions\n");
    let judged = &fresh.entities[PLAN].judgements;
    assert!(
        judged
            .iter()
            .any(|j| j.rule == W311 && j.state == "fresh" && j.passed),
        "{judged:?}"
    );
    let view = fresh.pass_view(&["plan".to_string()]);
    assert_eq!(view.entities[PLAN].judgements, *judged);
    // An input that differed at that commit: stale there, so not passing there.
    let changed = at("# Other instructions\n");
    let stale = changed.entities[PLAN]
        .judgements
        .iter()
        .find(|j| j.rule == W311);
    assert!(
        stale.is_some_and(|j| j.state == "stale" && !j.passed),
        "{stale:?}"
    );
}
