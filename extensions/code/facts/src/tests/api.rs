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
