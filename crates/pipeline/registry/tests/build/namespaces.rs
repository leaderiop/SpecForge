//! A rule's code is its extension's namespace and its own local code (K10).

use specforge_extension_sdk::prelude::*;
use specforge_test_macros::test as spec;

use crate::support::{build, coded, codes, declare, rule_codes};

fn warns(name: &str, code: &str) -> specforge_protocol_types::ExtensionDeclaration {
    declare(name, |c| {
        c.rule(code, |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("orphan {kind} '{id}'")
                .check(CheckKind::NoIncomingEdges);
        });
    })
}

// Pins K10-D1: today two extensions declaring `E901` collide (W023); T5 un-ignores it.
#[spec(
    behavior = "registry_build_rules",
    verify = "one local code from two extensions is two codes"
)]
#[ignore = "K10 T5"]
fn one_local_code_from_two_extensions_is_two_codes() {
    let build = build([warns("@acme/a", "W901"), warns("@beta/b", "W901")]);
    assert!(coded(&build, "W023").is_empty());
    let codes: Vec<_> = rule_codes(&build)
        .into_iter()
        .map(|(code, _)| code)
        .collect();
    assert_eq!(codes, ["acme.a/W901", "beta.b/W901"]);
}

/// `name` cataloguing `E101`, titled and explained.
fn cataloguing(name: &str) -> specforge_protocol_types::ExtensionDeclaration {
    declare(name, |c| {
        c.code("E101", |d| {
            d.title("Requirement not mapped").explanation("Map it.");
        });
    })
}

// Pins K10-D1: today a shared namespace is not detected; T5 un-ignores it.
#[spec(
    behavior = "registry_build_rules",
    verify = "two loaded extensions that share a code namespace are E039"
)]
#[ignore = "K10 T5"]
fn two_extensions_sharing_a_namespace_are_e039() {
    let build = build([cataloguing("gsd"), cataloguing("@gsd/core")]);
    let shared = coded(&build, "E039");
    assert_eq!(shared.len(), 1, "{shared:?}");
    assert!(
        shared[0].message.contains("gsd") && shared[0].message.contains("@gsd/core"),
        "{}",
        shared[0].message
    );
}

// Pins K10-D2: today a malformed code entry is accepted; T5 un-ignores it.
#[test]
#[ignore = "K10 T5"]
fn a_malformed_code_entry_is_e030() {
    let entry = |c: &mut ContributionsBuilder, code: &str, title: &str, docs: Option<&str>| {
        c.code(code, |d| {
            d.title(title).explanation("Fix it.");
            if let Some(docs) = docs {
                d.docs(docs);
            }
        });
    };
    let bad = declare("@acme/lint", |c| {
        entry(c, "E1", "Short", None);
        entry(c, "gsd/E101", "Qualified", None);
        entry(c, "E102", "Twice", None);
        entry(c, "E102", "Twice", None);
        entry(c, "E103", "", None);
        entry(c, "E104", "Docs", Some("ftp:x"));
    });
    let first_party = declare("@specforge/product", |c| entry(c, "W900", "Not mine", None));
    let build = build([bad, first_party]);
    // One per bad entry: the code shape (2), the duplicate, the empty title, the docs, first party.
    assert_eq!(coded(&build, "E030").len(), 6, "{:?}", codes(&build));
}
