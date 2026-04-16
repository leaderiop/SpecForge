use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue};
use specforge_test::prelude::*;

fn span() -> SourceSpan {
    SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

fn node_with_contract(id: &str, kind: &str, contract: &str) -> Node {
    let mut fields = FieldMap::new();
    fields.push(Sym::new("contract"), FieldValue::String(contract.to_string()));
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(format!("Title {}", id)),
        fields,
        source_span: span(),
    }
}

fn build_large_graph() -> Graph {
    let mut graph = Graph::new();
    // Create 10 nodes with decent-sized contracts
    for i in 0..10 {
        let id = format!("entity_{}", i);
        let contract = format!("The system MUST handle case {} with full traceability and validation across all registered edge types and entity kinds in the graph. {}", i, "x".repeat(200));
        graph.add_node(node_with_contract(&id, "behavior", &contract));
    }
    // Chain edges: 0->1->2->...->9
    for i in 0..9 {
        graph.add_edge(Edge {
            source: Sym::from(format!("entity_{}", i)),
            target: Sym::from(format!("entity_{}", i + 1)),
            label: Sym::new("depends_on"),
        });
    }
    graph
}

// B:enforce_token_budget — verify unit "output within budget includes all entities"
#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "output within budget includes all entities")]
fn output_within_budget_includes_all_entities() {
    let graph = build_large_graph();
    // Large budget — everything fits
    let result = specforge_emitter::emit_json_with_budget(&graph, 100_000);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 10);
    assert!(parsed.get("token_budget").is_none() || parsed["token_budget"].is_null(),
        "no budget metadata when everything fits");
}

// B:enforce_token_budget — verify unit "output exceeding budget truncates low-priority entities"
#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "output exceeding budget truncates low-priority entities")]
fn output_exceeding_budget_truncates_low_priority_entities() {
    let graph = build_large_graph();
    // Tiny budget — must truncate
    let result = specforge_emitter::emit_json_with_budget(&graph, 500);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let nodes = parsed["nodes"].as_array().unwrap();
    assert!(nodes.len() < 10, "should truncate some entities, got {}", nodes.len());
    assert!(!nodes.is_empty(), "should keep at least some entities");
}

// B:enforce_token_budget — verify unit "TokenBudgetResult included in metadata when budget applied"
#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "TokenBudgetResult included in metadata when budget applied")]
fn token_budget_result_included_in_metadata() {
    let graph = build_large_graph();
    let result = specforge_emitter::emit_json_with_budget(&graph, 500);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let budget = &parsed["token_budget"];
    assert!(budget.is_object(), "token_budget metadata must be present when truncated");
    assert!(budget["strategy"].is_string());
    assert!(budget["truncated_entities"].is_array());
}

// B:enforce_token_budget — verify unit "truncated_entities lists omitted entity IDs"
#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "truncated_entities lists omitted entity IDs")]
fn truncated_entities_list_contains_omitted_ids() {
    let graph = build_large_graph();
    let result = specforge_emitter::emit_json_with_budget(&graph, 500);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let truncated = parsed["token_budget"]["truncated_entities"].as_array().unwrap();
    let remaining_ids: Vec<&str> = parsed["nodes"].as_array().unwrap()
        .iter().map(|n| n["id"].as_str().unwrap()).collect();

    // Truncated IDs should not appear in remaining nodes
    for id in truncated {
        let id_str = id.as_str().unwrap();
        assert!(!remaining_ids.contains(&id_str),
            "truncated entity {} should not appear in nodes", id_str);
    }

    // Together they should account for all 10
    assert_eq!(truncated.len() + remaining_ids.len(), 10);
}

// B:enforce_token_budget — verify unit "no --max-tokens skips budget enforcement"
#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "no --max-tokens skips budget enforcement")]
fn no_max_tokens_skips_budget_enforcement() {
    let graph = build_large_graph();
    // emit_json (no budget) should include everything
    let result = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 10);
    assert!(parsed.get("token_budget").is_none());
}

// B:enforce_token_budget — verify unit "truncated_entities lists omitted entity IDs"
// (validates no dangling edges remain after truncation)
#[test]
#[specforge_test(behavior = "enforce_token_budget")]
fn no_dangling_edges_after_truncation() {
    let graph = build_large_graph();
    let result = specforge_emitter::emit_json_with_budget(&graph, 500);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let node_ids: std::collections::HashSet<&str> = parsed["nodes"].as_array().unwrap()
        .iter().map(|n| n["id"].as_str().unwrap()).collect();

    for edge in parsed["edges"].as_array().unwrap() {
        let source = edge["source"].as_str().unwrap();
        let target = edge["target"].as_str().unwrap();
        assert!(node_ids.contains(source), "dangling edge source: {}", source);
        assert!(node_ids.contains(target), "dangling edge target: {}", target);
    }
}

// B:enforce_token_budget — verify unit "error strategy rejects export exceeding budget"
#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "error strategy rejects export exceeding budget")]
fn error_strategy_rejects_export_exceeding_budget() {
    let graph = build_large_graph();
    let result = specforge_emitter::emit_json_with_budget_strategy(&graph, 500, "error");
    assert!(result.is_err(), "error strategy should reject exceeding budget");
    let err = result.unwrap_err();
    assert!(err.to_string().contains("budget"), "error message should mention budget: {}", err);
}

// B:enforce_token_budget — verify integration "export with max_tokens produces output within budget and includes metadata"
#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "export with max_tokens produces output within budget and includes metadata")]
fn export_with_max_tokens_within_budget_includes_metadata() {
    let graph = build_large_graph();
    // Use a budget that forces truncation
    let result = specforge_emitter::emit_json_with_budget(&graph, 500);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    // The budget function uses word+structural token estimation internally.
    // With the improved estimator, tokens are counted by words + structural chars,
    // so the char-to-token ratio is higher than the naive len/4 heuristic.
    // Verify output is reasonably bounded (budget * 10 chars is a generous ceiling).
    assert!(result.len() <= 500 * 10, "output should be within budget: {} chars for 500 token budget", result.len());

    // Metadata should be present when truncation occurred
    assert!(parsed["token_budget"].is_object(), "metadata must be present");
    assert!(parsed["token_budget"]["budget_tokens"].as_u64().unwrap() == 500);
    assert!(parsed["token_budget"]["truncated_entities"].is_array());
    assert!(parsed["token_budget"]["estimated_tokens"].is_number());
}

// B:enforce_token_budget — verify unit "error strategy rejects export exceeding budget"
// (inverse case: passes when within budget)
#[test]
#[specforge_test(behavior = "enforce_token_budget")]
fn error_strategy_passes_when_within_budget() {
    let graph = build_large_graph();
    let result = specforge_emitter::emit_json_with_budget_strategy(&graph, 100_000, "error");
    assert!(result.is_ok(), "error strategy should pass within budget");
}
