//! Settings in the operations: analyze gates and raises, a command sees a secret's presence (ADR 0288).

use std::sync::Arc;

use serde_json::json;
use specforge_common::Severity;
use specforge_extension_sdk::prelude::*;
use specforge_ops::analysis::analyze::{AnalyzeOptions, analyze};
use specforge_ops::view::ProjectView;
use specforge_project::SharedRuntime;
use specforge_project::testing::compile_at;
use specforge_wasm::testing::InProcessRuntime;
use tempfile::TempDir;

/// `@t/a`: an analyze pass reporting `W901` (gated by the bool `flag`, default false) and `W902` (raised to an
/// error by the preset of the enum `profile` while it is `strict`), both warnings.
fn extension() -> InProcessRuntime {
    InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/a", "1.0.0"));
        for code in ["W901", "W902"] {
            c.code(code, |d| {
                d.title("A finding").explanation("A settings test finding.");
            });
        }
        c.setting("flag", |s| {
            s.bool().default(false).gates(&["W901"]);
        });
        c.setting("profile", |s| {
            s.enum_values(&["compat", "strict"]).default("compat");
        });
        c.preset("profile", |p| {
            p.when("strict", |e| {
                e.raise("W902", ValidationSeverity::Error);
            });
        });
        c.pass("scan", |p| {
            p.run(|_: &PassInput| {
                vec![
                    PassDiagnostic::new("W901", PassSeverity::Warning, "gated"),
                    PassDiagnostic::new("W902", PassSeverity::Warning, "raised"),
                ]
            });
        });
        c
    })
}

/// What `analyze` reports over a project whose `specforge.json` sets `settings`, `strict` as given.
fn analyzed(settings: serde_json::Value, strict: bool) -> Vec<(String, Severity)> {
    let dir = TempDir::new().unwrap();
    let config =
        json!({"name": "p", "version": "0.1.0", "extensions": ["@t/a"], "settings": settings});
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    specforge_installed::testing::install(dir.path(), &extension(), &["@t/a"]);
    std::fs::write(dir.path().join("main.spec"), "behavior a \"A\" {\n}\n").unwrap();
    let shared: SharedRuntime = Arc::new(extension());
    let compiled = compile_at(dir.path(), Some(shared));
    let options = AnalyzeOptions {
        strict,
        ..AnalyzeOptions::default()
    };
    let outcome = analyze(&ProjectView::of(&compiled), &options).unwrap();
    outcome
        .passes
        .iter()
        .flat_map(|p| &p.findings)
        .map(|d| (d.code.rsplit('/').next().unwrap().to_string(), d.severity))
        .filter(|(code, _)| code.starts_with("W9"))
        .collect()
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "analyze findings are gated and raised as the project's report is"
)]
fn analyze_findings_are_gated_and_raised_too() {
    // The same rule on an analyze pass's findings, before `policy.promote`.
    let w = |code: &str, severity| (code.to_string(), severity);
    assert_eq!(analyzed(json!({}), false), [w("W902", Severity::Warning)]);
    let on = json!({"@t/a": {"flag": true}});
    assert_eq!(
        analyzed(on.clone(), false),
        [w("W901", Severity::Warning), w("W902", Severity::Warning)]
    );
    let strict = json!({"@t/a": {"profile": "strict"}});
    assert_eq!(analyzed(strict, false), [w("W902", Severity::Error)]);
}

#[specforge_test_macros::test(
    behavior = "exit_code_reflects_diagnostic_severity",
    verify = "a preset raises a code before strict, in every surface"
)]
fn strict_promotes_what_the_raise_left_as_warnings() {
    let on = json!({"@t/a": {"flag": true, "profile": "strict"}});
    assert_eq!(
        analyzed(on.clone(), false),
        [
            ("W901".to_string(), Severity::Warning),
            ("W902".to_string(), Severity::Error)
        ]
    );
    assert_eq!(
        analyzed(on, true),
        [
            ("W901".to_string(), Severity::Error),
            ("W902".to_string(), Severity::Error)
        ]
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a command and a tool see whether a secret is set and never its value"
)]
#[ignore = "K6 T7"]
fn a_command_sees_secret_presence_not_its_value() {
    // `secrets.api_key` is true with T_API_KEY set; the value is in no recorded input.
    unimplemented!("K6 T7: secrets");
}
