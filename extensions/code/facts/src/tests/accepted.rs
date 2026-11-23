//! An API snapshot's entries judged against the committed accepted report (ADR 0390 D3): pairing, narrowing,
//! one-sided containers and Rust lines.

use crate::*;
use specforge_extension_sdk::prelude::Fact;

const FUNCTION: ApiKind = ApiKind::Item(ItemKind::Function);

/// A JavaScript entry of package `@demo/w` at `path` (`.`-joined), written `signature`, tagged `tag`.
fn js(path: &str, signature: &str, tag: &str) -> ApiEntry {
    js_in("@demo/w", path, signature, tag)
}

fn js_in(package: &str, path: &str, signature: &str, tag: &str) -> ApiEntry {
    ApiEntry {
        scheme: Scheme::Js,
        container: package.to_string(),
        path: path.split('.').map(str::to_string).collect(),
        owner: None,
        kind: FUNCTION,
        visibility: Visibility::Public,
        signature: signature.to_string(),
        tag: Some(tag.to_string()),
    }
}

/// A Rust function `name` of crate `demo`, its whole line `line`, of `visibility`.
fn rust(name: &str, line: &str, visibility: Visibility) -> ApiEntry {
    ApiEntry {
        scheme: Scheme::Rust,
        container: "demo".to_string(),
        path: vec!["demo".to_string(), name.to_string()],
        owner: None,
        kind: FUNCTION,
        visibility,
        signature: line.to_string(),
        tag: None,
    }
}

fn field<'f>(fact: &'f Fact, name: &str) -> Option<&'f str> {
    fact.fields.get(name).and_then(|value| value.as_str())
}

/// Each fact's id, `change`, `signature` and `accepted_signature`.
fn judged(facts: &[Fact]) -> Vec<(&str, Option<&str>, &str, Option<&str>)> {
    facts
        .iter()
        .map(|f| {
            (
                f.id.as_str(),
                field(f, "change"),
                field(f, "signature").unwrap_or_default(),
                field(f, "accepted_signature"),
            )
        })
        .collect()
}

/// The one fact of `facts` whose signature is `signature`.
fn written<'f>(facts: &'f [Fact], signature: &str) -> &'f Fact {
    let found: Vec<&Fact> = facts
        .iter()
        .filter(|f| field(f, "signature") == Some(signature))
        .collect();
    assert_eq!(found.len(), 1, "{signature} in {facts:?}");
    found[0]
}

const F: &str = "api:js:%40demo%2Fw::f";

#[test]
fn the_comparison_key_bridges_the_report_and_the_doc_model() {
    let key = "function create(o: Options): Widget";
    assert_eq!(
        comparable("export declare function create(o: Options): Widget;"),
        key
    );
    assert_eq!(
        comparable("export  function create(o: Options):\n Widget;"),
        key
    );
    assert_eq!(comparable("LIMIT = 3"), comparable("const LIMIT = 3;"));
    assert_eq!(comparable("A = 1"), comparable("A = 1,"));
    assert_eq!(
        comparable("export declare class W "),
        comparable("export class W {")
    );
    assert_eq!(
        comparable("pub fn demo::f()"),
        comparable("pub(crate) fn demo::f()")
    );
    assert_eq!(
        comparable("#[non_exhaustive] pub struct demo::S"),
        "#[non_exhaustive] struct demo::S"
    );
    assert_ne!(
        comparable("pub fn demo::f(a: u8)"),
        comparable("pub fn demo::f(a: u16)")
    );
    assert_eq!(comparable("publish(): void;"), "publish(): void");
    assert_eq!(
        ApiChange::NAMES,
        ["added", "unchanged", "changed", "narrowed", "removed"]
    );
}

#[test]
fn an_overload_inserted_before_another_leaves_it_unchanged() {
    let accepted = [js("f", "export function f(a: number): void;", "Public")];
    let fresh = [
        js("f", "export declare function f(s: string): void;", "Public"),
        js("f", "export declare function f(a: number): void;", "Public"),
    ];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    let ids: Vec<String> = vec![F.to_string(), format!("{F}~2")];
    assert_eq!(
        facts.iter().map(|f| &f.id).collect::<Vec<_>>(),
        ids.iter().collect::<Vec<_>>()
    );
    assert_eq!(
        field(
            written(&facts, "export declare function f(s: string): void;"),
            "change"
        ),
        Some("added")
    );
    assert_eq!(
        field(
            written(&facts, "export declare function f(a: number): void;"),
            "change"
        ),
        Some("unchanged")
    );
}

