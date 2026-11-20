//! The delta payload against the wire, byte for byte.

use super::*;

fn bytes(payload: &DeltaPayload) -> String {
    serde_json::to_string(payload).unwrap_or_default()
}

fn read(text: &str) -> DeltaPayload {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

const BEFORE: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const AFTER: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";

#[test]
fn an_empty_change_writes_its_base_only() {
    let payload = DeltaPayload::default();
    assert_eq!(bytes(&payload), r#"{"base":{"kind":"none"}}"#);
    assert_eq!(read(&bytes(&payload)), payload);
}

#[test]
fn each_base_is_tagged_by_its_kind_and_round_trips() {
    for (base, wire) in [
        (DeltaBase::None, r#"{"base":{"kind":"none"}}"#),
        (
            DeltaBase::Revision { id: "a".repeat(40) },
            r#"{"base":{"kind":"revision","id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#,
        ),
        (DeltaBase::Previous, r#"{"base":{"kind":"previous"}}"#),
    ] {
        let payload = DeltaPayload {
            base,
            ..DeltaPayload::default()
        };
        assert_eq!(bytes(&payload), wire);
        assert_eq!(read(wire), payload);
    }
}

#[test]
fn a_node_writes_what_its_change_carries() {
    let added = DeltaNode {
        kind: "item".into(),
        id: "a".into(),
        change: DeltaChange::Added,
        fields: Vec::new(),
        aspects: Vec::new(),
        before: None,
        after: Some(AFTER.into()),
    };
    let removed = DeltaNode {
        kind: "item".into(),
        id: "b".into(),
        change: DeltaChange::Removed,
        fields: Vec::new(),
        aspects: Vec::new(),
        before: None,
        after: None,
    };
    let modified = DeltaNode {
        kind: "note".into(),
        id: "c".into(),
        change: DeltaChange::Modified,
        fields: vec!["size".into()],
        aspects: vec![DeltaAspect::Title, DeltaAspect::Edges],
        before: Some(BEFORE.into()),
        after: Some(AFTER.into()),
    };
    let payload = DeltaPayload {
        base: DeltaBase::Previous,
        nodes: vec![added, removed, modified],
        added_edges: vec![DeltaEdge {
            source: "c".into(),
            target: "a".into(),
            label: "refs".into(),
        }],
        removed_edges: Vec::new(),
    };
    let wire = format!(
        concat!(
            r#"{{"base":{{"kind":"previous"}},"nodes":["#,
            r#"{{"kind":"item","id":"a","change":"added","after":"{after}"}},"#,
            r#"{{"kind":"item","id":"b","change":"removed"}},"#,
            r#"{{"kind":"note","id":"c","change":"modified","fields":["size"],"aspects":["title","edges"],"#,
            r#""before":"{before}","after":"{after}"}}],"#,
            r#""added_edges":[{{"source":"c","target":"a","label":"refs"}}]}}"#
        ),
        before = BEFORE,
        after = AFTER
    );
    assert_eq!(bytes(&payload), wire);
    assert_eq!(read(&wire), payload);
}

#[test]
fn every_aspect_is_spelled_as_the_graph_names_it() {
    let spelled: Vec<String> = [
        DeltaAspect::Title,
        DeltaAspect::Methods,
        DeltaAspect::Obligations,
        DeltaAspect::Verdicts,
        DeltaAspect::Edges,
    ]
    .iter()
    .map(|a| serde_json::to_string(a).unwrap_or_default())
    .collect();
    assert_eq!(
        spelled,
        [
            "\"title\"",
            "\"methods\"",
            "\"obligations\"",
            "\"verdicts\"",
            "\"edges\""
        ]
    );
}

#[test]
fn unknown_keys_are_ignored_on_read_and_empty_lists_are_absent_on_write() {
    let read = read(
        r#"{"base":{"kind":"none"},"nodes":[],"removed_edges":[],"later":1,
            "added_edges":[{"source":"a","target":"b","label":"x","weight":2}]}"#,
    );
    assert_eq!(
        read,
        DeltaPayload {
            base: DeltaBase::None,
            nodes: Vec::new(),
            added_edges: vec![DeltaEdge {
                source: "a".into(),
                target: "b".into(),
                label: "x".into()
            }],
            removed_edges: Vec::new(),
        }
    );
    assert_eq!(
        bytes(&read),
        r#"{"base":{"kind":"none"},"added_edges":[{"source":"a","target":"b","label":"x"}]}"#
    );
}
