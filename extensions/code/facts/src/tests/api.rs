//! The `api_item` facts minted from an API snapshot's entries and the served items.

use crate::*;
use specforge_extension_sdk::prelude::Fact;

fn item(id: &str) -> Fact {
    Fact::new("item", id)
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// A public Rust entry of `kind` at `path` (the crate first), written `signature`.
fn rust(path: &[&str], owner: Option<Owner>, kind: ApiKind, signature: &str) -> ApiEntry {
    ApiEntry {
        scheme: Scheme::Rust,
        container: path[0].to_string(),
        path: strings(path),
        owner,
        kind,
        visibility: Visibility::Public,
        signature: signature.to_string(),
        tag: None,
    }
}

fn owner(self_ty: &str, trait_ty: Option<&str>) -> Option<Owner> {
    Some(Owner {
        self_ty: self_ty.to_string(),
        trait_ty: trait_ty.map(str::to_string),
    })
}

fn field<'f>(fact: &'f Fact, name: &str) -> Option<&'f str> {
    fact.fields.get(name).and_then(|value| value.as_str())
}

const FUNCTION: ApiKind = ApiKind::Item(ItemKind::Function);
const METHOD: ApiKind = ApiKind::Item(ItemKind::Method);

#[test]
fn the_kind_vocabulary_is_the_item_kinds_then_four_more() {
    assert_eq!(&ApiKind::NAMES[..16], &ItemKind::NAMES[..]);
    assert_eq!(
        &ApiKind::NAMES[16..],
        &["variant", "reexport", "extern_crate", "other"]
    );
    assert_eq!(ApiKind::ExternCrate.as_str(), "extern_crate");
    assert_eq!(METHOD.as_str(), "method");
    assert!(METHOD.is_item() && !ApiKind::Reexport.is_item());
}

#[test]
fn a_snapshot_crate_maps_to_the_served_lib_reading_dash_as_underscore() {
    let entries = [rust(
        &["example_api", "function"],
        None,
        FUNCTION,
        "pub fn example_api::function()",
    )];
    let served = [
        item("rust:example-api::function"),
        item("rust:example-api/bin/tool::function"),
    ];
    let facts = api_facts(&entries, &served);
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].id, "api:rust:example_api::function");
    assert_eq!(field(&facts[0], "item"), Some("rust:example-api::function"));
    assert_eq!(field(&facts[0], "path"), Some("example_api::function"));
    assert_eq!(field(&facts[0], "kind"), Some("function"));
    assert_eq!(field(&facts[0], "visibility"), Some("public"));
}

#[test]
fn two_trait_impls_of_one_type_are_two_facts_each_citing_its_item() {
    let debug = "pub fn example_api::Struct::fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result";
    let entries = [
        rust(
            &["example_api", "Struct", "fmt"],
            owner("example_api::Struct", Some("core::fmt::Debug")),
            METHOD,
            debug,
        ),
        rust(
            &["example_api", "Struct", "fmt"],
            owner("example_api::Struct", Some("core::fmt::Display")),
            METHOD,
            debug,
        ),
    ];
    let served = [
        item("rust:example-api::<Struct as Debug>::fmt"),
        item("rust:example-api::<Struct as Display>::fmt"),
    ];
    let facts = api_facts(&entries, &served);
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "api:rust:example_api::<Struct as Debug>::fmt",
            "api:rust:example_api::<Struct as Display>::fmt"
        ]
    );
    assert_eq!(
        field(&facts[1], "item"),
        Some("rust:example-api::<Struct as Display>::fmt")
    );
}

#[test]
fn an_entry_repeated_merges_and_another_signature_takes_a_suffix() {
    let entries = [
        rust(&["demo", "f"], None, FUNCTION, "pub fn demo::f()"),
        rust(&["demo", "f"], None, FUNCTION, "pub fn demo::f()"),
        rust(&["demo", "f"], None, FUNCTION, "pub fn demo::f(x: u8)"),
    ];
    let facts = api_facts(&entries, &[item("rust:demo::f")]);
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["api:rust:demo::f", "api:rust:demo::f~2"]);
    assert_eq!(field(&facts[1], "signature"), Some("pub fn demo::f(x: u8)"));
}

