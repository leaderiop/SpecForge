//! A setting gates its codes and a preset raises them, once, in the project's report (ADR 0288).

use serde_json::{Value, json};
use specforge_common::Severity;
use specforge_project::CompiledProject;
use specforge_project::testing::{
    FOLLOWER_EXTENSION, SETTINGS_EXTENSION, TestProject, follower_extension, settings_extension,
};
use specforge_wasm::testing::InProcessRuntime;
use std::sync::Arc;

const GADGET: &str = "gadget widget \"W\" {}\n";

/// The project, its two extensions loaded in process, over `settings`.
fn compile(settings: Value) -> CompiledProject {
    let dir = TestProject::new()
        .config(|c| {
            c["name"] = "p".into();
            c["settings"] = settings.clone();
        })
        .enabling(&[SETTINGS_EXTENSION, FOLLOWER_EXTENSION])
        .installing_enabled()
        .file("a.spec", GADGET);
    let runtime = InProcessRuntime::new()
        .with(settings_extension)
        .with(follower_extension);
    dir.compile_in(Arc::new(runtime))
}

/// The `(code, severity)` the project reports for `settings`, sorted, with the extension's namespace dropped.
fn reported(settings: Value) -> Vec<(String, Severity)> {
    let mut found: Vec<(String, Severity)> = compile(settings)
        .diagnostics()
        .into_iter()
        .filter_map(|d| {
            let code = d.code.rsplit('/').next().unwrap_or_default().to_string();
            matches!(code.as_str(), "W901" | "W902" | "W903" | "W904").then_some((code, d.severity))
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a code gated by a false bool setting is reported by no source"
)]
fn a_gated_code_is_reported_by_no_source_while_its_setting_is_false() {
    // W901 (a pass) and W903 (a declarative rule) are gone with `flag` false (its default) and back with true.
    let off = reported(json!({}));
    assert!(
        off.iter().all(|(c, _)| c != "W901" && c != "W903"),
        "{off:?}"
    );
    let on = reported(json!({"@t/s": {"flag": true}}));
    assert!(on.iter().any(|(c, _)| c == "W901"), "{on:?}");
    assert!(on.iter().any(|(c, _)| c == "W903"), "{on:?}");
    // The ungated code is untouched either way.
    assert!(
        off.contains(&("W902".to_string(), Severity::Warning)),
        "{off:?}"
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a preset raises a code upward only and the surface's strict applies after"
)]
fn a_preset_raises_a_code_and_strict_applies_after() {
    let strict = reported(json!({"@t/s": {"profile": "strict"}}));
    assert!(
        strict.contains(&("W902".to_string(), Severity::Error)),
        "{strict:?}"
    );
    // The follower's preset reads its peer's profile.
    assert!(
        strict.contains(&("W904".to_string(), Severity::Error)),
        "{strict:?}"
    );
    // The preset also sets `flag` true, so W901 is back, at its own level (never lowered).
    assert!(
        strict.contains(&("W901".to_string(), Severity::Warning)),
        "{strict:?}"
    );
    let compat = reported(json!({"@t/s": {"profile": "compat"}}));
    assert!(
        compat.contains(&("W902".to_string(), Severity::Warning)),
        "{compat:?}"
    );
    assert!(
        compat.contains(&("W904".to_string(), Severity::Warning)),
        "{compat:?}"
    );
    // A surface's strict comes after the raise: the warnings left become errors.
    let mut left: Vec<specforge_common::Diagnostic> =
        compile(json!({"@t/s": {"profile": "strict"}}))
            .diagnostics()
            .into_iter()
            .filter(|d| d.code.ends_with("/W901"))
            .collect();
    assert!(!left.is_empty());
    specforge_project::DiagnosticPolicy::strict(true).promote(&mut left);
    assert!(
        left.iter().all(|d| d.severity == Severity::Error),
        "{left:?}"
    );
}
