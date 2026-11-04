//! What a server, a guest or a checker's memo says reaches a terminal printable (ADR 0244), and what a `.spec` string's
//! escapes read to.

use specforge_common::Diagnostic;
use specforge_common::remedy::printable;
use specforge_effects::EffectsMode;
use specforge_effects::testing::{FakeHttp, MemoryConsent};
use specforge_extension_sdk::prelude::*;
use specforge_project::values::CheckStatus;
use specforge_wasm::testing::InProcessRuntime;

use crate::value_probes::{A_URL, EXT, Scene, coded, described};

/// What sets a terminal's clipboard.
const HOSTILE: &str = "\u{1b}]52;c;aGk=\u{7}";

fn told_body(item: &CheckItem) -> Vec<CheckFinding> {
    item.outcomes
        .iter()
        .map(|outcome| {
            let text = match &outcome.data {
                OutcomeData::Http {
                    body, final_url, ..
                } => {
                    format!("{}{final_url}", String::from_utf8_lossy(body))
                }
                other => format!("{other:?}"),
            };
            CheckFinding::new("E001", text).with_help(HOSTILE.to_string())
        })
        .collect()
}

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    described(&mut c);
    c.capability(Capability::http("links").from_values().why("to ask"));
    c.checker("link", |k| {
        k.applies_to(&["string"])
            .scope("links")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::HttpGet {
                    url: item.text.clone(),
                    range: None,
                    max_bytes: 1024,
                }])
            })
            .judge(told_body);
    });
    c.checker("pure", |k| {
        k.applies_to(&["string"])
            .codes(&["E001"])
            .plan(|_: &CheckItem| PlannedItem::findings(Vec::new()));
    });
    c.kind("service", |k| {
        k.keyword("service");
        k.field("link", |f| {
            f.field_type(FieldType::String).check("link");
        });
        k.field("text", |f| {
            f.field_type(FieldType::String).check("pure");
        });
    });
    c
}

fn scene(http: FakeHttp, spec: &str) -> Scene {
    Scene::with(http, spec, &[EXT], InProcessRuntime::new().with(extension))
}

fn assert_printable(found: &[Diagnostic]) {
    for d in found {
        assert_eq!(printable(&d.message), d.message.as_str(), "{d:?}");
        let help = d.suggestion.as_deref().unwrap_or_default();
        assert_eq!(printable(help), help, "{d:?}");
    }
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "an escape sequence in a probe's answer reaches a finding printable"
)]
fn an_escape_in_a_probes_answer_reaches_a_finding_printable() {
    let spec = format!("service a \"A\" {{\n  link \"{A_URL}\"\n}}\n");
    let scene = scene(FakeHttp::new().route(A_URL, 200, HOSTILE), &spec);
    let found = scene.online().diagnostics();
    let finding = coded(&found, "pin.probes/E001");
    assert_eq!(finding.len(), 1, "{found:?}");
    assert!(finding[0].message.contains("\\u{1b}]52"), "{found:?}");
    assert_printable(&found);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a checker's memo read from disk reaches a finding printable"
)]
fn a_memo_read_from_disk_reaches_a_finding_printable() {
    let spec = format!("service a \"A\" {{\n  link \"{A_URL}\"\n}}\n");
    let scene = scene(FakeHttp::new().route(A_URL, 200, "plain"), &spec);
    scene.online();
    let books = scene.dir.root().join(".specforge/effects/checks");
    let book = std::fs::read_dir(&books)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let text = std::fs::read_to_string(&book).unwrap();
    let forged = text.replace(
        "plain",
        &HOSTILE
            .replace('\u{1b}', "\\u001b")
            .replace('\u{7}', "\\u0007"),
    );
    assert_ne!(forged, text);
    std::fs::write(&book, forged).unwrap();
    let found = scene
        .checked(EffectsMode::Cached, &MemoryConsent::default())
        .diagnostics();
    let finding = coded(&found, "pin.probes/E001");
    assert_eq!(finding.len(), 1, "{found:?}");
    assert!(finding[0].message.contains("\\u{1b}]52"), "{found:?}");
    assert_printable(&found);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a spec string's escape reads no control character but a newline, a tab or a return"
)]
fn a_spec_strings_escapes_read_no_other_control_character() {
    let spec = r#"service a "A" {
  text "a\u001bb\x1bc\ed"
}
"#;
    let project =
        scene(FakeHttp::new(), spec).checked(EffectsMode::Cached, &MemoryConsent::default());
    let row = &project.value_checks().of("a")[0];
    assert_eq!(row.status, CheckStatus::Passed);
    assert_eq!(row.value, r"a\u001bb\x1bc\ed", "kept as written");
    assert_eq!(printable(&row.value), row.value.as_str());
}