#[test]
fn a_reexported_path_or_an_ambiguous_crate_cites_nothing() {
    // `demo::S` is defined in `demo::inner`: its public path names no served item.
    let reexported = rust(
        &["demo", "S"],
        None,
        ApiKind::Item(ItemKind::Struct),
        "pub struct demo::S",
    );
    let facts = api_facts(&[reexported], &[item("rust:demo::inner::S")]);
    assert_eq!(facts[0].id, "api:rust:demo::S");
    assert_eq!(field(&facts[0], "item"), None);
    // Two libs read as one crate: neither is cited.
    let ambiguous = rust(
        &["de_mo", "S"],
        None,
        ApiKind::Item(ItemKind::Struct),
        "pub struct de_mo::S",
    );
    let facts = api_facts(
        &[ambiguous],
        &[item("rust:de-mo::S"), item("rust:de_mo::S")],
    );
    assert_eq!(field(&facts[0], "item"), None);
}

#[test]
fn a_variant_reexport_or_crate_root_cites_no_item() {
    let entries = [
        rust(
            &["demo"],
            None,
            ApiKind::Item(ItemKind::Module),
            "pub mod demo",
        ),
        rust(
            &["demo", "E", "A"],
            None,
            ApiKind::Variant,
            "pub demo::E::A",
        ),
        rust(&["demo", "E"], None, ApiKind::Reexport, "pub use demo::E"),
    ];
    let served = [item("rust:demo::E::A"), item("rust:demo::E")];
    let facts = api_facts(&entries, &served);
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        ["api:rust:demo", "api:rust:demo::E", "api:rust:demo::E::A"]
    );
    assert!(
        facts.iter().all(|f| field(f, "item").is_none()),
        "{facts:?}"
    );
}

/// A public JavaScript entry of the package `@demo/widget` at the name path `path`, written `signature`.
fn js(path: &[&str], kind: ApiKind, signature: &str) -> ApiEntry {
    ApiEntry {
        scheme: Scheme::Js,
        container: "@demo/widget".to_string(),
        path: strings(path),
        owner: None,
        kind,
        visibility: Visibility::Public,
        signature: signature.to_string(),
        tag: Some("Public".to_string()),
    }
}

#[test]
fn a_js_entry_cites_the_one_served_item_of_its_package_with_its_name_path() {
    let entries = [
        js(&["Widget", "render"], METHOD, "render(): string;"),
        js(
            &["VERSION"],
            ApiKind::Item(ItemKind::Variable),
            "VERSION: string",
        ),
        js(
            &["util"],
            ApiKind::Item(ItemKind::Namespace),
            "namespace util",
        ),
    ];
    let served = [
        item("js:@demo/widget/src/widget.ts::Widget.render"),
        item("js:@demo/widget/src/index.ts::VERSION"),
        item("js:@demo/widget/src/version.ts::VERSION"),
        item("js:@demo/widget-extra/src/util.ts::util"),
    ];
    let facts = api_facts(&entries, &served);
    let cited: Vec<(&str, Option<&str>)> = facts
        .iter()
        .map(|f| (f.id.as_str(), field(f, "item")))
        .collect();
    assert_eq!(
        cited,
        [
            ("api:js:%40demo%2Fwidget::VERSION", None),
            (
                "api:js:%40demo%2Fwidget::Widget.render",
                Some("js:@demo/widget/src/widget.ts::Widget.render")
            ),
            ("api:js:%40demo%2Fwidget::util", None),
        ]
    );
    assert_eq!(field(&facts[1], "path"), Some("Widget.render"));
    assert_eq!(field(&facts[1], "tag"), Some("Public"));
}

#[test]
fn a_js_overload_takes_a_suffix_and_a_name_cannot_forge_one() {
    let entries = [
        js(&["parse"], FUNCTION, "parse(text: string): Widget;"),
        js(&["parse"], FUNCTION, "parse(bytes: Uint8Array): Widget;"),
        js(&["parse~2"], FUNCTION, "a name holding a tilde"),
    ];
    let facts = api_facts(&entries, &[]);
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "api:js:%40demo%2Fwidget::parse",
            "api:js:%40demo%2Fwidget::parse%7E2",
            "api:js:%40demo%2Fwidget::parse~2"
        ]
    );
}
