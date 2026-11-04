//! A checker's `when` settings (ADR 0338 D3): while one is off the checker is not called, sends no probe and asks no
//! scope, and its value owes nothing.

use std::sync::Arc;

use serde_json::json;
use specforge_effects::EffectsMode;
use specforge_effects::testing::{FakeHttp, MemoryConsent};
use specforge_extension_sdk::prelude::*;
use specforge_project::testing::TestProject;
use specforge_project::values::CheckStatus;
use specforge_wasm::testing::InProcessRuntime;

use crate::value_probes::{A_URL, EXT, Scene, described, head, judged_by_class};

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    described(&mut c);
    c.setting("families.links", |s| {
        s.bool().default(true);
    });
    c.setting("level", |s| {
        s.string();
    });
    c.capability(Capability::http("links").from_values().why("to ask"));
    c.checker("link", |k| {
        k.applies_to(&["string"])
            .scope("links")
            .codes(&["E001"])
            .when("families.links")
            .plan(head)
            .judge(judged_by_class);
    });
    c.checker("always", |k| {
        k.applies_to(&["string"])
            .codes(&["E001"])
            .plan(|_| PlannedItem::findings(Vec::new()));
    });
    c.checker("on_a_word", |k| {
        k.applies_to(&["string"])
            .codes(&["E001"])
            .when("level")
            .plan(|_| PlannedItem::findings(Vec::new()));
    });
    c.kind("service", |k| {
        k.keyword("service");
        k.field("link", |f| {
            f.field_type(FieldType::String).check("link");
        });
        k.field("either", |f| {
            f.field_type(FieldType::String)
                .any_of([CheckRef::named("link"), CheckRef::named("always")]);
        });
        k.field("word", |f| {
            f.field_type(FieldType::String).check("on_a_word");
        });
    });
    c
}

fn scene(on: Option<bool>, spec: &str) -> (Scene, Arc<InProcessRuntime>) {
    let runtime = Arc::new(InProcessRuntime::new().with(extension));
    let dir = TestProject::new()
        .config(|c| {
            c["name"] = "p".into();
            if let Some(on) = on {
                c["settings"] = json!({ EXT: { "families.links": on } });
            }
        })
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", spec);
    let scene = Scene {
        dir,
        runtime: runtime.clone(),
        http: Arc::new(FakeHttp::new().route(A_URL, 200, "")),
    };
    (scene, runtime)
}

fn check_calls(runtime: &InProcessRuntime) -> usize {
    runtime
        .calls()
        .iter()
        .filter(|call| call.export == "__check_values")
        .count()
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a checker whose when setting is off is not called and asks no scope"
)]
fn a_checker_whose_when_setting_is_off_is_not_called_and_asks_no_scope() {
    let spec = format!("service a \"A\" {{\n  link \"{A_URL}\"\n}}\n");
    let (scene, runtime) = scene(Some(false), &spec);
    let project = scene.checked(EffectsMode::Auto, &MemoryConsent::default());
    let found = project.diagnostics();
    assert_eq!(check_calls(&runtime), 0, "{:?}", runtime.calls());
    assert!(scene.http.asked().is_empty());
    assert!(
        !found.iter().any(|d| d.code == "W170" || d.code == "I206"),
        "{found:?}"
    );
    assert!(project.value_checks().of("a").is_empty());
}

#[test]
fn a_checker_whose_when_setting_is_on_runs_as_any_other() {
    for on in [None, Some(true)] {
        let spec = format!("service a \"A\" {{\n  link \"{A_URL}\"\n}}\n");
        let (scene, runtime) = scene(on, &spec);
        let project = scene.checked(EffectsMode::Auto, &MemoryConsent::default());
        let found = project.diagnostics();
        assert_eq!(check_calls(&runtime), 1, "{on:?}");
        assert!(found.iter().any(|d| d.code == "W170"), "{found:?}");
        assert_eq!(
            project.value_checks().of("a")[0].status,
            CheckStatus::NotRun
        );
    }
}

#[test]
fn an_any_of_with_a_branch_off_is_judged_by_the_rest() {
    let (scene, _) = scene(Some(false), "service a \"A\" {\n  either \"x\"\n}\n");
    let project = scene.checked(EffectsMode::Auto, &MemoryConsent::default());
    let rows = project.value_checks().of("a");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].checker.checker, "always");
    assert_eq!(rows[0].status, CheckStatus::Passed);
    assert!(scene.http.asked().is_empty());
}

#[test]
fn a_when_that_names_no_bool_setting_drops_the_checker() {
    let (scene, _) = scene(None, "service a \"A\" {\n  word \"x\"\n}\n");
    let project = scene.checked(EffectsMode::Cached, &MemoryConsent::default());
    let found = project.diagnostics();
    assert!(
        found
            .iter()
            .any(|d| d.code == "W189" && d.message.contains("'level'")),
        "{found:?}"
    );
    assert!(project.value_checks().of("a").is_empty());
}
