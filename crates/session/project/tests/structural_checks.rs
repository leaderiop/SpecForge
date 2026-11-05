//! The structural checks over a compiled project's entities: the order they report in, the gate that switches the kind
//! checks off, which fields a files check reads, and which rules fire.
//! Each pin that encodes a bug says which ticket flips it.

use specforge_common::Diagnostic;
use specforge_extension_sdk::PeerDependency;
use specforge_extension_sdk::prelude::*;
use specforge_project::testing::TestProject;
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

/// A project loading `extensions`, with `a.spec` as given.
fn project(extensions: &[&str], spec: &str) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(extensions)
        .installing_enabled()
        .file("a.spec", spec)
}

/// The compile's diagnostics as `[code, message]` pairs, keeping only
/// `codes`, in the order the compile reported them.
fn reported(runtime: InProcessRuntime, dir: &TestProject, codes: &[&str]) -> Vec<[String; 2]> {
    let diagnostics: Vec<Diagnostic> = dir.compile_in(std::sync::Arc::new(runtime)).diagnostics();
    diagnostics
        .into_iter()
        .filter(|d| codes.contains(&d.code.as_str()))
        .map(|d| [d.code.clone(), d.message.clone()])
        .collect()
}

fn codes_of(reported: &[[String; 2]]) -> Vec<&str> {
    reported.iter().map(|[code, _]| code.as_str()).collect()
}

/// `gadget` (an integer, an enum, a reference list to gadgets), `widget`, and a rule that fires on a widget
/// linking nothing.
fn shapes() -> InProcessRuntime {
    InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@test/shapes", "1.0.0"));
        c.kind("gadget", |k| {
            k.keyword("gadget");
            k.field("size", |f| {
                f.field_type(FieldType::Integer);
            });
            k.field("mode", |f| {
                f.vocabulary(&Vocabulary::new(&["fast", "slow"]));
            });
            k.field("parts", |f| {
                f.field_type(FieldType::ReferenceList).target_kind("gadget");
            });
        });
        c.kind("widget", |k| {
            k.keyword("widget");
        });
        c.code("W900", |d| {
            d.title("A test rule").explanation("A test rule's finding.");
        });
        c.rule("W900", |r| {
            r.check(CheckKind::NoOutgoingEdges)
                .severity(ValidationSeverity::Warning)
                .target_kind("widget")
                .message_template("{kind} '{id}' links nothing");
        });
        c
    })
}

/// One entity per check, in an order that is not the checks' order, so a
/// compile that reorders them shows.
const ONE_OF_EACH: &str = "\
ref gh.issue:1 \"Unreferenced\"
wibble wb_one { }
gadget gadget { }
gadget x { }
gadget g_bogus { bogus \"1\"\n  verify unit \"x\" }
widget w_one { }
gadget g_parts { parts [w_one] }
gadget g_size { size \"big\" }
";

const ORDER: [&str; 9] = [
    "E022",
    "W012",
    "E024",
    "E013",
    "E014",
    "W020",
    "W158",
    "E061",
    "test.shapes/W900",
];

/// E022 first, the linker's, reported with the graph build's diagnostics (ADR 0117); then every
/// structural check, then the rules: W012, E024, E013, E014, W020, W158, E061, W159, then the extension's
/// rule.
#[specforge_test(
    behavior = "check_entities_in_one_order",
    verify = "a compile reports the structural checks in the order the registry build runs them"
)]
fn structural_checks_report_in_one_order() {
    let dir = project(&["@test/shapes"], ONE_OF_EACH);

    let found = reported(shapes(), &dir, &ORDER);

    assert_eq!(codes_of(&found), ORDER, "{found:#?}");
    insta::assert_json_snapshot!("one_order", found);
}

/// An extension that declares a rule and no kind.
fn kindless() -> InProcessRuntime {
    InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@test/kindless", "1.0.0"));
        c.code("W902", |d| {
            d.title("A test rule").explanation("A test rule's finding.");
        });
        c.rule("W902", |r| {
            r.check(CheckKind::NoIncomingEdges)
                .severity(ValidationSeverity::Warning)
                .target_kind("ref")
                .message_template("{kind} '{id}' is not referenced");
        });
        c
    })
}

/// Extensions that load but declare no kind switch the kind, field and
/// identifier checks off; one W151 says so, naming the entities nobody
/// checks (no E024 for `wibble`, no E014 for the short ID, no I002).
#[test]
fn kindless_extensions_leave_entities_unchecked() {
    let dir = project(&["@test/kindless"], "wibble w { }\nthing ab { }\n");

    let found = reported(
        kindless(),
        &dir,
        &[
            "E024", "E013", "E014", "W020", "E022", "E061", "W151", "I002",
        ],
    );

    assert_eq!(codes_of(&found), ["W151"], "{found:#?}");
    assert_eq!(
        found[0][1],
        "the loaded extensions declare no entity kind: 2 entities (kinds: thing, wibble) are not checked against kinds, fields or identifiers"
    );
}

