mod build;
mod coerce;
pub use coerce::FieldCoercion;
mod derive;
pub use derive::{DerivedFrom, DerivedReference};
pub mod cycles;
mod graph;
pub use cycles::{CycleOptions, find_cycles};
pub mod rename;

pub use build::{
    GraphConfig, build_graph, build_graph_with_config, entity_pass, is_define_block,
    link_and_diagnose, node_from_entity,
};
pub use graph::{Edge, Graph, Node};
pub use specforge_common::{Diagnostic, Severity, SourceSpan};
pub use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, SpecFile};
