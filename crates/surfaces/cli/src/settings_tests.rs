//! K16 (ADR 0296) at the command line: `specforge settings set` and `unset` parsed by clap and run in process over a
//! project whose extension declares settings (the tests' in-process runtime, as the MCP test has), with the terminal
//! captured. The ops tests pin the writes; these pin the line, the printed words, the JSON form and the exit codes.

use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use serde_json::{Value, json};
use specforge_project::SharedRuntime;
use specforge_project::testing::{SETTINGS_EXTENSION, settings_extension};
use specforge_wasm::testing::InProcessRuntime;

use crate::outcome::Exit;
use crate::settings::Args;
use crate::terminal::{Facts, Terminal};

#[derive(Parser)]
struct Line {
    #[command(flatten)]
    args: Args,
}

const CONFIG: &str = "{\n  \"name\": \"p\",\n  \"version\": \"0.1.0\",\n  \"extensions\": [\"@t/s\"],\n  \"settings\": {\n    \"@t/s\": {\n      \"profile\": \"strict\"\n    }\n  }\n}\n";

fn runtime() -> SharedRuntime {
    Arc::new(InProcessRuntime::new().with(settings_extension))
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("specforge.json"), CONFIG).unwrap();
    specforge_installed::testing::install(dir.path(), &*runtime(), &[SETTINGS_EXTENSION]);
    std::fs::write(dir.path().join("a.spec"), "").unwrap();
    dir
}

fn config(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("specforge.json")).unwrap()
}

struct Ran {
    exit: Exit,
    stdout: String,
    stderr: String,
}

fn run(dir: &Path, words: &[&str]) -> Ran {
    let mut line = vec!["specforge"];
    line.extend(words);
    line.extend(["--path", dir.to_str().unwrap()]);
    let parsed = Line::try_parse_from(line).unwrap_or_else(|e| panic!("{words:?}: {e}"));
    let (terminal, capture) = Terminal::captured("", Facts::PIPED);
    let exit = parsed.args.run_in(&terminal, Some(runtime()));
    Ran {
        exit,
        stdout: capture.stdout(),
        stderr: capture.stderr(),
    }
}

