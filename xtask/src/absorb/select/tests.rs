//! The selector on a fixture project compiled in-process with its builtins: what joins, through which rule, and what
//! stays out.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use super::*;
use crate::read;

/// The fixture: a seed feature and invariant, a through behavior, referrers of each kind in and out, two personas
/// and terms the prose names or not.
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/absorb-sample");

/// The fixture's rule: its seed, through, referrer and whole files.
const FIXTURE_SAMPLE: Sample = Sample {
    name: "fixture",
    seeds: &["spec/features/w.spec", "spec/invariants/w.spec"],
    through: &["spec/behaviors/w-*.spec"],
    referrers: &["failure_mode", "constraint", "decision", "journey"],
    whole: &["spec/product/personas.spec"],
};

fn selected() -> Selection {
    let old = read::compile(std::path::Path::new(FIXTURE)).expect("the fixture compiles");
    select(&old, &FIXTURE_SAMPLE)
}

/// The ids selected of `kind`.
fn ids<'s>(selection: &'s Selection, kind: &str) -> Vec<&'s str> {
    selection
        .0
        .get(kind)
        .map(|ids| ids.iter().map(String::as_str).collect())
        .unwrap_or_default()
}

#[test]
fn a_seed_file_joins_whole_and_draws_the_referrers_of_its_entities() {
    let s = selected();
    assert_eq!(ids(&s, "feature"), ["w_host"]);
    assert_eq!(ids(&s, "invariant"), ["w_isolated"]);
    assert_eq!(
        ids(&s, "failure_mode"),
        ["w_escape"],
        "the seed invariant's"
    );
    assert_eq!(ids(&s, "decision"), ["w_component"], "the seed feature's");
    assert_eq!(ids(&s, "journey"), ["install_one"], "the seed feature's");
}

#[test]
fn a_referrer_of_a_through_entity_joins_but_the_through_entity_does_not() {
    let s = selected();
    assert_eq!(ids(&s, "constraint"), ["w_fast"]);
    assert!(ids(&s, "behavior").is_empty(), "{s:?}");
}

#[test]
fn only_the_referrer_kinds_join_and_only_one_step_out() {
    let s = selected();
    let all: Vec<&str> = s.0.values().flatten().map(String::as_str).collect();
    // Referrers of entities outside the sample, an entity two steps out, and kinds that are no referrer.
    for out in [
        "other_fast",
        "other_torn",
        "old_component",
        "wander",
        "other_run",
        "other_whole",
        "w_run",
    ] {
        assert!(!all.contains(&out), "{out}: {s:?}");
    }
    let kinds: Vec<&str> = s.0.keys().map(String::as_str).collect();
    assert_eq!(
        kinds,
        [
            "constraint",
            "decision",
            "failure_mode",
            "feature",
            "invariant",
            "journey",
            "persona",
            "term"
        ]
    );
}

#[test]
fn a_whole_file_joins_and_draws_no_referrers() {
    let s = selected();
    assert_eq!(ids(&s, "persona"), ["author", "operator"]);
    assert!(
        !ids(&s, "journey").contains(&"wander"),
        "a journey of a whole file's persona is no referrer"
    );
}

#[test]
fn a_term_joins_when_sampled_prose_names_its_title_or_an_alias() {
    let s = selected();
    let terms = ids(&s, "term");
    // Plugin by title; Box by its alias "sandbox"; Élève in another case; a phrase across its one space; Task by its
    // title in a journey's flow, its id `work_item` written nowhere.
    for joined in [
        "t_plugin",
        "t_box",
        "t_eleve",
        "t_entity_graph",
        "work_item",
    ] {
        assert!(terms.contains(&joined), "{joined}: {terms:?}");
    }
    // `component` is written in the prose, but as a word, never as the term's title; "caf" only inside "café".
    for out in ["component", "t_caf"] {
        assert!(!terms.contains(&out), "{out}: {terms:?}");
    }
    let names = |text: &str, name: &str| pattern(name).unwrap().is_match(text);
    assert!(names("an élève here", "Élève"));
    assert!(!names("the café", "caf"));
    assert!(names("the entity graph, built", "Entity Graph"));
    assert!(
        names("the entity\n    graph", "entity graph"),
        "a wrapped line"
    );
    assert!(!names("the entitygraph", "entity graph"));
}

#[test]
fn a_term_is_named_within_one_text_never_across_two() {
    // "Release" ends the feature's problem and "Train" starts its description: no single text names the term.
    let s = selected();
    assert!(!ids(&s, "term").contains(&"t_release_train"), "{s:?}");
    let texts = ["the whole host on a Release", "Train aside"].map(String::from);
    let release_train = pattern("Release Train").unwrap();
    assert!(!texts.iter().any(|text| release_train.is_match(text)));
    assert!(
        release_train.is_match("a Release\n  Train"),
        "a wrap within one text"
    );
    assert!(pattern("  ").is_none(), "a name of no word names nothing");
}

#[test]
fn a_term_named_only_by_another_term_stays_out() {
    let s = selected();
    assert!(!ids(&s, "term").contains(&"t_chain"), "{s:?}");
}

#[test]
fn an_unknown_sample_is_refused_with_the_sample_names() {
    let refused = sample("nope").unwrap_err();
    assert!(refused.contains("no sample 'nope'"), "{refused}");
    assert!(refused.contains("wasm-host"), "{refused}");
    assert_eq!(sample("wasm-host").unwrap().name, "wasm-host");
    let args = |extra: &[&str]| {
        let mut args = vec!["--from", "a", "--to", "b"];
        args.extend_from_slice(extra);
        crate::parse(&args.iter().map(ToString::to_string).collect::<Vec<_>>())
    };
    assert!(
        args(&["--only", "sample:nope"])
            .unwrap_err()
            .contains("wasm-host")
    );
    assert!(
        args(&["--selection", "s.json"])
            .unwrap_err()
            .contains("needs --only sample:<name>")
    );
    let parsed = args(&["--only", "sample:wasm-host", "--selection", "s.json"]).unwrap();
    assert_eq!(parsed.sample.map(|s| s.name), Some("wasm-host"));
}

#[test]
fn the_selection_is_sorted_json_by_old_kind() {
    let s = selected();
    let json = s.to_json();
    assert!(json.ends_with("}\n"), "{json}");
    let read: BTreeMap<String, Vec<String>> = serde_json::from_str(&json).unwrap();
    for (kind, ids) in &read {
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, &sorted, "{kind}");
    }
    let count: usize = read.values().map(Vec::len).sum();
    assert_eq!(count, 13, "{json}");
    match s.scope() {
        Scope::Only(ids) => assert_eq!(ids.len(), 13),
        Scope::All => panic!("a sample is a selection"),
    }
}

#[test]
fn a_second_only_of_either_form_is_refused_naming_both() {
    let ids = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(ids.path(), "[]").unwrap();
    let ids = format!("ids:{}", ids.path().display());
    for (first, second) in [
        ("sample:wasm-host", "sample:wasm-host"),
        ("sample:wasm-host", ids.as_str()),
        (ids.as_str(), "sample:wasm-host"),
        (ids.as_str(), ids.as_str()),
    ] {
        let args = [
            "--from", "a", "--to", "b", "--only", first, "--only", second,
        ];
        let refused = crate::parse(&args.map(String::from)).unwrap_err();
        assert!(
            refused.starts_with("--only takes one value")
                && refused.contains(&format!("'{first}' and '{second}'")),
            "{refused}"
        );
    }
}
