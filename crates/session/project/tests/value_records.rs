//! A record is the extension's that asked (ADR 0336): whatever the mode, one extension's answer, fetched with its secret
//! or through a redirect its grant permits, answers no other.

use specforge_effects::testing::{FakeHttp, MemoryConsent};
use specforge_effects::{Consent, EffectsMode, OutcomeClass};
use specforge_extension_sdk::prelude::*;
use specforge_project::CompiledProject;
use specforge_project::values::CheckStatus;
use specforge_wasm::testing::InProcessRuntime;

use crate::value_probes::{A_URL, EXT, OTHER, Scene, coded, described, head, judged_by_class};

/// An extension that asks `a.example` with its secret attached, and one field that does.
fn with_secret(name: &str, keyword: &str) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
    described(&mut c);
    c.secret("token", "PIN_TOKEN", "Asks a.example as the extension");
    c.capability(
        Capability::http("api")
            .hosts(&["a.example"])
            .schemes(&["https"])
            .secret_header("authorization", "token", Some("Bearer"))
            .why("to ask"),
    );
    c.checker("link", |k| {
        k.applies_to(&["string"])
            .scope("api")
            .codes(&["E001"])
            .plan(head)
            .judge(judged_by_class);
    });
    c.kind(keyword, |k| {
        k.keyword(keyword);
        k.field("link", |f| {
            f.field_type(FieldType::String).check("link");
        });
    });
    c
}

fn first() -> ContributionsBuilder {
    with_secret(EXT, "service")
}

fn second() -> ContributionsBuilder {
    with_secret(OTHER, "widget")
}

/// Two extensions over one fake network, each with its own secret-bearing capability; the first entity is `a` of
/// `service`, the second `b` of `widget`, as written.
fn two(http: FakeHttp, spec: &str) -> Scene {
    let runtime = InProcessRuntime::new().with(first).with(second);
    Scene::with(http, spec, &[EXT, OTHER], runtime)
}

fn a_spec() -> String {
    format!("service a \"A\" {{\n  link \"{A_URL}\"\n}}\n")
}

fn b_spec() -> String {
    format!("widget b \"B\" {{\n  link \"{A_URL}\"\n}}\n")
}

fn status_of(project: &CompiledProject, id: &str) -> CheckStatus {
    project.value_checks().of(id)[0].status
}

/// `A`'s secret went on the wire.
fn carried_a_secret(scene: &Scene) -> bool {
    scene.http.requests().iter().all(|r| r.secret.is_some())
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a record fetched with one extension's secret answers no other extension in any mode"
)]
fn a_record_fetched_with_a_secret_answers_no_other_extension() {
    let scene = two(FakeHttp::new().route(A_URL, 200, "for a"), &a_spec());
    let asked = scene.online();
    assert_eq!(status_of(&asked, "a"), CheckStatus::Passed);
    assert_eq!(scene.http.requests().len(), 1);
    assert!(carried_a_secret(&scene), "the probe path attaches it");

    scene.dir.write("a.spec", b_spec());
    for mode in [EffectsMode::Cached, EffectsMode::Auto] {
        let project = scene.checked(mode, &MemoryConsent::default());
        assert_eq!(status_of(&project, "b"), CheckStatus::NotRun, "{mode:?}");
        assert_eq!(
            project.value_checks().of("b")[0].outcome_class,
            None,
            "{mode:?}"
        );
        let found = project.diagnostics();
        assert_eq!(coded(&found, "I206").len(), 1, "{mode:?}: {found:?}");
    }
    let replayed = scene.checked(EffectsMode::Replay, &MemoryConsent::default());
    assert_eq!(status_of(&replayed, "b"), CheckStatus::NotRun, "a miss");
    assert_eq!(
        replayed.effects().log().misses.len(),
        1,
        "replay holds no record of `b`'s asking"
    );
    assert_eq!(scene.http.requests().len(), 1, "no mode here performed");

    // Its own record still answers it, in each; `auto` under its approval.
    scene.dir.write("a.spec", a_spec());
    let consent = MemoryConsent::default();
    consent.approve(scene.approval(EXT, "api")).unwrap();
    for mode in [EffectsMode::Cached, EffectsMode::Auto, EffectsMode::Replay] {
        let project = scene.checked(mode, &consent);
        assert_eq!(status_of(&project, "a"), CheckStatus::Passed, "{mode:?}");
        assert!(project.value_checks().of("a")[0].from_cache, "{mode:?}");
    }
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a 401 one extension was answered does not answer another"
)]
fn one_extensions_401_does_not_answer_another() {
    let scene = two(FakeHttp::new().route(A_URL, 401, ""), &b_spec());
    let refused = scene.online();
    assert_eq!(
        refused.value_checks().of("b")[0].outcome_class,
        Some(OutcomeClass::Status(401))
    );

    scene.dir.write("a.spec", a_spec());
    let cached = scene.checked(EffectsMode::Cached, &MemoryConsent::default());
    assert_eq!(status_of(&cached, "a"), CheckStatus::NotRun);
    assert_eq!(cached.value_checks().of("a")[0].outcome_class, None);
    let found = cached.diagnostics();
    assert!(coded(&found, "pin.probes/E001").is_empty(), "{found:?}");

    // Approved, it asks for itself.
    let consent = MemoryConsent::default();
    consent.approve(scene.approval(EXT, "api")).unwrap();
    scene.checked(EffectsMode::Auto, &consent);
    assert_eq!(
        scene.http.requests().len(),
        2,
        "not answered from `b`'s 401"
    );
}

