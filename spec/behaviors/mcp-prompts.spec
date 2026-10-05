// MCP Prompt behaviors — Guided prompts for agent workflows
//
// P7 Justification: Core prompts are domain-agnostic graph operations (implement,
// review, trace, explore). They contain zero domain knowledge — they traverse
// generic graph nodes and edges. No extension contributes a prompt today: an
// extension's surfaces are commands, MCP tools and MCP resources
// (ExtensionContributions.prompts stays reserved).
//
// 5 behaviors: serve, context, review, trace, explore (infer's scopes are in
// behaviors/infer.spec)

use "events/mcp"
use "invariants/core"
use "invariants/mcp"
use "ports/inbound"
use "ports/outbound"
use "types/graph"
use "types/mcp"

behavior serve_mcp_prompt "Serve MCP Prompt" {
  features   [mcp_prompts]
  invariants [mcp_structured_error_responses, mcp_served_project_consistency]
  category   query
  types      [McpPromptDescriptor, McpPromptArgument, McpError]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]
  ensures {
    arguments_derived  "listed arguments are exactly those the prompt reads, required exactly when it cannot render without them"
    refusal_structured "a prompt that cannot render answers a JSON-RPC error whose data is an McpError"
    one_layout         "every prompt result is an instruction then a JSON payload, both user messages"
    fresh_graph        "a prompt renders the graph a tool call would see"
  }
  contract   """
    Every core prompt is one Prompt spec: prompts/list derives its arguments
    from the typed arguments prompts/get reads, so a listed argument is read
    and a required one is one the prompt cannot render without. MCP sends
    prompt arguments as strings; a count is read from a string. Arguments
    that are not an object are -32602. A prompt that cannot render (an
    invalid argument, an unknown entity, an unusable project file) answers
    a JSON-RPC error, -32602 for input the client can fix and -32603 for a
    server-side failure, whose data is an McpError naming the prompt. A
    rendered prompt is its description and two user messages: the
    instruction, then the JSON payload. prompts/get first brings the
    served project up to date with disk, as tools/call does
    (bring_session_up_to_date). An unknown prompt is -32602 and is not an
    invocation.
  """
  verify unit "each core prompt lists exactly the arguments its handler reads"
  verify unit "a listed required argument is exactly one the prompt cannot render without"
  verify unit "a prompt refusal is a JSON-RPC error whose data is an McpError naming the prompt"
  verify unit "a missing required prompt argument is -32602 naming the argument"
  verify unit "prompt arguments that are not an object produce -32602 Invalid params"
  verify unit "a numeric prompt argument is read from a string, as MCP sends it"
  verify unit "every prompt result is an instruction then a JSON payload, both user messages"
  verify unit "an unknown prompt records no mcp_prompt_invoked event"
  verify unit "a stateless prompts/get renders without initialize"
}

