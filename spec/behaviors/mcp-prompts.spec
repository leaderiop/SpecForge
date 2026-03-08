// MCP Prompt behaviors — Guided prompts for agent workflows
//
// P7 Justification: Core prompts are domain-agnostic graph operations (implement,
// review, trace, explore). They contain zero domain knowledge — they traverse
// generic graph nodes and edges. Extensions MAY contribute additional domain-specific
// prompts via contributes.prompts in their manifest (see ExtensionContributions).
//
// 4 behaviors: context, review, trace, explore

use invariants/core
use invariants/mcp
use events/mcp
use types/graph
use types/mcp
use ports/inbound
use ports/outbound

behavior provide_mcp_context_prompt "Provide MCP Context Prompt" {
  invariants [graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpPromptDescriptor, Graph, McpContextPromptResult]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]

  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }

  ensures {
    context_returned "Structured entity context returned: contract, related entities, verify declarations"
    hints_included "structural_constraints entities included as additional context even if not directly connected"
    prompt_invoked_emitted "mcp_prompt_invoked event emitted"
  }

  contract """
    In MCP server mode, the system MUST register a specforge://prompts/context
    prompt that accepts entity_id (required) and structural_constraints? (optional
    string array of additional entity IDs to include as context). The prompt
    MUST return structured entity context including the entity's contract text,
    all directly related entities (upstream and downstream), verify
    declarations as verification expectations, and related entities.
    structural_constraints entities are included as additional context even if not
    directly connected. If the entity does not exist, the prompt MUST
    return an error.
  """

  verify unit "specforge://prompts/context returns structured entity context"
  verify unit "response includes contract and related entities"
  verify unit "non-existent entity returns error"
  verify unit "context prompt works with zero extensions installed"
  verify contract "requires/ensures consistency for MCP context prompt"

}

behavior provide_mcp_review_prompt "Provide MCP Review Prompt" {
  invariants [graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpPromptDescriptor, McpCoverageResult, McpReviewPromptResult, McpReviewFinding]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]

  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }

  ensures {
    coverage_analysis_returned "Coverage analysis returned for entity and neighbors up to specified depth"
    gaps_identified "Missing verification coverage, uncovered verify declarations, and missing evidence links identified"
    prompt_invoked_emitted "mcp_prompt_invoked event emitted"
  }

  contract """
    In MCP server mode, the system MUST register a specforge://prompts/review
    prompt that accepts entity_id (required) and depth? (optional integer,
    default 1). The prompt MUST return a coverage analysis for the entity and
    its neighbors up to the specified depth, identifying missing verification coverage,
    uncovered verify declarations, and entities lacking evidence links.
  """

  verify unit "specforge://prompts/review returns coverage analysis"
  verify unit "response identifies entities with missing verification coverage"
  verify unit "depth parameter controls neighbor traversal depth"
  verify unit "review prompt returns empty findings when no testable entities exist"
  verify contract "requires/ensures consistency for MCP review prompt"

}

behavior provide_mcp_trace_prompt "Provide MCP Trace Prompt" {
  invariants [graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpPromptDescriptor, TraceChain, McpTracePromptResult, McpTraceGap]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]

  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }

  ensures {
    gaps_returned "Identified gaps returned with deterministic gap context"
    affected_entities_listed "Entities affected by the plan listed in response"
    prompt_invoked_emitted "mcp_prompt_invoked event emitted"
  }

  contract """
    In MCP server mode, the system MUST register a specforge://prompts/trace
    prompt that accepts plan (required, inline JSON describing intended changes).
    The prompt MUST perform gap analysis against the current graph, identify
    entities affected by the plan, flag missing traceability links, and return
    identified gaps with deterministic gap context.
    The plan parameter MUST conform to the AgentPlan type defined in
    types/graph.spec. If the JSON does not conform, the prompt MUST
    return an error with descriptive validation messages.
  """

  verify unit "specforge://prompts/trace identifies gaps in plan"
  verify unit "response returns identified gaps with gap context"
  verify unit "affected entities are listed"
  verify unit "malformed plan JSON returns validation error"
  verify contract "requires/ensures consistency for MCP trace prompt"

}

behavior provide_mcp_explore_prompt "Provide MCP Explore Prompt" {
  invariants [graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpPromptDescriptor, Graph, McpExplorePromptResult, McpRelationshipPath]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]

  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }

  ensures {
    exploration_returned "Guided exploration returned: starting points, high-connectivity entities, orphan nodes"
    bfs_from_entity "When entity_id provided, BFS traversal starts from that node"
    prompt_invoked_emitted "mcp_prompt_invoked event emitted"
  }

  contract """
    In MCP server mode, the system MUST register a specforge://prompts/explore
    prompt that accepts entity_id? (optional starting point) and kind? (optional
    entity kind filter). The prompt MUST return a guided exploration of the graph
    including suggested starting points, high-connectivity entities, and orphan
    nodes. When entity_id is provided, exploration MUST start from that entity
    using BFS traversal from that node. When kind is specified, results MUST
    be filtered to that entity kind.
  """

  verify unit "specforge://prompts/explore returns exploration starting points"
  verify unit "entity_id focuses exploration on that entity"
  verify unit "kind filter restricts results to matching entity kind"
  verify unit "high_connectivity field lists entities with highest edge degree"
  verify unit "orphan_nodes field lists entities with zero incoming and outgoing edges"
  verify contract "requires/ensures consistency for MCP explore prompt"

}
