//! A writing command on the command line (ADR 0289): its plan is previewed with `--dry-run` and written otherwise,
//! and a plan the host refuses fails the command. The fixture is `@t/core`'s `record`, served in process, answering
//! the plan the test spells.

use super::run_in;
use crate::terminal::{Facts, Terminal};
use serde_json::{Value, json};
use specforge_package::Sha256;
use specforge_project::SharedRuntime;
use specforge_project::testing::{
    CORE, GSD_WRITES_EXTENSIONS, GsdWrites, TestProject, gsd_writes_answer,
};
use specforge_test_macros::test as specforge_test;
use std::sync::Arc;

fn project() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file(
            "m.spec",
            "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n",
        )
        .file("03-01.spec", "uat_test U1 \"One\" {\n  phase 03\n}\n")
}

/// `specforge core record <args>`, the command answering a plan that sets `result` of `U1` on `revision`: its exit
/// code, stdout and stderr.
fn record(project: &TestProject, revision: &str, args: &[&str]) -> (i32, String, String) {
    run_command(project, "record", revision, Facts::PIPED, args)
}

/// `specforge core <command> <args>` over a terminal of `facts`, the command answering that plan.
fn run_command(
    project: &TestProject,
    command: &str,
    revision: &str,
    facts: Facts,
    args: &[&str],
) -> (i32, String, String) {
    let ops = json!([{"op": "set", "entity": "v1.0/03/U1", "field": "result", "value": "pass",
                      "revision": revision}]);
    run_answering(project, command, gsd_writes_answer(0, ops), facts, args)
}

/// `specforge core <command> <args>`, the command answering `answer`.
fn run_answering(
    project: &TestProject,
    command: &str,
    answer: specforge_wasm::WasmCallResult,
    facts: Facts,
    args: &[&str],
) -> (i32, String, String) {
    let runtime = Arc::new(
        GsdWrites::new()
            .answering(CORE, &format!("cmd__{command}"), answer)
            .runtime(),
    );
    let env = project.environment_in(runtime as SharedRuntime);
    let (terminal, capture) = Terminal::captured("", facts);
    let mut rest = vec![command.to_string()];
    rest.extend(args.iter().map(ToString::to_string));
    rest.extend(["--path".to_string(), project.root().display().to_string()]);
    let code = run_in(env, &project.project_root(), "core", rest, &[], &terminal);
    (code, capture.stdout(), capture.stderr())
}

fn revision(project: &TestProject) -> String {
    format!(
        "sha256:{}",
        Sha256::of(project.read("03-01.spec").as_bytes())
    )
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a writing command previews its plan with --dry-run and writes it otherwise on the command line"
)]
fn a_writing_command_previews_and_applies_on_the_cli() {
    let p = project();
    let revision = revision(&p);
    let before = p.read("03-01.spec");
    let (code, out, _) = record(&p, &revision, &["--dry-run"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("03-01.spec") && out.contains("+  result \"pass\""),
        "{out}"
    );
    assert_eq!(p.read("03-01.spec"), before);
    let (code, out, _) = record(&p, &revision, &["--format", "json"]);
    assert_eq!(code, 0, "{out}");
    let document: Value = serde_json::from_str(&out).unwrap();
    assert!(
        document["plan"]["affected_files"].is_array() && document.get("output").is_some(),
        "{document}"
    );
    assert!(p.read("03-01.spec").contains("result \"pass\""));
    // The same plan again names a revision the file no longer has.
    let (code, _, err) = record(&p, &revision, &[]);
    assert_eq!(code, 1);
    assert!(err.contains("[stale_revision]"), "{err}");
    let (_, _, err) = record(&p, &revision, &["--format", "json"]);
    assert!(err.contains("\"code\":\"stale_revision\""), "{err}");
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a writing command's plan and the file it writes agree on the command line and over MCP"
)]
fn a_writing_commands_plan_agrees_on_both_surfaces() {
    use specforge_mcp::testing::{Served, rpc};
    let (cli_project, mcp_project) = (project(), project());
    let revision = revision(&cli_project);
    let (code, out, _) = record(&cli_project, &revision, &["--format", "json"]);
    assert_eq!(code, 0, "{out}");
    let cli: Value = serde_json::from_str(&out).unwrap();

    let ops = json!([{"op": "set", "entity": "v1.0/03/U1", "field": "result", "value": "pass",
                      "revision": revision}]);
    let runtime: SharedRuntime = Arc::new(
        GsdWrites::new()
            .answering(CORE, "cmd__record", gsd_writes_answer(0, ops))
            .runtime(),
    );
    let mut served = Served::initialized_in(mcp_project, runtime);
    let reply = rpc::call_tool(&mut served, "specforge.core.record", json!({}));
    let mcp: Value = serde_json::from_str(&rpc::tool_text(&reply)).unwrap();

    assert_eq!(cli["plan"], mcp["plan"], "the same plan report");
    assert_eq!(
        cli_project.read("03-01.spec"),
        std::fs::read_to_string(served.root().join("03-01.spec")).unwrap(),
        "the same file"
    );
}

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "a human-attested command runs from a terminal and is refused to an agent's call"
)]
fn a_human_command_runs_only_from_a_terminal() {
    let p = project();
    let revision = revision(&p);
    let before = p.read("03-01.spec");
    let (code, out, err) = run_command(&p, "attest", &revision, Facts::PIPED, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("[human_only]"), "{err}");
    assert_eq!(p.read("03-01.spec"), before);
    let person = Facts {
        interactive: true,
        ..Facts::PIPED
    };
    let (code, out, err) = run_command(&p, "attest", &revision, person, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(p.read("03-01.spec").contains("result \"pass\""));
}

/// `record` answering `stdout` and, when `ops` is not empty, a plan.
fn picked_record(
    project: &TestProject,
    stdout: &str,
    ops: Value,
    args: &[&str],
) -> (i32, String, String) {
    let mut answer = json!({"exit_code": 0, "stdout": stdout, "stderr": ""});
    if ops.as_array().is_some_and(|ops| !ops.is_empty()) {
        answer["plan"] = json!({"ops": ops});
    }
    let answer = specforge_wasm::WasmCallResult::Ok(answer.to_string().into_bytes());
    run_answering(project, "record", answer, Facts::PIPED, args)
}

#[test]
fn pick_reads_the_commands_own_answer_whether_or_not_it_answers_a_plan() {
    let p = project();
    let ops = json!([{"op": "set", "entity": "v1.0/03/U1", "field": "result", "value": "pass",
                      "revision": revision(&p)}]);
    // a run that answers no plan, then one that does: the same path reads the same field
    let (code, out, err) =
        picked_record(&p, r#"{"phase": "03"}"#, json!([]), &["--pick", ".phase"]);
    assert_eq!((code, out.as_str()), (0, "03\n"), "{err}");
    let (code, out, err) = picked_record(&p, r#"{"phase": "03"}"#, ops, &["--pick", ".phase"]);
    assert_eq!((code, out.as_str()), (0, "03\n"), "{err}");
    assert!(
        p.read("03-01.spec").contains("result \"pass\""),
        "the plan is applied"
    );
}

#[test]
fn pick_fails_a_writing_command_whose_answer_is_not_json() {
    let p = project();
    let ops = json!([{"op": "set", "entity": "v1.0/03/U1", "field": "result", "value": "pass",
                      "revision": revision(&p)}]);
    let (code, out, err) = picked_record(&p, "done", ops, &["--pick", ".phase"]);
    assert_eq!((code, out.as_str()), (1, ""), "{err}");
    assert!(err.contains("not JSON"), "{err}");
}
