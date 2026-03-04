use std::collections::HashMap;

use crate::junit::{RawTestResult, RawTestStatus};
use crate::report::{EntityResult, TestResult, TestStatus};
use specforge_graph::SpecGraph;

/// Configuration for entity mapping.
pub struct EntityMapConfig {
    /// Explicit test-name-to-entity-id mappings (from `tests` field).
    pub explicit_mappings: HashMap<String, String>,
    /// If true, unknown entity IDs cause errors. If false, emit warnings.
    pub strict: bool,
}

impl Default for EntityMapConfig {
    fn default() -> Self {
        Self {
            explicit_mappings: HashMap::new(),
            strict: false,
        }
    }
}

/// Result of mapping tests to entities.
pub struct EntityMapResult {
    pub entities: Vec<EntityResult>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub unmapped_count: usize,
}

/// Map raw test results to spec entities.
///
/// Resolution order:
/// 1. Explicit mappings from config
/// 2. Convention: split test name on `__` to extract entity_id prefix
pub fn map_tests_to_entities(
    raw_results: &[RawTestResult],
    graph: &SpecGraph,
    config: &EntityMapConfig,
) -> EntityMapResult {
    let mut entity_tests: HashMap<String, Vec<TestResult>> = HashMap::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut unmapped_count = 0;

    // Build a set of known entity IDs for validation
    let known_ids: std::collections::HashSet<String> = graph
        .nodes()
        .map(|n| n.id.raw().to_string())
        .collect();

    for raw in raw_results {
        // Try explicit mapping first
        let entity_id = if let Some(id) = config.explicit_mappings.get(&raw.name) {
            Some(id.clone())
        } else {
            // Convention: split on `__` to get entity_id
            extract_entity_id(&raw.name)
        };

        let entity_id = match entity_id {
            Some(id) => id,
            None => {
                unmapped_count += 1;
                let msg = format!("unmapped test: {}", raw.name);
                if config.strict {
                    errors.push(msg);
                } else {
                    warnings.push(msg);
                }
                continue;
            }
        };

        // Validate entity_id exists in graph
        if !known_ids.contains(&entity_id) {
            let msg = format!("unknown entity '{}' from test '{}'", entity_id, raw.name);
            if config.strict {
                errors.push(msg);
            } else {
                warnings.push(msg);
            }
            continue;
        }

        let test_result = TestResult {
            name: raw.name.clone(),
            status: convert_status(raw.status),
            duration_ms: raw.duration_secs.map(|s| s * 1000.0),
            message: raw.message.clone(),
        };

        entity_tests
            .entry(entity_id)
            .or_default()
            .push(test_result);
    }

    let mut entities: Vec<EntityResult> = entity_tests
        .into_iter()
        .map(|(entity_id, tests)| EntityResult { entity_id, tests })
        .collect();
    entities.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));

    EntityMapResult {
        entities,
        warnings,
        errors,
        unmapped_count,
    }
}

/// Extract entity_id from a test name using the double-underscore convention.
fn extract_entity_id(test_name: &str) -> Option<String> {
    // Strip module path prefix (e.g., "tests::validate_input__foo" -> "validate_input__foo")
    let name = test_name.rsplit("::").next().unwrap_or(test_name);
    let (entity_id, _rest) = name.split_once("__")?;
    if entity_id.is_empty() {
        return None;
    }
    Some(entity_id.to_string())
}

fn convert_status(raw: RawTestStatus) -> TestStatus {
    match raw {
        RawTestStatus::Pass => TestStatus::Pass,
        RawTestStatus::Fail => TestStatus::Fail,
        RawTestStatus::Skip => TestStatus::Skip,
        RawTestStatus::Error => TestStatus::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::FieldRegistry;

    fn make_graph_with_entities(ids: &[&str]) -> SpecGraph {
        // Build a minimal graph with the given entity IDs
        let mut spec_source = String::from("spec \"test\" {\n  version \"1.0\"\n  plugins []\n}\n");
        for id in ids {
            spec_source.push_str(&format!(
                "behavior {id} \"{id}\" {{\n  contract \"\"\"test\"\"\"\n}}\n"
            ));
        }
        let parsed = specforge_parser::parse(&spec_source, "test.spec");
        let resolved = specforge_resolver::resolve(vec![parsed], ".");
        let registry = FieldRegistry::with_builtins();
        let result = specforge_graph::build_graph(&resolved.files, &registry);
        result.graph
    }

    #[test]
    fn extract_entity_id_from_convention() {
        assert_eq!(
            extract_entity_id("validate_input__rejects_empty"),
            Some("validate_input".to_string())
        );
    }

    #[test]
    fn extract_entity_id_with_module_path() {
        assert_eq!(
            extract_entity_id("tests::validate_input__rejects_empty"),
            Some("validate_input".to_string())
        );
    }

    #[test]
    fn extract_entity_id_no_separator() {
        assert_eq!(extract_entity_id("some_test_without_separator"), None);
    }

    #[test]
    fn map_happy_path() {
        let graph = make_graph_with_entities(&["validate_input"]);
        let raw = vec![
            RawTestResult {
                name: "validate_input__rejects_empty".to_string(),
                classname: None,
                status: RawTestStatus::Pass,
                duration_secs: Some(0.001),
                message: None,
            },
            RawTestResult {
                name: "validate_input__accepts_valid".to_string(),
                classname: None,
                status: RawTestStatus::Fail,
                duration_secs: None,
                message: Some("assertion failed".to_string()),
            },
        ];
        let config = EntityMapConfig::default();
        let result = map_tests_to_entities(&raw, &graph, &config);
        assert!(result.errors.is_empty());
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].entity_id, "validate_input");
        assert_eq!(result.entities[0].tests.len(), 2);
    }

    #[test]
    fn strict_mode_unknown_entity() {
        let graph = make_graph_with_entities(&["validate_input"]);
        let raw = vec![RawTestResult {
            name: "unknown_entity__some_test".to_string(),
            classname: None,
            status: RawTestStatus::Pass,
            duration_secs: None,
            message: None,
        }];
        let config = EntityMapConfig {
            strict: true,
            ..Default::default()
        };
        let result = map_tests_to_entities(&raw, &graph, &config);
        assert!(!result.errors.is_empty());
        assert!(result.errors[0].contains("unknown entity"));
    }

    #[test]
    fn lenient_mode_unknown_entity() {
        let graph = make_graph_with_entities(&["validate_input"]);
        let raw = vec![RawTestResult {
            name: "unknown_entity__some_test".to_string(),
            classname: None,
            status: RawTestStatus::Pass,
            duration_secs: None,
            message: None,
        }];
        let config = EntityMapConfig::default();
        let result = map_tests_to_entities(&raw, &graph, &config);
        assert!(result.errors.is_empty());
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn unmapped_tests_counted() {
        let graph = make_graph_with_entities(&["validate_input"]);
        let raw = vec![RawTestResult {
            name: "no_separator_test".to_string(),
            classname: None,
            status: RawTestStatus::Pass,
            duration_secs: None,
            message: None,
        }];
        let config = EntityMapConfig::default();
        let result = map_tests_to_entities(&raw, &graph, &config);
        assert_eq!(result.unmapped_count, 1);
    }
}
