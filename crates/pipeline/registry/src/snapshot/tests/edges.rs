//! An entity's edge counts: the edges to and from other entities (ADR 0270).

use super::{declared, graph_of};
use crate::snapshot::EntitySnapshot;
use std::path::Path;

#[specforge_test_macros::test(
    behavior = "snapshot_entities_once",
    verify = "an entity's edge counts leave out an edge from it to itself, as its degree does, and the pass input carries those counts"
)]
fn an_entitys_edge_counts_leave_out_an_edge_to_itself() {
    let registries = declared(&[("behavior", true, true)], false, vec![]);
    let source = "behavior loop \"Loop\" {\n  features [loop, other]\n}\n\n\
                  behavior other \"Other\" {\n}\n";
    let build = graph_of(source, &registries);
    let snapshot = EntitySnapshot::of(&build, &registries, Path::new(""), Path::new(""));
    let by_id = |id: &str| {
        snapshot
            .records()
            .iter()
            .find(|r| r.id == id)
            .expect("a record")
    };
    let passed = snapshot.pass_entities();
    assert_eq!(passed.len(), 2);
    for pass in &passed {
        let record = by_id(&pass.id);
        let degree = build.graph().degree(&pass.id);
        assert_eq!(record.incoming.total, degree.incoming, "{}", pass.id);
        assert_eq!(record.outgoing.total, degree.outgoing, "{}", pass.id);
        assert_eq!(
            pass.incoming_edge_count, record.incoming.total,
            "{}",
            pass.id
        );
        assert_eq!(
            pass.outgoing_edge_count, record.outgoing.total,
            "{}",
            pass.id
        );
        let typed = |counts: &crate::entity::EdgeCounts| counts.by_type.values().sum::<usize>();
        assert!(typed(&record.incoming) <= record.incoming.total);
        assert!(typed(&record.outgoing) <= record.outgoing.total);
    }
    let looped = by_id("loop");
    assert_eq!((looped.incoming.total, looped.outgoing.total), (0, 1));
    assert!(looped.linked.any, "a written self-reference keeps its link");
}
