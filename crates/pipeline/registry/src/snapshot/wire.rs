//! Adapters from the snapshot to fixed external shapes (ADR 0019, "Wire"): the pass input, the
//! custom validators' context and the coverage rule's entities.

use specforge_graph::ValueShape;
use specforge_protocol_types::{
    PassEdge, PassEntity, PassSpan, ValidatorContext, ValidatorEntity, ValidatorField,
    ValidatorMethod, ValidatorParam, ValidatorRef,
};
use std::collections::BTreeMap;

use super::{EntitySnapshot, Standing};
use crate::entity::EntityRecord;

impl EntitySnapshot {
    /// `PassInput::entities`: one [`pass_entity`] per entity, in id order.
    pub fn pass_entities(&self) -> Vec<PassEntity> {
        self.iter()
            .map(|(record, standing)| pass_entity(record, standing))
            .collect()
    }

    /// `PassInput::edges`, in graph order.
    pub fn pass_edges(&self) -> Vec<PassEdge> {
        self.edges()
            .iter()
            .map(|e| PassEdge {
                source: e.source.clone(),
                target: e.target.clone(),
                label: e.label.clone(),
            })
            .collect()
    }

    /// What a custom rule's `wasm_function` receives for `id`: its fields
    /// (every written one, in order, `value` its text as a JSON string), its
    /// methods, its references resolved against the snapshot (unique, in
    /// order; a dangling one has no kind), the declared types and the
    /// host's primitives.
    pub fn validator_context(&self, id: &str) -> Option<ValidatorContext> {
        let (record, _) = self.get(id)?;
        let referenced = self.references[self.index[id]]
            .iter()
            .map(|target| ValidatorRef {
                id: target.clone(),
                kind: self.kind_of(target).map(str::to_string),
            })
            .collect();
        Some(ValidatorContext {
            entity: ValidatorEntity {
                id: record.id.clone(),
                kind: record.kind.clone(),
                fields: record
                    .fields
                    .iter()
                    .map(|f| ValidatorField {
                        key: f.key.clone(),
                        value: serde_json::Value::String(f.text.clone()),
                        annotations: f.annotations.clone(),
                    })
                    .collect(),
                values: written_values(record),
                methods: record
                    .methods
                    .iter()
                    .map(|m| ValidatorMethod {
                        name: m.name.clone(),
                        params: m
                            .params
                            .iter()
                            .map(|p| ValidatorParam {
                                name: p.name.clone(),
                                ty: p.ty.clone(),
                            })
                            .collect(),
                        returns: m.returns.clone(),
                    })
                    .collect(),
            },
            referenced,
            declared_types: self.declared_types().to_vec(),
            primitives: primitives(),
        })
    }

    /// The context of the load-time probe of a custom rule: an entity of
    /// `kind` that writes nothing.
    pub fn probe_context(kind: &str) -> ValidatorContext {
        ValidatorContext {
            entity: ValidatorEntity {
                id: "__probe__".to_string(),
                kind: kind.to_string(),
                fields: Vec::new(),
                values: BTreeMap::new(),
                methods: Vec::new(),
            },
            referenced: Vec::new(),
            declared_types: Vec::new(),
            primitives: primitives(),
        }
    }

    /// The coverage rule's entities, in id order. They are the pass
    /// entities, read as the `@specforge/testing:coverage` pass reads them
    /// (`specforge_coverage::Entity::from`), so a per-entity view and the
    /// pass cannot disagree.
    pub fn coverage_entities(&self) -> Vec<specforge_coverage::Entity> {
        self.pass_entities()
            .iter()
            .map(specforge_coverage::Entity::from)
            .collect()
    }
}

/// One entity as a pass receives it:
/// - its field texts by key;
/// - its edge totals and span;
/// - `testable`, and `owes_none` = it owes no obligations of its own.
pub(super) fn pass_entity(record: &EntityRecord, standing: &Standing) -> PassEntity {
    PassEntity {
        id: record.id.clone(),
        kind: record.kind.clone(),
        fields: record
            .fields
            .iter()
            .map(|f| (f.key.clone(), f.text.clone()))
            .collect(),
        values: written_values(record),
        incoming_edge_count: record.incoming.total,
        outgoing_edge_count: record.outgoing.total,
        span: Some(PassSpan {
            file: record.span.file.as_str().to_string(),
            start_line: record.span.start_line,
            start_col: record.span.start_col,
            end_line: record.span.end_line,
            end_col: record.span.end_col,
        }),
        testable: standing.testable,
        owes_none: !standing.owes_obligations(),
        verify_kinds: record.obligations.iter().map(|o| o.kind.clone()).collect(),
        verify_texts: record.obligations.iter().map(|o| o.text.clone()).collect(),
        judgements: Vec::new(),
    }
}

/// The structured value of each of `record`'s typed blocks and block lists (ADR 0285) and of each list field, by field
/// name: a list as the array of its items, unjoined, so an item holding the joiner stays whole (ADR 0294). A pass and a
/// validator get the same.
fn written_values(record: &EntityRecord) -> BTreeMap<String, serde_json::Value> {
    record
        .fields
        .iter()
        .filter_map(|f| {
            let value = match (&f.written, &f.items) {
                (Some(written), _) => written.clone(),
                (None, Some(items)) if f.shape == ValueShape::List => items.as_slice().into(),
                _ => return None,
            };
            Some((f.key.clone(), value))
        })
        .collect()
}

fn primitives() -> Vec<String> {
    specforge_graph::PRIMITIVE_TYPES
        .iter()
        .map(|s| s.to_string())
        .collect()
}
