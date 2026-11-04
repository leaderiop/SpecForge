//! A checked path or URL value in the editor (ADR 0336 D5): its completion and its links, from the form its checker
//! declares and only inside the roots.

use specforge_extension_sdk::prelude::*;
use specforge_lsp::answers;
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::{CompletionResponse, Position};

use crate::served::Served;

const EXT: &str = "@pin/linked";

fn declare(c: &mut ContributionsBuilder) {
    c.checker("named", |k| {
        k.applies_to(&["string"])
            .value(ValueForm::Path)
            .root_arg("root")
            .arg("root", |a| {
                a.field_type(FieldType::String);
            })
            .plan(|_: &CheckItem| PlannedItem::findings(Vec::new()));
    });
    c.checker("address", |k| {
        k.applies_to(&["string"])
            .value(ValueForm::Url)
            .plan(|_: &CheckItem| PlannedItem::findings(Vec::new()));
    });
    c.kind("service", |k| {
        k.keyword("service");
        k.field("doc", |f| {
            f.field_type(FieldType::String)
                .check_with("named", serde_json::json!({"root": "project"}));
        });
        k.field("site", |f| {
            f.field_type(FieldType::String).check("address");
        });
        k.field("note", |f| {
            f.field_type(FieldType::String);
        });
    });
}

/// A project holding `docs/guide.md`, `docs/more/deep.md`, `.env` and `a.spec`, a service writing `doc <value>`.
fn served(value: &str) -> Served {
    let spec = format!("service a \"A\" {{\n  doc \"{value}\"\n}}\n");
    Served::new(&[
        ("a.spec", &spec),
        ("docs/guide.md", "guide"),
        ("docs/more/deep.md", "deep"),
        ("docs/.env", "SECRET=1"),
    ])
    .extension(EXT, declare)
    .open(&["a.spec"])
}

/// The labels completing at the end of `value`.
fn completed(value: &str) -> Vec<String> {
    let served = served(value);
    let at = Position::new(1, 7 + value.len() as u32);
    let answer =
        answers::completion(&served.state(), &served.uri("a.spec"), at).expect("an answer");
    let CompletionResponse::Array(items) = answer else {
        panic!("an array");
    };
    items.into_iter().map(|item| item.label).collect()
}

fn linked(spec: &str, files: &[(&str, &str)]) -> Vec<String> {
    let mut all = vec![("a.spec", spec)];
    all.extend_from_slice(files);
    let served = Served::new(&all).extension(EXT, declare).open(&["a.spec"]);
    let links = specforge_lsp::links::document_links(&served.state(), &served.uri("a.spec"))
        .expect("the document is open");
    let root = served.root();
    links
        .into_iter()
        .map(|link| {
            let target = link.target.expect("a target").to_string();
            let root = format!("file://{}", root.display());
            target.replace(&root, "<root>")
        })
        .collect()
}

#[spec(
    behavior = "complete_checked_paths",
    verify = "a path value completes the root prefixes, then the entries inside the root"
)]
fn a_path_value_completes_root_prefixes_then_entries_inside_the_root() {
    assert_eq!(completed(""), ["<spec>/", "<project>/"]);
    assert_eq!(completed("<sp"), ["<spec>/"]);
    assert_eq!(completed("<project>/docs/"), ["guide.md", "more/"]);
    assert_eq!(completed("docs/g"), ["guide.md"]);
    assert_eq!(completed("docs/more/"), ["deep.md"]);
    // A hidden name offers itself only when asked for, and a withheld one never.
    assert_eq!(completed("docs/."), Vec::<String>::new());
}

