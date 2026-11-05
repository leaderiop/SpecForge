//! `@specforge/checks`' structured family as a project runs it: the committed blob reading the project's own files.

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_project::testing::TestProject;

/// A feature whose `refs` holds `refs` (each quoted), checked by `checker`.
fn refs_project(refs: &[&str], checker: &str, args: Value) -> TestProject {
    let items: Vec<String> = refs.iter().map(|r| format!("\"{r}\"")).collect();
    TestProject::new()
        .config(|c| {
            c["checks"] = json!({ "fields": { "feature.refs": [{ "checker": checker, "args": args }] } });
        })
        .enabling(&["@specforge/product", "@specforge/checks"])
        .file(
            "a.spec",
            format!(
                "feature f \"F\" {{\n  status planned\n  problem \"p\"\n  solution \"s\"\n  refs [{}]\n}}\n",
                items.join(", ")
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

/// The columns of the value `value` in the `refs` line, from its first character.
fn columns_of(project: &TestProject, value: &str) -> usize {
    let text = std::fs::read_to_string(project.root().join("a.spec")).unwrap();
    let line = text.lines().nth(4).unwrap();
    line.find(value).unwrap() + 1
}

#[specforge_test_macros::test(
    behavior = "checks_structured",
    verify = "a json file that does not parse is E097 at its line"
)]
fn a_json_file_that_does_not_parse_is_e097_at_its_line() {
    let project = refs_project(
        &["good.json", "bad.json"],
        "@specforge/checks:json",
        json!({}),
    )
    .file("good.json", "{\"a\": 1}")
    .file("bad.json", "{\n  \"a\": 1,\n}\n");
    let found = project.compile().diagnostics();
    let parse = coded(&found, "E097");
    assert_eq!(parse.len(), 1, "{found:?}");
    assert!(
        parse[0]
            .message
            .contains("'bad.json' does not parse as JSON"),
        "{}",
        parse[0].message
    );
    assert!(parse[0].message.ends_with("at 3:1"), "{}", parse[0].message);
}

#[specforge_test_macros::test(
    behavior = "checks_structured",
    verify = "a missing pointer is E098 on the fragment"
)]
fn a_missing_pointer_is_e098_on_the_fragment() {
    let value = "ci.yaml#/jobs/tset";
    let project = refs_project(
        &["ci.yaml#/jobs/test", value],
        "@specforge/checks:pointer",
        json!({}),
    )
    .file("ci.yaml", "jobs:\n  test:\n    runs-on: ubuntu\n");
    let found = project.compile().diagnostics();
    let missing = coded(&found, "E098");
    assert_eq!(missing.len(), 1, "{found:?}");
    assert!(
        missing[0].message.contains("'/jobs' is the deepest part"),
        "{}",
        missing[0].message
    );
    let span = missing[0].span.as_ref().expect("on the fragment");
    let hash = columns_of(&project, value) + value.find('#').unwrap();
    assert_eq!((span.start_line, span.start_col), (5, hash), "{span:?}");
}

#[specforge_test_macros::test(
    behavior = "checks_structured",
    verify = "a missing heading is E098 listing the headings"
)]
fn a_missing_heading_is_e098_listing_the_headings() {
    let project = refs_project(
        &["docs/guide.md#install", "docs/guide.md#setup"],
        "@specforge/checks:markdown_anchor",
        json!({}),
    )
    .file(
        "docs/guide.md",
        "# Guide\n\n## Install\n\n```sh\n# setup\n```\n",
    );
    let found = project.compile().diagnostics();
    let missing = coded(&found, "E098");
    assert_eq!(missing.len(), 1, "{found:?}");
    assert_eq!(missing[0].message, "'docs/guide.md' has no anchor '#setup'");
    let help = format!("{:?}", missing[0]);
    assert!(help.contains("its headings: #guide, #install"), "{help}");
}

#[specforge_test_macros::test(
    behavior = "checks_structured",
    verify = "a structured file over one MiB is W176"
)]
fn a_structured_file_over_one_mib_is_w176() {
    let large = format!("[{}0]", "0, ".repeat(400_000));
    let project = refs_project(&["large.json"], "@specforge/checks:json", json!({}))
        .file("large.json", large);
    let found = project.compile().diagnostics();
    let over = coded(&found, "W176");
    assert_eq!(over.len(), 1, "{found:?}");
    assert!(
        over[0].message.contains("over the 1 MiB"),
        "{}",
        over[0].message
    );
    assert!(coded(&found, "E097").is_empty(), "{found:?}");
}