#[test]
fn two_changed_overloads_pair_in_document_order() {
    let accepted = [
        js("f", "export function f(a: number): void;", "Public"),
        js("f", "export function f(a: string): void;", "Public"),
    ];
    let fresh = [
        js("f", "export declare function f(a: bigint): void;", "Public"),
        js(
            "f",
            "export declare function f(a: boolean): void;",
            "Public",
        ),
    ];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    assert_eq!(
        judged(&facts),
        [
            (
                F,
                Some("changed"),
                "export declare function f(a: bigint): void;",
                Some("export function f(a: number): void;")
            ),
            (
                &*format!("{F}~2"),
                Some("changed"),
                "export declare function f(a: boolean): void;",
                Some("export function f(a: string): void;")
            ),
        ]
    );
}

#[test]
fn a_removed_entry_takes_the_next_repeat_suffix_after_the_runs() {
    let accepted = [
        js("f", "export function f(a: number): void;", "Public"),
        js("f", "export function f(a: string): void;", "Public"),
    ];
    let fresh = [js(
        "f",
        "export declare function f(a: number): void;",
        "Public",
    )];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    assert_eq!(
        judged(&facts),
        [
            (
                F,
                Some("unchanged"),
                "export declare function f(a: number): void;",
                None
            ),
            (
                &*format!("{F}~2"),
                Some("removed"),
                "export function f(a: string): void;",
                Some("export function f(a: string): void;")
            ),
        ]
    );
    assert_eq!(field(&facts[1], "accepted_tag"), Some("Public"));
    assert!(!facts[1].fields.contains_key("item"));
}

#[test]
fn a_container_on_one_side_only_is_not_judged() {
    let fresh = [
        js_in("a", "f", "export declare function f(): void;", "Public"),
        js_in("b", "g", "export declare function g(): void;", "Public"),
    ];
    let accepted = [
        js_in("a", "f", "export function f(): void;", "Public"),
        js_in("c", "h", "export function h(): void;", "Public"),
    ];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    assert_eq!(
        judged(&facts),
        [
            (
                "api:js:a::f",
                Some("unchanged"),
                "export declare function f(): void;",
                None
            ),
            (
                "api:js:b::g",
                None,
                "export declare function g(): void;",
                None
            ),
        ]
    );
    assert_eq!(facts[1], api_facts(&fresh[1..], &[])[0]);
}

#[test]
fn an_accepted_internal_entry_with_no_counterpart_mints_nothing() {
    let accepted = [
        js("f", "export function f(): void;", "Public"),
        js("_hidden", "export function _hidden(): void;", "Internal"),
    ];
    let fresh = [js("f", "export declare function f(): void;", "Public")];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    assert_eq!(facts.len(), 1, "{facts:?}");
    assert_eq!(field(&facts[0], "change"), Some("unchanged"));
}

#[test]
fn a_wider_tag_is_unchanged() {
    let accepted = [js("f", "export function f(): void;", "Beta")];
    let fresh = [js("f", "export declare function f(): void;", "Public")];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    assert_eq!(field(&facts[0], "change"), Some("unchanged"));
    assert!(!facts[0].fields.contains_key("accepted_tag"));
}

#[test]
fn a_narrower_tag_with_another_signature_is_changed_naming_the_accepted_tag() {
    let accepted = [js("f", "export function f(): void;", "Public")];
    let fresh = [js(
        "f",
        "export declare function f(a: number): void;",
        "Beta",
    )];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    assert_eq!(field(&facts[0], "change"), Some("changed"));
    assert_eq!(field(&facts[0], "accepted_tag"), Some("Public"));
    assert_eq!(field(&facts[0], "tag"), Some("Beta"));
}

#[test]
fn a_class_narrowed_to_beta_narrows_each_member_inheriting_its_tag() {
    let accepted = [
        js("W", "export class W {", "Public"),
        js("W.render", "render(): string;", "Public"),
        js("W.size", "size: number;", "Public"),
        js("W.beta", "beta(): void;", "Beta"),
    ];
    let fresh = [
        js("W", "export declare class W ", "Beta"),
        js("W.render", "render(): string;", "Beta"),
        js("W.size", "size: number;", "Beta"),
        js("W.beta", "beta(): void;", "Beta"),
    ];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    let changes: Vec<(&str, Option<&str>, Option<&str>)> = facts
        .iter()
        .map(|f| (f.id.as_str(), field(f, "change"), field(f, "accepted_tag")))
        .collect();
    assert_eq!(
        changes,
        [
            ("api:js:%40demo%2Fw::W", Some("narrowed"), Some("Public")),
            ("api:js:%40demo%2Fw::W.beta", Some("unchanged"), None),
            (
                "api:js:%40demo%2Fw::W.render",
                Some("narrowed"),
                Some("Public")
            ),
            (
                "api:js:%40demo%2Fw::W.size",
                Some("narrowed"),
                Some("Public")
            ),
        ]
    );
}

