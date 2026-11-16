//! The candidate vocabulary a build is handed (ADR 0122) changes the suggestions of E024, W151 and
//! W020 and nothing else. It lists every extension that declares a kind keyword (ADR 0369), and
//! E024 and W151 suggest each of them. Every test starts from `.spec` text (ADR 0115).

use specforge_common::Diagnostic;
use specforge_registry::CandidateVocabulary;
use specforge_test_macros::test as spec;

use crate::support::written::checked;
use crate::support::{build, build_with, candidates, coded_in, declare, formal, product, software};

const TEXT: &str =
    "feature f1 \"F\" {\n}\n\ninvariant i1 \"I\" {\n  expression \"x\"\n}\n\nxyzzy x1 \"X\" {\n}\n";

/// `diagnostics` without their suggestions.
fn without_suggestions(diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .cloned()
        .map(|mut d| {
            d.suggestion = None;
            d
        })
        .collect()
}

#[spec(
    behavior = "suggest_missing_extensions",
    verify = "the candidate vocabulary changes only suggestions, never which diagnostics a check reports"
)]
fn the_candidate_vocabulary_changes_only_suggestions() {
    let bare = checked(&build([software()]), &[("t.spec", TEXT)]);
    let handed = build_with([software()], candidates(&[product(), formal()]));
    let with = checked(&handed, &[("t.spec", TEXT)]);

    // The same codes, messages, spans and order.
    assert_eq!(without_suggestions(&bare), without_suggestions(&with));
    assert_eq!(coded_in(&with, "E024").len(), 2, "{with:?}");
    assert_eq!(coded_in(&with, "W020").len(), 1, "{with:?}");

    // Only the suggestions of the two names the vocabulary knows differ.
    let differing: Vec<&str> = bare
        .iter()
        .zip(&with)
        .filter(|(a, b)| a.suggestion != b.suggestion)
        .map(|(a, _)| a.message.as_str())
        .collect();
    assert_eq!(differing.len(), 2, "{differing:?}");
    assert!(
        differing.iter().any(|m| m.contains("'feature'")),
        "{differing:?}"
    );
    assert!(
        differing.iter().any(|m| m.contains("'expression'")),
        "{differing:?}"
    );

    // W151, over a build that declares no kind.
    let kindless = || declare("@test/kindless", |_| {});
    let text = "behavior b1 \"B\" {\n}\n";
    let bare = checked(&build([kindless()]), &[("t.spec", text)]);
    let with = checked(
        &build_with([kindless()], candidates(&[software()])),
        &[("t.spec", text)],
    );
    assert_eq!(coded_in(&with, "W151").len(), 1, "{with:?}");
    assert_eq!(without_suggestions(&bare), without_suggestions(&with));
    assert_ne!(bare[0].suggestion, with[0].suggestion);
}

#[test]
fn a_build_handed_no_vocabulary_suggests_search() {
    let diags = checked(&build([software()]), &[("t.spec", TEXT)]);
    let e024 = coded_in(&diags, "E024");
    assert!(
        e024[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("specforge extensions search feature"),
        "{diags:?}"
    );
    let w020 = coded_in(&diags, "W020");
    assert_eq!(w020[0].suggestion, None, "{diags:?}");
}

/// Two extensions declare the keyword `term`: one vocabulary, written with both under `term`, and
/// E024 for a `term` no loaded extension declares suggests each of them (ADR 0369 D1, D2).
#[test]
fn two_extensions_declaring_term_yield_one_vocabulary_and_e024_names_both() {
    let term = |name: &str| {
        declare(name, |c| {
            c.kind("Term", |k| {
                k.keyword("term");
            });
        })
    };
    let vocabulary = CandidateVocabulary::of(&[term("@specforge/product"), term("@gsd/core")])
        .expect("a keyword two extensions declare is no error");

    let text = vocabulary.to_json();
    let written: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        written["kinds"]["term"],
        serde_json::json!(["@gsd/core", "@specforge/product"]),
        "{text}"
    );
    let read_back = CandidateVocabulary::from_json(&text);
    assert_eq!(read_back, vocabulary);

    let diags = checked(
        &build_with([software()], read_back),
        &[("t.spec", "term t1 \"T\" {\n}\n")],
    );
    let e024 = coded_in(&diags, "E024");
    assert_eq!(e024.len(), 1, "{diags:?}");
    assert_eq!(
        e024[0].suggestion.as_deref(),
        Some(
            "install one of the extensions that declare it with: specforge add @gsd/core or specforge add @specforge/product"
        ),
        "{diags:?}"
    );
}

/// `@specforge/software` and `@test/behaviors` both declare the keyword `behavior`: W151, over a
/// build that declares no kind, suggests each of them (ADR 0369 D2).
#[test]
fn w151_names_every_extension_that_declares_the_first_kind() {
    let behaviors = declare("@test/behaviors", |c| {
        c.kind("Behavior", |k| {
            k.keyword("behavior");
        });
    });
    let kindless = declare("@test/kindless", |_| {});
    let diags = checked(
        &build_with([kindless], candidates(&[software(), behaviors])),
        &[("t.spec", "behavior b1 \"B\" {\n}\n")],
    );
    let w151 = coded_in(&diags, "W151");
    assert_eq!(w151.len(), 1, "{diags:?}");
    // An answer that takes the first declarer would name `@specforge/software` alone.
    assert_eq!(
        w151[0].suggestion.as_deref(),
        Some(
            "install one of the extensions that declare it with: specforge add @specforge/software or specforge add @test/behaviors"
        ),
        "{diags:?}"
    );
}
