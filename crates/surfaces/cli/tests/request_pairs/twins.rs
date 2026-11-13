//! The pairing table's rows for the command-line twins of MCP reads (ADR 0355): `search` and `specforge.search`.
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