behavior provide_mcp_context_prompt "Provide MCP Context Prompt" {
  features   [mcp_prompts]
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpPromptDescriptor, Graph, McpContextPromptResult]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    context_returned       "Structured entity context returned: contract, related entities, verify declarations"
    hints_included         "structural_constraints entities included as additional context even if not directly connected"
    prompt_invoked_emitted "mcp_prompt_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge://prompts/context
    prompt that accepts entity_id (required) and structural_constraints? (optional
    string array of additional entity IDs to include as context). The prompt
    MUST return structured entity context including the entity's contract text,
    every field the entity declares (an invariant's guarantee, a decision's
    rationale, whatever its kind names its text),
    all directly related entities (upstream and downstream), verify
    declarations as verification expectations, and related entities.
    structural_constraints entities are included as additional context even if not
    directly connected. If the entity does not exist, the prompt MUST
    return an error.
  """
  verify unit "specforge://prompts/context returns structured entity context"
  verify unit "response includes contract and related entities"
  verify unit "context includes every field, like an invariant's guarantee"
  verify unit "non-existent entity returns error"
  verify unit "context prompt works with zero extensions installed"
  verify contract "Provide MCP Context Prompt: MCP context prompt holds — graph_available, context_returned, hints_included, prompt_invoked_emitted"
}

behavior provide_mcp_review_prompt "Provide MCP Review Prompt" {
  features   [mcp_prompts]
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpPromptDescriptor, McpCoverageResult, McpReviewPromptResult, McpReviewFinding]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    coverage_analysis_returned "Coverage analysis returned for entity and neighbors up to specified depth"
    gaps_identified            "Missing verification coverage, uncovered verify declarations, and missing evidence links identified"
    prompt_invoked_emitted     "mcp_prompt_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge://prompts/review
    prompt that accepts entity_id? (optional; the whole graph when omitted)
    and depth? (optional count, default 1). The prompt MUST return a coverage analysis for the entity and
    its neighbors up to the specified depth, identifying missing verification coverage,
    uncovered verify declarations, and entities lacking evidence links.
  """
  verify unit "specforge://prompts/review returns coverage analysis"
  verify unit "review coverage matches specforge.coverage obligation by obligation"
  verify unit "response identifies entities with missing verification coverage"
  verify unit "depth parameter controls neighbor traversal depth"
  verify unit "review prompt returns empty findings when no testable entities exist"
  verify contract "Provide MCP Review Prompt: MCP review prompt holds — graph_available, coverage_analysis_returned, gaps_identified, prompt_invoked_emitted"
  verify unit "detects orphan entities"
}

behavior provide_mcp_trace_prompt "Provide MCP Trace Prompt" {
  features   [mcp_prompts]
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpPromptDescriptor, TraceChain, McpTracePromptResult, McpTraceGap]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    gaps_returned            "Identified gaps returned with deterministic gap context"
    affected_entities_listed "Entities affected by the plan listed in response"
    prompt_invoked_emitted   "mcp_prompt_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge://prompts/trace
    prompt that accepts plan (inline JSON describing intended changes) or, in
    its place, entity_id (trace that one entity's chain).
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
  verify contract "Provide MCP Trace Prompt: MCP trace prompt holds — graph_available, gaps_returned, affected_entities_listed, prompt_invoked_emitted"
}

behavior provide_mcp_explore_prompt "Provide MCP Explore Prompt" {
  features   [mcp_prompts]
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpPromptDescriptor, Graph, McpExplorePromptResult, McpRelationshipPath]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_prompt_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    exploration_returned   "Guided exploration returned: starting points, high-connectivity entities, orphan nodes"
    bfs_from_entity        "When entity_id provided, BFS traversal starts from that node"
    prompt_invoked_emitted "mcp_prompt_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge://prompts/explore
    prompt that accepts entity_id? (optional starting point) and kind? (optional
    entity kind filter). The prompt MUST return a guided exploration of the graph
    including suggested starting points, high-connectivity entities, and orphan
    nodes. When entity_id is provided, exploration MUST start from that entity
    using BFS traversal from that node. When kind is specified, results MUST
    be filtered to that entity kind. depth? (optional count; unbounded when
    omitted) bounds the BFS, which reaches exactly the entities review's
    depth reaches. If entity_id names no entity, the prompt MUST return an
    error.
  """
  verify unit "specforge://prompts/explore returns exploration starting points"
  verify unit "entity_id focuses exploration on that entity"
  verify unit "kind filter restricts results to matching entity kind"
  verify unit "high_connectivity field lists entities with highest edge degree"
  verify unit "orphan_nodes field lists entities with zero incoming and outgoing edges"
  verify unit "unknown entity_id returns error"
  verify unit "explore and review reach the same entities at the same depth"
  verify contract "Provide MCP Explore Prompt: MCP explore prompt holds — graph_available, exploration_returned, bfs_from_entity, prompt_invoked_emitted"
}
