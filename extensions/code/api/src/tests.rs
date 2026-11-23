use specforge_code_facts::{api_facts, ApiKind, ItemKind, Visibility};
use specforge_extension_sdk::prelude::Fact;

use super::*;

/// A `cargo public-api` snapshot, hand-written to the tool's own snapshot lines: crate `example_api`.
const RUST: &str = include_str!("../fixtures/rust.public-api.txt");
/// An api-extractor `.api.json` doc model, hand-written to the model: package `@demo/widget`.
const WIDGET: &str = include_str!("../fixtures/widget.api.json");
/// api-extractor 7.60.1's doc model of `@demo/widget`, recorded (see `fixtures/README.md`).
const API_EXTRACTOR_7_60_1: &str = include_str!("../fixtures/api-extractor-7.60.1.widget.api.json");
/// api-extractor 7.60.1's doc model of `@demo/rich`: a constructor, an index signature, an accessor pair, a namespace.
const API_EXTRACTOR_7_60_1_RICH: &str =
    include_str!("../fixtures/api-extractor-7.60.1.rich.api.json");
/// The served `item` facts an api collector receives, as JSON.
const ITEMS: &str = include_str!("../fixtures/items.json");

fn served() -> Vec<Fact> {
    serde_json::from_str(ITEMS).expect("the served facts parse")
}

#[test]
fn an_api_extractor_model_mints_api_items_citing_their_js_items() {
    let entries = read_js_model(WIDGET).expect("the model reads");
    let facts = api_facts(&entries, &served());
    assert!(facts.iter().any(|f| f.id.ends_with("::parse~2")));
    assert!(
        facts
            .iter()
            .all(|f| !f.id.contains(".ts") && !f.id.contains(".js")),
        "{facts:?}"
    );
    insta::assert_json_snapshot!("js", facts);
}

#[test]
fn a_recorded_api_extractor_7_60_1_model_mints_api_items_of_every_member() {
    let entries = read_js_model(API_EXTRACTOR_7_60_1).expect("the recording reads");
    let read: Vec<(String, &str)> = entries
        .iter()
        .map(|e| (e.path.join("."), e.kind.as_str()))
        .collect();
    // api-extractor writes members alphabetically.
    let expected = [
        ("create", "function"),
        ("Id", "type"),
        ("Mode", "enum"),
        ("Mode.Fast", "variant"),
        ("Mode.Slow", "variant"),
        ("Options", "interface"),
        ("Options.mode", "field"),
        ("VERSION", "variable"),
        ("Widget", "class"),
        ("Widget.render", "method"),
        ("Widget.size", "field"),
    ];
    let expected: Vec<(String, &str)> = expected
        .iter()
        .map(|(path, kind)| (path.to_string(), *kind))
        .collect();
    assert_eq!(read, expected);
    assert!(entries.iter().all(|e| e.container == "@demo/widget"));
    insta::assert_json_snapshot!("api_extractor_7_60_1", api_facts(&entries, &served()));
}

#[test]
fn a_recorded_model_with_a_constructor_and_an_index_signature_reads_them_named_by_kind() {
    let entries = read_js_model(API_EXTRACTOR_7_60_1_RICH).expect("the recording reads");
    let read: Vec<(String, &str, Option<&str>)> = entries
        .iter()
        .map(|e| (e.path.join("."), e.kind.as_str(), e.tag.as_deref()))
        .collect();
    let at = |path: &str| {
        read.iter()
            .find(|(p, _, _)| p == path)
            .unwrap_or_else(|| panic!("{path} in {read:?}"))
    };
    assert_eq!(at("Base.constructor").1, "method");
    assert_eq!(at("Runnable.[]").1, "other");
    assert_eq!(at("Base.tune").2, Some("Beta"));
    assert_eq!(at("Flags").2, Some("Alpha"));
    assert_eq!(at("util.LIMIT").1, "variable");
    assert_eq!(read.iter().filter(|(p, _, _)| p == "parse").count(), 2);
    assert!(entries.iter().all(|e| e.container == "@demo/rich"));
    insta::assert_json_snapshot!("api_extractor_7_60_1_rich", api_facts(&entries, &served()));
}