#[test]
fn a_rename_is_one_removed_and_one_added() {
    let accepted = [js("f", "export function f(): void;", "Public")];
    let fresh = [js("g", "export declare function g(): void;", "Public")];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    let changes: Vec<(&str, Option<&str>)> = facts
        .iter()
        .map(|f| (f.id.as_str(), field(f, "change")))
        .collect();
    assert_eq!(
        changes,
        [
            (F, Some("removed")),
            ("api:js:%40demo%2Fw::g", Some("added"))
        ]
    );
}

#[test]
fn a_rust_line_on_both_sides_is_unchanged_and_another_parameter_type_is_changed() {
    let accepted = [
        rust("same", "pub fn demo::same()", Visibility::Public),
        rust("f", "pub fn demo::f(a: u8)", Visibility::Public),
    ];
    let fresh = [
        rust("same", "pub fn demo::same()", Visibility::Public),
        rust("f", "pub fn demo::f(a: u16)", Visibility::Public),
    ];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    assert_eq!(
        judged(&facts),
        [
            (
                "api:rust:demo::f",
                Some("changed"),
                "pub fn demo::f(a: u16)",
                Some("pub fn demo::f(a: u8)")
            ),
            (
                "api:rust:demo::same",
                Some("unchanged"),
                "pub fn demo::same()",
                None
            ),
        ]
    );
    assert!(facts
        .iter()
        .all(|f| !f.fields.contains_key("accepted_tag")
            && !f.fields.contains_key("accepted_visibility")));
}

#[test]
fn a_rust_line_gone_is_removed() {
    let accepted = [
        rust("kept", "pub fn demo::kept()", Visibility::Public),
        rust("gone", "pub fn demo::gone()", Visibility::Public),
    ];
    let fresh = [rust("kept", "pub fn demo::kept()", Visibility::Public)];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    assert_eq!(
        judged(&facts)[0],
        (
            "api:rust:demo::gone",
            Some("removed"),
            "pub fn demo::gone()",
            Some("pub fn demo::gone()")
        )
    );
    assert_eq!(field(&facts[0], "visibility"), Some("public"));
}

#[test]
fn a_rust_item_made_pub_crate_is_narrowed_naming_the_accepted_visibility() {
    let accepted = [rust("f", "pub fn demo::f()", Visibility::Public)];
    let fresh = [rust("f", "pub(crate) fn demo::f()", Visibility::Crate)];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    assert_eq!(field(&facts[0], "change"), Some("narrowed"));
    assert_eq!(field(&facts[0], "accepted_visibility"), Some("public"));
    assert_eq!(
        field(&facts[0], "accepted_signature"),
        Some("pub fn demo::f()")
    );
    // The other way is wider: unchanged.
    let wider = api_facts_against(&accepted, &fresh, &[]);
    assert_eq!(field(&wider[0], "change"), Some("unchanged"));
}

#[test]
fn a_rust_attribute_added_reads_changed_whatever_it_means() {
    let accepted = [rust("S", "pub struct demo::S", Visibility::Public)];
    let fresh = [rust(
        "S",
        "#[non_exhaustive] pub struct demo::S",
        Visibility::Public,
    )];
    let facts = api_facts_against(&fresh, &accepted, &[]);
    assert_eq!(field(&facts[0], "change"), Some("changed"));
}

#[test]
fn a_forged_accepted_name_is_minted_encoded_and_collides_with_no_fresh_id() {
    let fresh = [
        js("f", "export declare function f(a: number): void;", "Public"),
        js("f", "export declare function f(a: string): void;", "Public"),
    ];
    let mut accepted = vec![
        js("f", "export function f(a: number): void;", "Public"),
        js("f", "export function f(a: string): void;", "Public"),
    ];
    for forged in ["f~2", "f#x", "f@y"] {
        accepted.push(js(forged, "export function forged(): void;", "Public"));
    }
    let facts = api_facts_against(&fresh, &accepted, &[]);
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            F,
            "api:js:%40demo%2Fw::f%23x",
            "api:js:%40demo%2Fw::f%40y",
            "api:js:%40demo%2Fw::f%7E2",
            "api:js:%40demo%2Fw::f~2",
        ],
        "five entries, five facts: none overwrites another"
    );
    assert_eq!(field(&facts[4], "change"), Some("unchanged"));
}
