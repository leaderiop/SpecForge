//! The pairing table's rows for the command-line twins of MCP reads (ADR 0355): `search` and `specforge.search`,
//! `refs` and `specforge.find_references`, `trace --in` and `specforge.trace`'s `in`.
//! One table with the rows of `request_pairs` (ADR 0101 D5): [`super::pair`] and the unique-label test read both.

use serde_json::json;

use super::{Outcome, Pair, pair, rv1};

pub(super) const TWIN_PAIRS: &[Pair] = &[
    Pair {
        label: "search_entities",
        project: rv1,
        cli: &["search", "alpha", "--path", "{root}", "--format", "json"],
        exit: 0,
        tool: "specforge.search",
        arguments: || json!({"query": "alpha"}),
    },
    Pair {
        label: "search_field_without_value",
        project: rv1,
        cli: &[
            "search", "alpha", "--field", "contract", "--path", "{root}", "--format", "json",
        ],
        exit: 2,
        tool: "specforge.search",
        arguments: || json!({"query": "alpha", "field": "contract"}),
    },
    Pair {
        label: "refs_incoming",
        project: rv1,
        cli: &["refs", "Red", "--path", "{root}", "--format", "json"],
        exit: 0,
        tool: "specforge.find_references",
        arguments: || json!({"entity_id": "Red"}),
    },
    Pair {
        label: "refs_both_with_declaration",
        project: rv1,
        cli: &[
            "refs",
            "Red",
            "--direction",
            "both",
            "--include-declaration",
            "--path",
            "{root}",
            "--format",
            "json",
        ],
        exit: 0,
        tool: "specforge.find_references",
        arguments: || json!({"entity_id": "Red", "direction": "both", "include_declaration": true}),
    },
    Pair {
        label: "trace_in_entity",
        project: rv1,
        cli: &[
            "trace", "--in", "alpha", "--path", "{root}", "--format", "json",
        ],
        exit: 0,
        tool: "specforge.trace",
        arguments: || json!({"in": "alpha"}),
    },
    Pair {
        label: "trace_entity_and_in",
        project: rv1,
        cli: &[
            "trace", "alpha", "--in", "Red", "--path", "{root}", "--format", "json",
        ],
        exit: 2,
        tool: "specforge.trace",
        arguments: || json!({"entity_id": "alpha", "in": "Red"}),
    },
];

#[test]
fn search_is_one_request() {
    let (cli, _) = pair("search_entities");
    assert!(matches!(cli, Outcome::Answered(_)), "{cli:?}");
}

#[test]
fn search_field_without_value_is_refused_on_both() {
    assert_eq!(pair("search_field_without_value").0, Outcome::Refused);
}

#[test]
fn refs_is_one_request() {
    let (cli, _) = pair("refs_incoming");
    assert!(matches!(cli, Outcome::Answered(_)), "{cli:?}");
}

#[test]
fn refs_with_direction_and_declaration_is_one_request() {
    match pair("refs_both_with_declaration").0 {
        Outcome::Answered(document) => {
            let roles: Vec<&str> = document["locations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|location| location["role"].as_str().unwrap())
                .collect();
            assert!(roles.contains(&"declaration"), "{document}");
        }
        Outcome::Refused => panic!("both surfaces refused"),
    }
}

#[specforge_test_macros::test(
    behavior = "trace_within_entity",
    verify = "specforge trace --in and specforge.trace's in answer the same document"
)]
fn trace_in_is_one_request() {
    match pair("trace_in_entity").0 {
        Outcome::Answered(document) => {
            let first = &document["traces"][0]["entity_id"];
            assert_eq!(first, "alpha", "{document}");
        }
        Outcome::Refused => panic!("both surfaces refused"),
    }
}

#[specforge_test_macros::test(
    behavior = "trace_within_entity",
    verify = "specforge trace and specforge.trace both refuse an entity with in"
)]
fn trace_entity_and_in_is_one_request() {
    assert_eq!(pair("trace_entity_and_in").0, Outcome::Refused);
}
