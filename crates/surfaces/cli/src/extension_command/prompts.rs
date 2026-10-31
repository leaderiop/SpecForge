//! An extension's prompts and gated commands on the command line (ADR 0291): `specforge prompt` renders a prompt,
//! `specforge <short> <name>` falls back to a prompt when no command has the name, and a command whose gate fails
//! is refused before its export runs. The fixture is `@t/core`, served in process.

use super::run_in;
use crate::outcome::OutputFormat;
use crate::terminal::{Facts, Terminal};
use serde_json::Value;
use specforge_project::testing::{CORE, GsdPrompts, NO_CONTEXT, TestProject, gsd_prompts_sources};
use specforge_project::{CompiledProject, SharedRuntime};
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

/// The fixture, with the prompt `ingest`: its own `path` (the tail) and `format` arguments.
fn fixture() -> GsdPrompts {
    GsdPrompts::new().prompting(serde_json::json!({
        "id": "ingest", "title": "Ingest", "description": "Ingest documents", "export": "prompt__ingest",
        "args": [
            {"name": "path", "field_type": "string", "rest": true},
            {"name": "format", "field_type": "enum", "enum_values": ["md", "txt"]},
        ],
    }))
}

/// `specforge <ext> <rest…>` over `project`: its exit code, stdout and stderr.
fn extension(project: &TestProject, ext: &str, rest: &[&str]) -> (i32, String, String) {
    let mut rest: Vec<String> = rest.iter().map(ToString::to_string).collect();
    rest.extend(["--path".to_string(), project.root().display().to_string()]);
    extension_here(project, ext, rest)
}

/// [`extension`] with the project as the current directory: the command line names no `--path` of its own.
fn extension_here(project: &TestProject, ext: &str, rest: Vec<String>) -> (i32, String, String) {
    let runtime: SharedRuntime = Arc::new(fixture().runtime());
    let env = project.environment_in(runtime);
    let (terminal, capture) = Terminal::captured("", Facts::PIPED);
    let code = run_in(env, &project.project_root(), ext, rest, &[], &terminal);
    (code, capture.stdout(), capture.stderr())
}

/// `specforge prompt <rest…>` over `project`: its exit code, stdout and stderr.
fn prompt(project: &TestProject, rest: &[&str]) -> (i32, String, String) {
    let runtime: SharedRuntime = Arc::new(fixture().runtime());
    let compiled = CompiledProject::of(project.environment_in(runtime));
    let (terminal, capture) = Terminal::captured("", Facts::PIPED);
    let mut words: Vec<String> = rest.iter().map(ToString::to_string).collect();
    let name = words.remove(0);
    let code = crate::prompt::run_in(&compiled, &name, words, OutputFormat::Human, &terminal);
    (code.code(), capture.stdout(), capture.stderr())
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "specforge prompt prints a rendered prompt, as JSON with its host message, and refuses a failing gate"
)]
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
fn an_extension_name_routes_a_prompt_when_no_command_has_it() {
    let ok = project("fine", "ok");
    // `research_phase` is a prompt only; `record_plan` is a command only.
    let (code, out, err) = extension(&ok, "t", &["research-phase", "03"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("prompt__research_phase "), "{out}");
    let (code, out, _) = extension(&ok, "t", &["record-plan", "03"]);
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

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt renders the same messages and payload on the command line and over MCP"
)]
fn a_prompt_renders_alike_on_the_command_line_and_over_mcp() {
    use serde_json::json;
    use specforge_mcp::testing::{Served, rpc};
    let (cli_project, mcp_project) = (project("fine", "blocked"), project("fine", "blocked"));
    let (code, out, err) = prompt(
        &cli_project,
        &[
            "t.plan_phase",
            "03",
            "--depth",
            "2",
            "--host",
            "codex",
            "--format",
            "json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    let cli: Value = serde_json::from_str(&out).unwrap();

    let runtime: SharedRuntime = Arc::new(GsdPrompts::new().runtime());
    let mut served = Served::initialized_in(mcp_project, runtime);
    let reply = rpc::tool(
        &mut served,
        "specforge.prompt",
        json!({"name": "specforge.t.plan_phase", "arguments": {"phase": "03", "depth": 2}, "host": "codex"}),
    );
    assert_eq!(cli["messages"], reply["messages"]);
    assert_eq!(cli["payload"], reply["payload"]);
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "completions and help list an extension's prompts beside its commands, with their arguments"
)]
fn the_extension_command_line_lists_its_prompts() {
    let runtime: SharedRuntime = Arc::new(GsdPrompts::new().runtime());
    let env = project("fine", "ok").environment_in(runtime);
    let cli = super::with_commands_of(clap::Command::new("specforge"), &env);
    let ext = cli.find_subcommand("t").expect("the extension is listed");
    let plan = ext
        .find_subcommand("plan-phase")
        .expect("its prompt is a subcommand");
    let options: Vec<&str> = plan.get_arguments().filter_map(|a| a.get_long()).collect();
    for expected in [
        "research", "depth", "mode", "labels", "host", "path", "format",
    ] {
        assert!(options.contains(&expected), "{expected} in {options:?}");
    }
    assert!(
        plan.get_arguments()
            .any(|a| a.get_id() == "phase" && a.is_required_set())
    );
    assert!(
        ext.find_subcommand("record-plan").is_some(),
        "and its commands"
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "specforge prompt refuses an unknown prompt, an unknown host or format and an option without its value, rendering nothing"
)]
fn specforge_prompt_refuses_what_it_cannot_render() {
    let ok = project("fine", "ok");
    for (words, said) in [
        (&["t.nope"][..], "no prompt 't.nope'"),
        (
            &["t.plan_phase", "03", "--host", "vim"][..],
            "unknown host 'vim'",
        ),
        (
            &["t.plan_phase", "03", "--host"][..],
            "--host takes a value",
        ),
        (
            &["t.plan_phase", "03", "--format", "yaml"][..],
            "unknown format 'yaml'",
        ),
        (&["t.plan_phase", "03", "--depth", "9"][..], "depth"),
    ] {
        let (code, out, err) = prompt(&ok, words);
        assert_ne!(code, 0, "{words:?}");
        assert!(
            out.is_empty() && err.contains(said),
            "{words:?}: {out} {err}"
        );
    }
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a prompt's own path and format arguments reach it on the command line"
)]
fn a_prompts_own_path_and_format_reach_it() {
    let ok = project("fine", "ok");
    let (code, out, err) = prompt(&ok, &["t.ingest", "--path", "docs/adr", "--format", "md"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"path\":\"docs/adr\""), "{out}");
    assert!(out.contains("\"format\":\"md\""), "{out}");

    let words = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
    let (code, out, err) = extension_here(&ok, "t", words(&["ingest", "docs/adr"]));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"path\":\"docs/adr\""), "{out}");
    // `--format md` is the prompt's: the output stays human, not JSON.
    let (code, out, err) = extension_here(&ok, "t", words(&["ingest", "--format", "md"]));
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("prompt__ingest "), "{out}");
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "the host reads its options before the first double dash only"
)]
fn the_host_reads_its_options_before_double_dash_only() {
    let ok = project("fine", "ok");
    let (code, _, err) = prompt(&ok, &["t.plan_phase", "03", "--", "--host", "codex"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.contains("no positional argument takes this word"),
        "{err}"
    );
}
