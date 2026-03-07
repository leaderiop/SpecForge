// MCP Prompt behaviors — Guided prompts for agent workflows
//
// 4 behaviors: implement, review, trace, explore

use invariants/core
use invariants/mcp
use events/mcp
use types/graph
use types/mcp
use ports/inbound
use ports/outbound

behavior provide_mcp_implement_prompt "Provide MCP Implement Prompt" {
  invariants [graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpPromptDescriptor, Graph, McpImplementPromptResult]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]

  contract """
    In MCP server mode, the system MUST register a specforge://prompts/implement
    prompt that accepts entity_id (required). The prompt MUST return
    implementation context including the entity's contract text, all directly
    related entities (upstream and downstream), verify declarations as
    verification expectations, and related entities. If the entity does not
    exist, the prompt MUST return an error.
  """

  verify unit "specforge://prompts/implement returns implementation context"
  verify unit "response includes contract and related entities"
  verify unit "non-existent entity returns error"

}

behavior provide_mcp_review_prompt "Provide MCP Review Prompt" {
  invariants [graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpPromptDescriptor, McpCoverageResult, McpReviewPromptResult, McpReviewFinding]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]

  contract """
    In MCP server mode, the system MUST register a specforge://prompts/review
    prompt that accepts entity_id (required) and depth? (optional integer,
    default 1). The prompt MUST return a coverage analysis for the entity and
    its neighbors up to the specified depth, identifying missing test coverage,
    uncovered verify declarations, and entities lacking proof links.
  """

  verify unit "specforge://prompts/review returns coverage analysis"
  verify unit "response identifies entities with missing test coverage"
  verify unit "depth parameter controls neighbor traversal depth"

}

behavior provide_mcp_trace_prompt "Provide MCP Trace Prompt" {
  invariants [graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpPromptDescriptor, TraceChain, McpTracePromptResult, McpTraceGap]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]

  contract """
    In MCP server mode, the system MUST register a specforge://prompts/trace
    prompt that accepts plan (required, inline JSON describing intended changes).
    The prompt MUST perform gap analysis against the current graph, identify
    entities affected by the plan, flag missing traceability links, and return
    identified gaps with deterministic gap context.
  """

  verify unit "specforge://prompts/trace identifies gaps in plan"
  verify unit "response returns identified gaps with gap context"
  verify unit "affected entities are listed"

}

behavior provide_mcp_explore_prompt "Provide MCP Explore Prompt" {
  invariants [graph_traversal_integrity, diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  types      [McpPromptDescriptor, Graph, McpExplorePromptResult, McpRelationshipPath]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]

  contract """
    In MCP server mode, the system MUST register a specforge://prompts/explore
    prompt that accepts entity_id? (optional starting point) and kind? (optional
    entity kind filter). The prompt MUST return a guided exploration of the graph
    including suggested starting points, high-connectivity entities, and orphan
    nodes. When entity_id is provided, exploration MUST start from that entity.
    When kind is specified, results MUST be filtered to that entity kind.
  """

  verify unit "specforge://prompts/explore returns exploration starting points"
  verify unit "entity_id focuses exploration on that entity"
  verify unit "kind filter restricts results to matching entity kind"

}
