use specforge_code_facts::{api_facts, ApiKind, ItemKind, Visibility};
use specforge_extension_sdk::prelude::Fact;

use super::*;

/// A `cargo public-api` snapshot, hand-written to the tool's own snapshot lines: crate `example_api`.
const RUST: &str = include_str!("../fixtures/rust.public-api.txt");
/// The served `item` facts an api collector receives, as JSON.
const ITEMS: &str = include_str!("../fixtures/items.json");

fn served() -> Vec<Fact> {
    serde_json::from_str(ITEMS).expect("the served facts parse")
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
fn crlf_and_blank_lines_read_like_lf() {
    let lf = read_rust_text("pub mod demo\n\npub fn demo::f()\n").expect("reads");
    let crlf = read_rust_text("pub mod demo\r\n\r\npub fn demo::f()\r\n").expect("reads");
    assert_eq!(lf, crlf);
    assert_eq!(lf.len(), 2);
    assert!(read_rust_text("").expect("reads").is_empty());
}
