//! The ids absorb converts from this repository's own spec, read as an old project as `cargo xtask absorb` reads it,
//! are no keyword of the target: a kind `@gsd/core` declares, or a structural keyword, is an id the target refuses
//! (E013). This test reads the self-spec as an old project, so the swap removes it with the self-spec sample (ADR 0367
//! D1).

use serde_json::Value;
use std::collections::BTreeSet;

use super::super::read;
use super::super::table::{self, Fate};

/// The structural keywords of the core grammar: an id no project may take.
const STRUCTURAL: [&str; 4] = ["spec", "ref", "use", "define"];

#[test]
fn no_old_id_the_root_spec_converts_is_a_kind_keyword() {
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
    let converted: Vec<(&str, &str)> = entities
        .iter()
        .filter(|e| table::fate(&e.kind).and_then(Fate::new_kind).is_some())
        .map(|e| (e.kind.as_str(), e.id.as_str()))
        .collect();
    assert!(
        converted.len() >= 300,
        "the self-spec's features, constraints, invariants, decisions, releases and milestones: {}",
        converted.len()
    );
    let clashing: Vec<&(&str, &str)> = converted
        .iter()
        .filter(|(_, id)| keywords.contains(*id))
        .collect();
    assert!(
        clashing.is_empty(),
        "old ids absorb converts that the target refuses as a keyword: {clashing:?}"
    );
    assert!(
        entities
            .iter()
            .any(|e| e.kind == "term" && e.id == "blocker"),
        "the old term that stopped the first trial with E013"
    );
    assert!(
        !converted.iter().any(|(_, id)| *id == "blocker"),
        "an old term is dropped whole (ADR 0372 D1)"
    );
}