#[test]
fn a_nameless_member_of_a_kind_the_reader_does_not_name_is_an_error_naming_its_place() {
    let model = |member: &str| {
        format!(
            r#"{{"kind": "Package", "name": "p", "members": [{{"kind": "EntryPoint", "name": "",
            "members": [{{"kind": "Interface", "name": "I", "members": [{member}]}}]}}]}}"#
        )
    };
    let named: Vec<String> = [
        "CallSignature",
        "ConstructSignature",
        "IndexSignature",
        "Constructor",
    ]
    .iter()
    .map(|kind| {
        let entries = read_js_model(&model(&format!(r#"{{"kind": "{kind}"}}"#)))
            .unwrap_or_else(|e| panic!("{kind}: {e}"));
        entries[1].path[1].clone()
    })
    .collect();
    assert_eq!(named, ["()", "new()", "[]", "constructor"]);
    for member in [r#"{"kind": "Mystery"}"#, "{}"] {
        let error = read_js_model(&model(member)).expect_err(member);
        assert!(
            error.contains("members[0].members[0].members[0]"),
            "{error}"
        );
    }
}

#[test]
fn each_member_reads_its_kind_path_signature_and_tag() {
    let entries = read_js_model(WIDGET).expect("the model reads");
    let read: Vec<(String, &str)> = entries
        .iter()
        .map(|e| (e.path.join("."), e.kind.as_str()))
        .collect();
    let expected = [
        ("Widget", "class"),
        ("Widget.render", "method"),
        ("Widget.size", "field"),
        ("Options", "interface"),
        ("create", "function"),
        ("parse", "function"),
        ("parse", "function"),
        ("Mode", "enum"),
        ("Mode.Fast", "variant"),
        ("Id", "type"),
        ("VERSION", "variable"),
        ("util", "namespace"),
    ];
    let expected: Vec<(String, &str)> = expected
        .iter()
        .map(|(path, kind)| (path.to_string(), *kind))
        .collect();
    assert_eq!(read, expected);
    let create = &entries[4];
    assert_eq!(create.container, "@demo/widget");
    assert_eq!(
        create.signature,
        "export declare function create(options?: Options): Widget;"
    );
    assert_eq!(create.tag.as_deref(), Some("Public"));
    assert_eq!(entries[2].tag.as_deref(), Some("Beta"));
    assert!(entries
        .iter()
        .all(|e| e.visibility == Visibility::Public && e.owner.is_none()));
}

#[test]
fn an_unknown_member_kind_reads_other() {
    let model = r#"{"kind": "Package", "name": "p", "members": [{"kind": "EntryPoint", "name": "",
        "members": [{"kind": "CallSignature", "name": "(call)"}]}]}"#;
    let entries = read_js_model(model).expect("the model reads");
    assert_eq!(entries[0].kind, ApiKind::Other);
}

