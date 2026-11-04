//! `@specforge/checks` as a project runs it: the committed blob, attached to a field in `specforge.json`.

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_project::testing::TestProject;

const CHECKS: &str = "@specforge/checks";
const PRODUCT: &str = "@specforge/product";

/// A project with one feature, its `owner` written as `owner` and checked by `checks`.
pub(crate) fn feature(owner: &str, checks: Value) -> TestProject {
    TestProject::new()
        .config(|c| c["checks"] = json!({ "fields": { "feature.owner": checks } }))
        .enabling(&[PRODUCT, CHECKS])
        .file(
            "a.spec",
            format!(
                "feature f \"F\" {{\n  status planned\n  problem \"p\"\n  solution \"s\"\n  owner \"{owner}\"\n}}\n"
            ),
        )
}

fn coded(diagnostics: &[Diagnostic], code: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.code == code)
        .cloned()
        .collect()
}

#[specforge_test_macros::test(
    behavior = "ck_formats_family",
    verify = "a format that refuses a value reports E103 on the part of the value's span the kit points at"
)]
fn checks_formats_report_e103_on_the_value_span() {
    let project = feature("ada at example.com", json!(["@specforge/checks:email"])).compile();
    let diagnostics = project.diagnostics();
    let found = coded(&diagnostics, "E103");
    assert_eq!(found.len(), 1, "{diagnostics:?}");
    let span = found[0].span.as_ref().expect("on the value");
    assert_eq!(span.start_line, 5, "{span:?}");

    let at = feature(
        "1.2.3-rc.1",
        json!([{"checker": "@specforge/checks:semver", "args": {"prerelease": false}}]),
    )
    .compile()
    .diagnostics();
    let found = coded(&at, "E103");
    assert_eq!(found.len(), 1, "{at:?}");
    let span = found[0].span.as_ref().expect("on the part");
    // `  owner "` is nine columns, and the pre-release starts five bytes in.
    assert_eq!(span.start_col, 9 + 5 + 1, "{span:?}");
    assert_eq!(span.end_col, 9 + 10 + 1, "{span:?}");
}

#[specforge_test_macros::test(
    behavior = "ck_digits_family",
    verify = "a digit checker needs no scope and asks no approval"
)]
fn checks_digits_need_no_scope_and_no_approval() {
    let project = feature("79927398710", json!(["@specforge/checks:luhn"])).compile();
    let diagnostics = project.diagnostics();
    assert_eq!(coded(&diagnostics, "E103").len(), 1, "{diagnostics:?}");
    assert!(coded(&diagnostics, "W170").is_empty(), "{diagnostics:?}");
    assert!(coded(&diagnostics, "I206").is_empty(), "{diagnostics:?}");

    let clean = feature("79927398713", json!(["@specforge/checks:luhn"]))
        .compile()
        .diagnostics();
    assert!(coded(&clean, "E103").is_empty(), "{clean:?}");
}

#[specforge_test_macros::test(
    behavior = "ck_family_switches",
    verify = "a family whose switch is off reports nothing"
)]
fn checks_a_family_switched_off_reports_nothing() {
    let project = feature("79927398710", json!(["@specforge/checks:luhn"]))
        .config(|c| c["settings"] = json!({ CHECKS: { "families.digits": false } }))
        .compile();
    let diagnostics = project.diagnostics();
    assert!(coded(&diagnostics, "E103").is_empty(), "{diagnostics:?}");
}

#[specforge_test_macros::test(
    behavior = "ck_files_family",
    verify = "a missing file in a list reports E016 on that item's span, not the entity's"
)]
#[ignore = "06 T11"]
fn checks_files_e016_sits_on_the_values_span() {
    let refs = r#"  refs ["here.md", "missing.md"]"#;
    let project = TestProject::new()
        .config(|c| c["checks"] = json!({ "fields": { "feature.refs": ["@specforge/checks:file"] } }))
        .enabling(&[PRODUCT, CHECKS])
        .file("here.md", "here")
        .file(
            "a.spec",
            format!("feature f \"F\" {{\n  status planned\n  problem \"p\"\n  solution \"s\"\n{refs}\n}}\n"),
        )
        .compile();
    let diagnostics = project.diagnostics();
    let found = coded(&diagnostics, "E016");
    assert_eq!(found.len(), 1, "{diagnostics:?}");
    let span = found[0].span.as_ref().expect("on the item");
    let quote = refs.find("\"missing.md\"").unwrap() + 1;
    assert_eq!(span.start_line, 5, "{span:?}");
    assert!([quote, quote + 1].contains(&span.start_col), "{span:?}");
}
