//! The ids absorb converts this repository's own spec to, read as an old project as `cargo xtask absorb` reads it,
//! are GSD's and no keyword of the target: a kind `@gsd/core` declares, or a structural keyword, is an id the target
//! refuses (E013). This test reads the self-spec as an old project, so the swap removes it with the self-spec sample
//! (ADR 0367 D1).

use regex::Regex;
use serde_json::Value;
use std::collections::BTreeSet;

use super::super::plan;
use super::super::read;

/// The structural keywords of the core grammar: an id no project may take.
const STRUCTURAL: [&str; 4] = ["spec", "ref", "use", "define"];

#[test]
fn no_id_the_root_spec_converts_to_is_a_kind_keyword() {
    let golden: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../gsd/core/tests/golden/kinds.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut keywords: BTreeSet<String> = golden["entities"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| kind["name"].as_str().unwrap().to_string())
        .collect();
    assert!(
        keywords.contains("blocker"),
        "gsd declares a `blocker` kind"
    );
    keywords.extend(STRUCTURAL.map(String::from));

    let old =
        read::compile(specforge_repo_root::repo_root()).expect("the self-spec compiles clean");
    let entities = old.entities();
    plan::prechecks(&entities).expect("the self-spec converts");
    let planned = plan::plan(&entities);
    let converted: Vec<(&str, &str, String)> = planned
        .ids
        .entries()
        .map(|(kind, id, new)| (kind, id, new.name.clone()))
        .collect();
    assert!(
        converted.len() >= 300,
        "the self-spec's features, constraints, invariants, decisions, releases and milestones: {}",
        converted.len()
    );
    let grammar = Regex::new(r"^([A-Z][A-Z0-9]*-\d+|D-\d+|v[0-9][0-9A-Za-z.\-]*|\d{2,})$").unwrap();
    let ungrammatical: Vec<&(&str, &str, String)> = converted
        .iter()
        .filter(|(_, _, new)| !grammar.is_match(new))
        .collect();
    assert!(
        ungrammatical.is_empty(),
        "ids that match no GSD grammar: {ungrammatical:?}"
    );
    let clashing: Vec<&(&str, &str, String)> = converted
        .iter()
        .filter(|(_, _, new)| keywords.contains(new))
        .collect();
    assert!(
        clashing.is_empty(),
        "new ids the target refuses as a keyword: {clashing:?}"
    );
    assert!(
        entities
            .iter()
            .any(|e| e.kind == "term" && e.id == "blocker"),
        "the old term that stopped the first trial with E013"
    );
    assert!(
        !converted.iter().any(|(_, id, _)| *id == "blocker"),
        "an old term is dropped whole (ADR 0372 D1)"
    );
}
