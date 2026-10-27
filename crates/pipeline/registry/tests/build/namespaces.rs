//! A rule's code is its extension's namespace and its own local code (K10).

use specforge_extension_sdk::prelude::*;
use specforge_test_macros::test as spec;

use crate::support::{build, coded, declare, rule_codes};

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
