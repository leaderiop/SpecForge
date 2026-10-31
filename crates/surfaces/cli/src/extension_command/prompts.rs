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
    // After the host's `--` every word is the prompt's.
    let (code, out, err) = prompt(
        &ok,
        &["t.ingest", "--", "--path", "docs/adr", "--format", "md"],
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"path\":\"docs/adr\""), "{out}");
    assert!(out.contains("\"format\":\"md\""), "{out}");

    let words = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
    let (code, out, err) = extension_here(&ok, "t", words(&["ingest", "docs/adr"]));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"path\":\"docs/adr\""), "{out}");
    // `--format md` after the `--` is the prompt's: the output stays human, not JSON.
    let (code, out, err) = extension_here(&ok, "t", words(&["ingest", "--", "--format", "md"]));
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("prompt__ingest "), "{out}");
    // Before it, `--format` is the host's.
    let (code, _, err) = extension_here(&ok, "t", words(&["ingest", "--format", "md"]));
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("unknown format 'md'"), "{err}");
}

/// `specforge prompt <line…>` as clap reads it: the words the renderer is given, and the project it names.
fn fallback(line: &[&str]) -> (Vec<String>, String) {
    use clap::Parser;
    #[derive(clap::Parser)]
    struct Line {
        #[command(flatten)]
        args: crate::prompt::Args,
    }
    let parsed = Line::try_parse_from(std::iter::once("prompt").chain(line.iter().copied()))
        .unwrap_or_else(|e| panic!("{line:?}: {e}"));
    let words = parsed.args.words();
    let path = parsed.args.project_of(&words).path().display().to_string();
    (words, path)
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
        err.contains("no such option") || err.contains("no positional argument takes this word"),
        "{err}"
    );
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "the fallback line an install writes leaves the host's options in the text to the prompt"
)]
fn the_fallback_line_leaves_host_options_in_the_text_to_the_prompt() {
    let ok = project("fine", "ok");
    // The line a skill writes: `--host`, then `--`, then the user's text. Clap drops a leading `--`; it is put back.
    let (words, path) = fallback(&[
        "t.ingest",
        "--host",
        "claude-code",
        "--",
        "docs",
        "--",
        "--format",
        "json",
    ]);
    assert_eq!(path, ".");
    let (code, out, err) = prompt(&ok, &named("t.ingest", &words));
    assert_eq!(code, 0, "{err}");
    // Human output, the text a tail: the second `--` is the prompt's boundary.
    assert!(out.starts_with("prompt__ingest "), "{out}");
    assert!(out.contains("\"path\":\"docs --format json\""), "{out}");

    // A prompt flag before the second `--` still works.
    let (words, _) = fallback(&["t.plan_phase", "--host", "codex", "--", "--research", "03"]);
    let (code, out, err) = prompt(&ok, &named("t.plan_phase", &words));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"research\":true"), "{out}");

    // Without the second `--` a host option in the text is the prompt's to refuse: no JSON, no other project.
    let (words, path) = fallback(&[
        "t.plan_phase",
        "--host",
        "codex",
        "--",
        "03",
        "--format",
        "json",
    ]);
    assert_eq!(path, ".");
    let (code, out, err) = prompt(&ok, &named("t.plan_phase", &words));
    assert_eq!(code, 2, "{err}");
    assert!(
        out.is_empty() && err.contains("no such option"),
        "{out}{err}"
    );
}

/// `name` and the `words` clap handed the renderer, as [`prompt`] takes them.
fn named<'a>(name: &'a str, words: &'a [String]) -> Vec<&'a str> {
    std::iter::once(name)
        .chain(words.iter().map(String::as_str))
        .collect()
}

#[specforge_test(
    behavior = "serve_extension_prompt",
    verify = "a path in the prompt's text never moves the project, and a leading path names it"
)]
fn a_path_in_the_text_never_moves_the_project() {
    let (_, path) = fallback(&[
        "t.plan_phase",
        "--host",
        "claude-code",
        "--",
        "03",
        "--path",
        "/etc",
    ]);
    assert_eq!(path, ".");
    // Before the `--` it is the host's, wherever it stands.
    let (_, path) = fallback(&["t.plan_phase", "03", "--path", "/other", "--", "--research"]);
    assert_eq!(path, "/other");
    // A leading `--path` names the project and is not handed to the prompt (REVIEW-k20b m1).
    let (words, path) = fallback(&["--path", "/other", "t.ingest"]);
    assert_eq!(path, "/other");
    assert!(!words.iter().any(|w| w == "--path"), "{words:?}");
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "the host option reaches a command on the command line, and a host that is none is refused"
)]
fn the_host_option_reaches_a_command() {
    let p = project("fine", "fine");
    let (code, out, err) = extension(&p, "t", &["record-plan", "03", "--host", "codex"]);
    assert_eq!((code, out.contains("host codex")), (0, true), "{out}{err}");
    let (code, out, _) = extension(&p, "t", &["record-plan", "03"]);
    assert_eq!((code, out.contains("host ")), (0, false), "{out}");
    let (code, _, err) = extension(&p, "t", &["record-plan", "03", "--host", "vim"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("vim"), "{err}");
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "a command that declares a host argument keeps it as its own and is not told the calling host"
)]
fn a_command_declaring_host_keeps_its_own_argument() {
    let p = project("fine", "fine");
    let (code, out, err) = extension(&p, "t", &["name-host", "--host", "codex"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"host\":\"codex\""), "its own arg: {out}");
    assert!(!out.contains("\nhost "), "not told the calling host: {out}");
}
