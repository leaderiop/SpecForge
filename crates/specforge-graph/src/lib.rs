mod build;
mod coerce;
pub use coerce::FieldCoercion;
mod derive;
pub use derive::{DerivedFrom, DerivedReference};
mod graph;
mod obligations;
// The cycle walker lives in specforge-common (the registry's rules use it
// without linking the graph); re-exported here unchanged.
pub use obligations::obligations;
pub use specforge_common::cycles::{CycleOptions, find_cycles};
pub mod rename;

pub use build::{
    GraphConfig, build_graph, build_graph_with_config, entity_pass, is_define_block,
    link_and_diagnose, node_from_entity,
};
pub use graph::{Edge, Graph, Node, Reached};
pub use specforge_common::{Diagnostic, Severity, SourceSpan};
pub use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, SpecFile};
