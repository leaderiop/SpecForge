mod build;
mod graph;

pub use build::{build_graph, build_graph_with_config, GraphConfig};
pub use graph::{Edge, Graph, Node};
pub use specforge_common::{Diagnostic, Severity, SourceSpan};
pub use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, SpecFile};
