//! An extension's prompts and gated commands on the command line (ADR 0291): `specforge prompt` renders a prompt,
//! `specforge <short> <name>` falls back to a prompt when no command has the name, and a command whose gate fails
//! is refused before its export runs. The fixture is `@t/core`, served in process.

use super::run_in;
use crate::terminal::{Facts, Terminal};
use serde_json::Value;
use specforge_project::SharedRuntime;
use specforge_project::testing::{CORE, GsdPrompts, NO_CONTEXT, TestProject, gsd_prompts_sources};
use specforge_test_macros::test as specforge_test;
use std::sync::Arc;

fn project(goal: &str, state: &str) -> TestProject {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[CORE])
        .installing_enabled();
    for (path, text) in gsd_prompts_sources(goal, state) {
        project.write(path, text);
    }
    project
}

/// `specforge <ext> <rest…>` over `project`: its exit code, stdout and stderr.
fn extension(project: &TestProject, ext: &str, rest: &[&str]) -> (i32, String, String) {
    let runtime: SharedRuntime = Arc::new(GsdPrompts::new().runtime());
    let env = project.environment_in(runtime);
    let (terminal, capture) = Terminal::captured("", Facts::PIPED);
    let mut rest: Vec<String> = rest.iter().map(ToString::to_string).collect();
    rest.extend(["--path".to_string(), project.root().display().to_string()]);
    let code = run_in(env, &project.project_root(), ext, rest, &[], &terminal);
    (code, capture.stdout(), capture.stderr())
}

/// `specforge prompt <rest…>`. The command does not exist yet; the ticket that adds it calls it here.
fn prompt(_project: &TestProject, _rest: &[&str]) -> (i32, String, String) {
    (
        2,
        String::new(),
        "error: unrecognized subcommand 'prompt'".into(),
    )
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "specforge prompt prints a rendered prompt, as JSON with its host message, and refuses a failing gate"
)]
#[ignore = "K8 T8"]
fn specforge_prompt_prints_a_rendered_prompt() {
    let ok = project("fine", "ok");
    let (code, out, err) = prompt(
        &ok,
        &["t.plan_phase", "03", "--research", "--host", "codex"],
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("prompt__plan_phase "), "{out}");
    assert!(out.contains("\"host\":\"codex\""), "{out}");

    let (code, out, err) = prompt(&ok, &["t.plan_phase", "03", "--format", "json"]);
    assert_eq!(code, 0, "{err}");
    let rendered: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(rendered["payload"]["prompt"], "@t/core@0.4.0:plan_phase");
    assert_eq!(rendered["messages"][0]["role"], "user");

    let empty = project(NO_CONTEXT, "ok");
    let (code, _, err) = prompt(&empty, &["t.plan_phase", "03"]);
    assert_eq!(code, 1);
    assert!(
        err.contains("gate_failed") && err.contains("t/E901"),
        "{err}"
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "an extension's name routes a prompt when no command of it has that name"
)]
#[ignore = "K8 T8"]
fn an_extension_name_routes_a_prompt_when_no_command_has_it() {
    let ok = project("fine", "ok");
    // `research_phase` is a prompt only; `record_plan` is a command only.
    let (code, out, err) = extension(&ok, "t", &["research_phase", "03"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("prompt__research_phase "), "{out}");
    let (code, out, _) = extension(&ok, "t", &["record-plan", "--phase", "03"]);
    assert_eq!(code, 0);
    assert!(
        !out.starts_with("prompt__"),
        "a command still runs as a command: {out}"
    );
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a gated command is refused on the command line with its failing diagnostics, and its export is not called"
)]
fn a_gated_command_is_refused_on_the_cli() {
    let blocked = project("fine", "blocked");
    let (code, out, err) = extension(&blocked, "t", &["execute-phase", "03"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        err.contains("error[gate_failed]") && err.contains("t/E902"),
        "{err}"
    );
    assert!(out.is_empty(), "the export never printed: {out}");

    let (code, err_json) = {
        let (code, _, err) = extension(&blocked, "t", &["execute-phase", "03", "--format", "json"]);
        (code, err)
    };
    assert_eq!(code, 1);
    assert!(err_json.contains("\"code\":\"gate_failed\""), "{err_json}");
}