fn parse_error(words: &[&str]) -> String {
    let mut line = vec!["specforge"];
    line.extend(words);
    match Line::try_parse_from(line) {
        Ok(_) => panic!("{words:?} parsed"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn set_takes_the_value_as_positional_words() {
    let dir = project();

    let ran = run(dir.path(), &["set", "@t/s", "level", "2"]);

    assert_eq!(ran.exit, Exit::Passed, "{}", ran.stderr);
    assert_eq!(ran.stdout, "Wrote @t/s level = 2\n");
    let written: Value = serde_json::from_str(&config(dir.path())).unwrap();
    assert_eq!(written["settings"]["@t/s"]["level"], 2);
    assert_eq!(written["settings"]["@t/s"]["profile"], "strict");
}

#[test]
fn set_takes_a_json_value_and_the_two_forms_conflict() {
    let dir = project();

    let ran = run(
        dir.path(),
        &[
            "set",
            "@t/s",
            "sections",
            "--value-json",
            r#"[{"heading": "Goals"}]"#,
        ],
    );

    assert_eq!(ran.exit, Exit::Passed, "{}", ran.stderr);
    let written: Value = serde_json::from_str(&config(dir.path())).unwrap();
    assert_eq!(
        written["settings"]["@t/s"]["sections"],
        json!([{"heading": "Goals"}])
    );
    let conflict = parse_error(&["set", "@t/s", "level", "2", "--value-json", "2"]);
    assert!(conflict.contains("cannot be used with"), "{conflict}");
}

#[test]
fn malformed_json_is_refused_before_anything_is_written() {
    let dir = project();

    let ran = run(
        dir.path(),
        &["set", "@t/s", "sections", "--value-json", "[{"],
    );

    assert_eq!(ran.exit, Exit::Failed);
    assert!(ran.stderr.contains("invalid_input"), "{}", ran.stderr);
    assert!(
        ran.stderr.contains("--value-json is not JSON"),
        "{}",
        ran.stderr
    );
    assert_eq!(config(dir.path()), CONFIG);
}

#[test]
fn a_negative_number_is_a_value_without_the_double_dash() {
    // It parses, and the setting's own range then refuses it (E086), not clap.
    let dir = project();

    let ran = run(dir.path(), &["set", "@t/s", "level", "-1"]);

    assert_eq!(ran.exit, Exit::Failed);
    assert!(ran.stderr.contains("E086"), "{}", ran.stderr);
}

#[test]
fn dry_run_says_what_it_would_write_and_writes_nothing() {
    let dir = project();

    let ran = run(dir.path(), &["set", "@t/s", "level", "2", "--dry-run"]);

    assert_eq!(ran.exit, Exit::Passed, "{}", ran.stderr);
    assert_eq!(ran.stdout, "Would write @t/s level = 2\n");
    assert_eq!(config(dir.path()), CONFIG);
}

#[test]
fn unset_says_it_is_back_to_its_default_and_is_idempotent() {
    let dir = project();

    let ran = run(dir.path(), &["unset", "@t/s", "profile"]);

    assert_eq!(ran.exit, Exit::Passed, "{}", ran.stderr);
    assert_eq!(
        ran.stdout,
        "Removed @t/s profile: back to its preset or default\n"
    );
    let again = run(dir.path(), &["unset", "@t/s", "profile"]);
    assert_eq!(again.exit, Exit::Passed);
    assert_eq!(again.stdout, "@t/s profile: unchanged\n");
    let preview = run(dir.path(), &["set", "@t/s", "level", "3"]);
    assert_eq!(preview.exit, Exit::Passed);
    let would = run(dir.path(), &["unset", "@t/s", "level", "--dry-run"]);
    assert_eq!(
        would.stdout,
        "Would remove @t/s level: back to its preset or default\n"
    );
}

#[test]
fn the_json_form_is_the_document_and_the_files_written() {
    let dir = project();

    let ran = run(
        dir.path(),
        &["set", "@t/s", "level", "2", "--format", "json"],
    );

    assert_eq!(ran.exit, Exit::Passed, "{}", ran.stderr);
    let document: Value = serde_json::from_str(&ran.stdout).unwrap();
    assert_eq!(document["extension"], "@t/s");
    assert_eq!(document["value"], 2);
    assert_eq!(document["changed"], true);
    assert_eq!(
        document["files_written"],
        json!(["specforge.json"]),
        "{document}"
    );
    let preview = run(
        dir.path(),
        &["set", "@t/s", "level", "3", "--dry-run", "--format", "json"],
    );
    let document: Value = serde_json::from_str(&preview.stdout).unwrap();
    assert_eq!(document["dry_run"], true);
    assert!(document.get("files_written").is_none(), "{document}");
}

#[test]
fn a_refusal_exits_one_and_writes_nothing() {
    let dir = project();
    for words in [
        &["set", "@t/s", "nope", "1"][..],
        &["set", "@t/s", "level", "9"],
        &["set", "@t/s", "api_key", "hunter2-do-not-echo"],
        &["set", "@x/y", "level", "1"],
        &["unset", "@t/s", "never_written"],
    ] {
        let ran = run(dir.path(), words);
        assert_eq!(ran.exit, Exit::Failed, "{words:?}: {}", ran.stderr);
        assert_eq!(ran.exit.code(), 1);
        assert!(!ran.stderr.contains("hunter2"), "{}", ran.stderr);
        assert_eq!(config(dir.path()), CONFIG, "{words:?}");
    }
}

#[test]
fn the_listing_picks_one_setting_by_its_name() {
    let dir = project();

    let ran = run(
        dir.path(),
        &["--pick", ".extensions[].settings[name=profile].value"],
    );
    assert_eq!(
        (ran.exit, ran.stdout.as_str()),
        (Exit::Passed, "strict\n"),
        "{}",
        ran.stderr
    );
    let ran = run(
        dir.path(),
        &[
            "--format",
            "json",
            "--pick",
            ".extensions[].settings[name=profile].origin",
        ],
    );
    assert_eq!(ran.stdout, "explicit\n", "{}", ran.stderr);
    // an unset setting is not listed without --all, and has no value with it: nothing prints
    let ran = run(
        dir.path(),
        &[
            "--all",
            "--pick",
            ".extensions[].settings[name=api_key].value",
        ],
    );
    assert_eq!(
        (ran.exit, ran.stdout.as_str()),
        (Exit::Passed, ""),
        "{}",
        ran.stderr
    );
}

#[test]
fn the_listing_pick_refuses_an_explicit_human_format() {
    let dir = project();

    let ran = run(dir.path(), &["--format", "human", "--pick", ".count"]);
    assert_eq!(ran.exit.code(), 2, "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(ran.stderr.contains("--pick"), "{}", ran.stderr);
}