#[test]
fn a_bad_model_is_an_error_naming_its_place() {
    let member = |fields: &str| {
        format!(
            r#"{{"kind": "Package", "name": "p", "members": [{{"kind": "EntryPoint", "name": "", "members": [
                {{"kind": "Class", "name": "A"}}, {{"kind": "Class", "name": "B"}},
                {{"kind": "Class", "name": "C"}}, {{{fields}}}]}}]}}"#
        )
    };
    let deep = format!(
        r#"{{"kind": "Package", "name": "p", "members": {}{}}}"#,
        "[".repeat(200),
        "]".repeat(200)
    );
    let cases = [
        ("not json", "no JSON".to_string()),
        (r#"{"kind": "Class", "name": "p"}"#, "Package".to_string()),
        (r#"{"kind": "Package"}"#, "name".to_string()),
        (
            &member(r#""name": "D""#),
            "members[0].members[3]".to_string(),
        ),
        (
            &member(r#""kind": "Class""#),
            "members[0].members[3]".to_string(),
        ),
        (&deep, "no JSON".to_string()),
    ];
    for (model, named) in cases {
        let error = read_js_model(model).expect_err(model);
        assert!(error.contains(&named), "{model}: {error}");
    }
    let kindless = read_js_model(&member(r#""name": "D""#)).unwrap_err();
    assert!(kindless.contains("kind"), "{kindless}");
    let nameless = read_js_model(&member(r#""kind": "Class""#)).unwrap_err();
    assert!(nameless.contains("name"), "{nameless}");
}

#[test]
fn a_bad_rust_line_is_an_error_naming_it_and_an_unknown_keyword_reads_other() {
    let error = read_rust_text("pub mod demo\ngarbage\n").unwrap_err();
    assert!(error.contains("line 2"), "{error}");
    for bad in [
        "pub",
        "pub fn",
        "pub demo: u8",
        "#[unclosed pub fn demo::f",
        "impl",
    ] {
        assert!(read_rust_text(bad).is_err(), "{bad}");
    }
    let entries = read_rust_text("pub frobnicate a::b").expect("reads");
    assert_eq!(entries[0].kind, ApiKind::Other);
    assert_eq!(entries[0].path, ["a", "b"]);
}

#[test]
fn a_cargo_public_api_snapshot_mints_api_items_citing_their_items() {
    let entries = read_rust_text(RUST).expect("the snapshot reads");
    insta::assert_json_snapshot!("rust", api_facts(&entries, &served()));
}

#[test]
fn each_line_reads_its_kind_owner_and_visibility() {
    let entries = read_rust_text(RUST).expect("the snapshot reads");
    let kinds: Vec<(String, &str)> = entries
        .iter()
        .map(|e| (e.path.join("::"), e.kind.as_str()))
        .collect();
    let expected = [
        ("example_api", "module"),
        ("example_api::Struct", "struct"),
        ("example_api::Struct::v1_field", "field"),
        ("example_api::Struct::new", "method"),
        ("example_api::Struct::fmt", "method"),
        ("example_api::Struct::fmt", "method"),
        ("example_api::serde", "extern_crate"),
        ("example_api::StructV2", "reexport"),
        ("example_api::enums", "module"),
        ("example_api::enums::DiverseVariants", "enum"),
        ("example_api::enums::DiverseVariants::Tuple", "variant"),
        ("example_api::enums::DiverseVariants::Unit", "variant"),
        ("example_api::enums::EnumWithExplicitDiscriminants", "enum"),
        (
            "example_api::enums::EnumWithExplicitDiscriminants::First",
            "variant",
        ),
        ("example_api::constants", "module"),
        ("example_api::constants::CONST_BOOL", "const"),
        ("example_api::attributes", "module"),
        ("example_api::attributes::C", "struct"),
        ("example_api::attributes::C::b", "field"),
        ("example_api::function", "function"),
        ("example_api::from_macro", "function"),
        ("example_api::restricted", "function"),
    ];
    let expected: Vec<(String, &str)> = expected
        .iter()
        .map(|(path, kind)| (path.to_string(), *kind))
        .collect();
    assert_eq!(kinds, expected);
    let display = &entries[5];
    let owner = display.owner.as_ref().expect("a trait impl member");
    assert_eq!(
        (owner.self_ty.as_str(), owner.trait_ty.as_deref()),
        ("example_api::Struct", Some("core::fmt::Display"))
    );
    assert!(entries[3]
        .owner
        .as_ref()
        .is_some_and(|o| o.trait_ty.is_none()));
    assert_eq!(
        entries[1].signature,
        "#[non_exhaustive] pub struct example_api::Struct"
    );
    assert_eq!(entries[21].visibility, Visibility::Crate);
    assert!(entries[..21]
        .iter()
        .all(|e| e.visibility == Visibility::Public));
    assert!(entries.iter().all(|e| e.container == "example_api"));
}

#[test]
fn generics_qualifiers_and_a_where_clause_read_through() {
    let text = "\
pub struct demo::Gen<T>
impl<T: core::fmt::Debug> core::fmt::Debug for demo::Gen<T> where T: Clone
pub fn demo::Gen<T>::fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
pub unsafe extern \"C\" fn demo::raw(p: *const u8)
pub unsafe trait demo::Marker
pub static mut demo::COUNTER: usize
pub macro demo::shout!
";
    let entries = read_rust_text(text).expect("the snapshot reads");
    let fmt = &entries[1];
    assert_eq!(fmt.kind, ApiKind::Item(ItemKind::Method));
    assert_eq!(fmt.path, ["demo", "Gen<T>", "fmt"]);
    assert_eq!(
        fmt.owner.as_ref().map(|o| o.trait_ty.as_deref()),
        Some(Some("core::fmt::Debug"))
    );
    let facts = api_facts(&entries[1..2], &[]);
    assert_eq!(facts[0].id, "api:rust:demo::<Gen as Debug>::fmt");
    let rest: Vec<(&str, &str)> = entries[2..]
        .iter()
        .map(|e| (e.path[1].as_str(), e.kind.as_str()))
        .collect();
    assert_eq!(
        rest,
        [
            ("raw", "function"),
            ("Marker", "trait"),
            ("COUNTER", "static"),
            ("shout", "macro")
        ]
    );
}

#[test]
fn a_snapshot_path_cannot_forge_a_repeat_suffix_nor_carry_an_escape_into_an_id() {
    let text =
        "pub fn c::a~2()\npub fn c::a(x: u8)\npub fn c::a(y: u16)\npub fn c::b\u{1b}[31m()\n";
    let entries = read_rust_text(text).expect("the snapshot reads");
    let facts = api_facts(&entries, &[]);
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "api:rust:c::a",
            "api:rust:c::a%7E2",
            "api:rust:c::a~2",
            "api:rust:c::b%1B[31m"
        ],
        "four entries, four facts: none overwrites another"
    );
}

#[test]
fn crlf_and_blank_lines_read_like_lf() {
    let lf = read_rust_text("pub mod demo\n\npub fn demo::f()\n").expect("reads");
    let crlf = read_rust_text("pub mod demo\r\n\r\npub fn demo::f()\r\n").expect("reads");
    assert_eq!(lf, crlf);
    assert_eq!(lf.len(), 2);
    assert!(read_rust_text("").expect("reads").is_empty());
}