#[spec(
    behavior = "complete_checked_paths",
    verify = "a spec root outside the project offers no prefix and no entry"
)]
fn a_spec_root_outside_the_project_offers_no_prefix_and_no_entry() {
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.md"), "x").unwrap();
    let completed = |value: &str| {
        let spec = format!("service a \"A\" {{\n  doc \"{value}\"\n}}\n");
        let served = Served::new(&[("a.spec", &spec), ("docs/guide.md", "guide")])
            .extension(EXT, declare)
            .spec_root(outside.path())
            .open(&["a.spec"]);
        let at = Position::new(1, 7 + value.len() as u32);
        let answer =
            answers::completion(&served.state(), &served.uri("a.spec"), at).expect("an answer");
        let CompletionResponse::Array(items) = answer else {
            panic!("an array");
        };
        items.into_iter().map(|item| item.label).collect::<Vec<_>>()
    };
    assert_eq!(completed(""), ["<project>/"]);
    assert!(completed("<spec>/").is_empty());
    assert_eq!(completed("<project>/docs/"), ["guide.md"]);
}

#[spec(
    behavior = "complete_checked_paths",
    verify = "a path prefix that leaves its root completes nothing"
)]
fn a_path_prefix_that_leaves_its_root_completes_nothing() {
    for value in ["../", "docs/../../", "/etc/", "<nope>/", "/e"] {
        assert_eq!(completed(value), Vec::<String>::new(), "{value}");
    }
}

#[spec(
    behavior = "complete_checked_paths",
    verify = "a string no path checker reads completes nothing"
)]
fn a_string_no_path_checker_reads_completes_nothing() {
    let spec = "service a \"A\" {\n  note \"do\"\n}\n";
    let served = Served::new(&[("a.spec", spec), ("docs/guide.md", "guide")])
        .extension(EXT, declare)
        .open(&["a.spec"]);
    let answer = answers::completion(&served.state(), &served.uri("a.spec"), Position::new(1, 10));
    let Some(CompletionResponse::Array(items)) = answer else {
        panic!("an array");
    };
    assert!(items.is_empty(), "{items:?}");
}

#[spec(
    behavior = "link_checked_values",
    verify = "a path value links to its file inside its root"
)]
fn a_path_value_links_to_its_file_inside_its_root() {
    let spec = "service a \"A\" {\n  doc \"docs/guide.md\"\n}\n";
    assert_eq!(
        linked(spec, &[("docs/guide.md", "guide")]),
        ["<root>/docs/guide.md"]
    );
}

#[spec(
    behavior = "link_checked_values",
    verify = "a path outside its root, missing, withheld or under an unknown root has no link"
)]
fn a_path_outside_its_root_or_missing_has_no_link() {
    for value in [
        "../outside.md",
        "docs/none.md",
        "<nope>/a.md",
        "/etc/hosts",
        ".env",
    ] {
        let spec = format!("service a \"A\" {{\n  doc \"{value}\"\n}}\n");
        assert_eq!(
            linked(&spec, &[(".env", "SECRET=1")]),
            Vec::<String>::new(),
            "{value}"
        );
    }
}

#[cfg(unix)]
#[spec(
    behavior = "link_checked_values",
    verify = "a link that leaves its root is neither listed nor linked"
)]
fn a_link_that_leaves_its_root_is_neither_listed_nor_linked() {
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.md"), "secret").unwrap();
    let spec = "service a \"A\" {\n  doc \"out/secret.md\"\n}\n";
    let served = Served::new(&[("a.spec", spec)]).extension(EXT, declare);
    std::os::unix::fs::symlink(outside.path(), served.root().join("out")).unwrap();
    let served = served.open(&["a.spec"]);
    let links =
        specforge_lsp::links::document_links(&served.state(), &served.uri("a.spec")).unwrap();
    assert!(links.is_empty(), "{links:?}");
}

#[spec(
    behavior = "link_checked_values",
    verify = "a URL value links to its URL when it is http or https"
)]
fn a_url_value_links_to_its_url() {
    let spec = "service a \"A\" {\n  site \"https://a.example/x\"\n}\n\nservice b \"B\" {\n  site \"ftp://a.example/x\"\n}\n";
    assert_eq!(linked(spec, &[]), ["https://a.example/x"]);
}

#[test]
fn an_editor_holds_its_checks_to_the_editors_budget() {
    let served = served("x");
    assert_eq!(
        served.state().environment().value_budget(),
        specforge_project::values::ValueBudget::EDITOR
    );
}
