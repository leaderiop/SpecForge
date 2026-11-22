//! A pass is sent each entity's title (ADR 0387), as a command's graph node carries it.

use crate::snapshot::EntitySnapshot;

#[test]
fn a_pass_entity_carries_the_title_its_header_writes() {
    let registries = super::declared(&[("memo", false, false)], false, vec![]);
    let source = "memo note \"Phase 3: Core\" {\n}\n\nmemo other \"Other\" {\n}\n";
    let snapshot = EntitySnapshot::of(&super::graph_of(source, &registries), &registries);
    let titles: Vec<(String, Option<String>)> = snapshot
        .pass_entities(&[])
        .into_iter()
        .map(|e| (e.id, e.title))
        .collect();
    assert_eq!(
        titles,
        [
            ("note".to_string(), Some("Phase 3: Core".to_string())),
            ("other".to_string(), Some("Other".to_string())),
        ]
    );
}