/// An extension whose grant reaches only `a.example`, and one that follows a redirect anywhere.
fn near() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(OTHER, "1.0.0"));
    described(&mut c);
    c.capability(
        Capability::http("only-a")
            .hosts(&["a.example"])
            .why("to ask"),
    );
    c.checker("link", |k| {
        k.applies_to(&["string"])
            .scope("only-a")
            .codes(&["E001"])
            .plan(head)
            .judge(judged_by_class);
    });
    c.kind("widget", |k| {
        k.keyword("widget");
        k.field("link", |f| {
            f.field_type(FieldType::String).check("link");
        });
    });
    c
}

fn anywhere() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    described(&mut c);
    c.capability(Capability::http("links").from_values().why("to ask"));
    c.checker("link", |k| {
        k.applies_to(&["string"])
            .scope("links")
            .codes(&["E001"])
            .plan(head)
            .judge(judged_by_class);
    });
    c.kind("service", |k| {
        k.keyword("service");
        k.field("link", |f| {
            f.field_type(FieldType::String).check("link");
        });
    });
    c
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a redirect one extension's grant permits reaches no other extension without its own grant"
)]
fn a_redirect_one_grant_permits_reaches_no_other_extension() {
    let there = "https://b.example/landed";
    let http = FakeHttp::new().redirect(A_URL, there).route(there, 200, "");
    let runtime = InProcessRuntime::new().with(anywhere).with(near);
    let scene = Scene::with(http, &a_spec(), &[EXT, OTHER], runtime);
    assert_eq!(status_of(&scene.online(), "a"), CheckStatus::Passed);

    scene.dir.write("a.spec", b_spec());
    let cached = scene.checked(EffectsMode::Cached, &MemoryConsent::default());
    assert_eq!(status_of(&cached, "b"), CheckStatus::NotRun);

    // Asked under its own grant, the same request is stopped at the hop.
    let consent = MemoryConsent::default();
    consent.approve(scene.approval(OTHER, "only-a")).unwrap();
    let auto = scene.checked(EffectsMode::Auto, &consent);
    let found = auto.diagnostics();
    assert_eq!(coded(&found, "E090").len(), 1, "{found:?}");
    assert_eq!(status_of(&auto, "b"), CheckStatus::Failed);
}

/// Windows sets no mode, so it cannot tell a committed record this way.
#[cfg(unix)]
#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a committed effect record or checker memo forges no pass"
)]
fn a_committed_record_or_memo_forges_no_pass() {
    use std::os::unix::fs::PermissionsExt as _;

    let scene = two(FakeHttp::new().route(A_URL, 200, ""), &a_spec());
    assert_eq!(status_of(&scene.online(), "a"), CheckStatus::Passed);
    let cached = || scene.checked(EffectsMode::Cached, &MemoryConsent::default());
    assert_eq!(status_of(&cached(), "a"), CheckStatus::Passed);

    // As a git checkout writes them.
    fn open_all(dir: &std::path::Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                open_all(&path);
            } else {
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            }
        }
    }
    open_all(&scene.dir.root().join(".specforge/effects"));
    let project = cached();
    assert_eq!(status_of(&project, "a"), CheckStatus::NotRun);
    assert!(!project.value_checks().of("a")[0].from_cache);
}
