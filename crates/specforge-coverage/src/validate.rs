use crate::merge::MergedReport;
use specforge_graph::SpecGraph;

/// Validate that all entity IDs in the merged report exist in the spec graph.
///
/// Returns a list of unknown entity IDs found in the report but not in the graph.
pub fn validate_report_ids(report: &MergedReport, graph: &SpecGraph) -> Vec<String> {
    let mut unknown = Vec::new();
    for entity_id in report.entities.keys() {
        if graph.get_node(entity_id).is_none() {
            unknown.push(entity_id.clone());
        }
    }
    unknown.sort();
    unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{EntityResult, SpecForgeReport, TestResult, TestStatus};
    use specforge_common::{EntityId, EntityKind, SourceSpan};
    use specforge_graph::GraphNode;

    #[test]
    fn validates_known_ids() {
        let mut graph = SpecGraph::new();
        graph.add_node(GraphNode {
            id: EntityId::parse("auth_login"),
            kind: EntityKind::Behavior,
            title: None,
            file: "test.spec".to_string(),
            span: SourceSpan::file_start("test.spec"),
        });

        let mut merged = MergedReport::new();
        merged.merge_report(SpecForgeReport {
            schema_version: "1.0".to_string(),
            timestamp: None,
            adapter: None,
            entities: vec![EntityResult {
                entity_id: "auth_login".to_string(),
                tests: vec![TestResult {
                    name: "test".to_string(),
                    status: TestStatus::Pass,
                    duration_ms: None,
                    message: None,
                }],
            }],
        });

        let unknown = validate_report_ids(&merged, &graph);
        assert!(unknown.is_empty());
    }

    #[test]
    fn detects_unknown_ids() {
        let graph = SpecGraph::new();

        let mut merged = MergedReport::new();
        merged.merge_report(SpecForgeReport {
            schema_version: "1.0".to_string(),
            timestamp: None,
            adapter: None,
            entities: vec![EntityResult {
                entity_id: "nonexistent".to_string(),
                tests: vec![TestResult {
                    name: "test".to_string(),
                    status: TestStatus::Pass,
                    duration_ms: None,
                    message: None,
                }],
            }],
        });

        let unknown = validate_report_ids(&merged, &graph);
        assert_eq!(unknown, vec!["nonexistent"]);
    }
}