/// A rule targeting a kind `@test/absent` would declare, and nobody loaded.
fn ghost_rules() -> InProcessRuntime {
    InProcessRuntime::new().with(|| {
        let mut meta = ExtensionMeta::new("@test/ghost-rules", "1.0.0");
        meta.peer_dependencies = vec![PeerDependency {
            name: "@test/absent".to_string(),
            version: "^1.0".to_string(),
            optional: true,
        }];
        let mut c = ContributionsBuilder::new(meta);
        // A declared kind, so the kind checks run (E024 for `ghost`).
        c.kind("anchor", |k| {
            k.keyword("anchor");
        });
        c.code("W901", |d| {
            d.title("A test rule").explanation("A test rule's finding.");
        });
        c.rule("W901", |r| {
            r.check(CheckKind::NoIncomingEdges)
                .severity(ValidationSeverity::Warning)
                .target_kind("ghost")
                .message_template("{kind} '{id}' is not referenced");
        });
        c
    })
}

/// ADR 0020 D5: a rule for an undeclared kind is inert. It does not fire on
/// that keyword's entities, which are E024.
#[test]
fn a_rule_is_inert_for_an_undeclared_target_kind() {
    let dir = project(&["@test/ghost-rules"], "ghost g1 { }\n");

    let found = reported(ghost_rules(), &dir, &["E024", "test.ghost-rules/W901"]);

    assert_eq!(codes_of(&found), ["E024"], "{found:#?}");
}

/// `doc` has the fields a project attaches `@specforge/checks:file` to (`paths`, `guide`); `note` has a field
/// named `paths` that none is attached to. The builtins' component beside it serves `@specforge/checks`.
fn docs() -> InProcessRuntime {
    InProcessRuntime::new()
        .with(|| {
            let mut c = ContributionsBuilder::new(ExtensionMeta::new("@test/docs", "1.0.0"));
            c.kind("doc", |k| {
                k.keyword("doc");
                k.field("paths", |f| {
                    f.field_type(FieldType::StringList);
                });
                k.field("guide", |f| {
                    f.field_type(FieldType::String);
                });
            });
            c.kind("note", |k| {
                k.keyword("note");
                k.field("paths", |f| {
                    f.field_type(FieldType::StringList);
                });
            });
            c
        })
        .beside(specforge_project::testing::component_runtime())
}

/// A project loading `@test/docs` and `@specforge/checks`, whose `doc` fields are read from `root`.
fn docs_project(root: &str, spec: &str) -> TestProject {
    let file =
        serde_json::json!([{ "checker": "@specforge/checks:file", "args": { "root": root } }]);
    project(&["@test/docs", "@specforge/checks"], spec).config(|c| {
        c["checks"] = serde_json::json!({ "fields": { "doc.paths": file, "doc.guide": file } })
    })
}

/// `note.paths` is not checked: the check is attached to `doc.paths` only.
#[specforge_test(
    behavior = "ck_files_family",
    verify = "a field of the same name on another kind is not checked"
)]
fn a_field_named_like_another_kinds_checked_field_is_not_checked() {
    let dir = docs_project("project", "note n1 { paths [\"alpha\"] }\n");

    let found = reported(docs(), &dir, &["E016"]);

    assert!(found.is_empty(), "{found:#?}");
}

/// A single path on a `string` field is checked like a one-item list.
#[specforge_test(
    behavior = "ck_files_family",
    verify = "a single path on a string field is checked like a one-item list"
)]
fn a_single_path_is_checked() {
    let dir = docs_project("project", "doc d1 { guide \"missing.md\" }\n");

    let found = reported(docs(), &dir, &["E016"]);

    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0][1].contains("'missing.md'"), "{found:#?}");
}

/// A bare word on a `string` field is a path; the graph stores it as an identifier, which the checker reads as the
/// one word it is.
#[specforge_test(
    behavior = "ck_files_family",
    verify = "a compile checks a bare word as a path"
)]
fn a_bare_word_is_checked() {
    let dir = docs_project("project", "doc d1 { guide README }\n");

    let found = reported(docs(), &dir, &["E016"]);

    assert_eq!(
        found,
        [[
            "E016".to_string(),
            "'README' does not exist under <project>".to_string()
        ]]
    );
}

/// With `spec_root: spec` and a check whose `root` is `spec`, the paths start at the spec root, and a close sibling
/// there is the suggestion.
#[specforge_test(
    behavior = "ck_files_family",
    verify = "a path is read from the root its check names and suggests a close sibling there"
)]
fn a_file_is_read_from_its_checks_root_and_suggests_a_sibling() {
    let dir = docs_project("spec", "")
        .config(|c| c["spec_root"] = "spec".into())
        .file(
            "spec/a.spec",
            "doc d1 { guide \"guid.md\" }\ndoc d2 { guide \"src/x.ts\" }\n",
        )
        .file("spec/guide.md", "")
        .file("src/x.ts", "");

    let diagnostics: Vec<Diagnostic> = dir.compile_in(std::sync::Arc::new(docs())).diagnostics();
    let e016: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();

    assert_eq!(e016.len(), 2, "{e016:#?}");
    assert!(e016[0].message.contains("'guid.md'"), "{e016:#?}");
    assert_eq!(
        e016[0].suggestion.as_deref(),
        Some("did you mean 'guide.md'?")
    );
    assert!(e016[1].message.contains("'src/x.ts'"), "{e016:#?}");
}
