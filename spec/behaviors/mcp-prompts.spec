// MCP Prompt behaviors — Guided prompts for agent workflows
//
// P7 Justification: Core prompts are domain-agnostic graph operations (implement,
// review, trace, explore). They contain zero domain knowledge — they traverse
// generic graph nodes and edges. No extension contributes a prompt today: an
// extension's surfaces are commands, MCP tools and MCP resources.
//
// 5 behaviors: serve, context, review, trace, explore (infer's scopes are in
// behaviors/infer.spec; the exploration and the review they render are in
// behaviors/output.spec)

use "events/mcp"
use "invariants/core"
use "invariants/mcp"
use "ports/inbound"
use "ports/outbound"
use "types/graph"
use "types/mcp"

defaults behavior {
  features [mcp_prompts]
  ports    [McpProtocol, CompilerApi]
  produces [mcp_prompt_invoked]
}

behavior serve_mcp_prompt "Serve MCP Prompt" {
  invariants [mcp_structured_error_responses, mcp_served_project_consistency]
  category   query
  types      [McpPromptDescriptor, McpPromptArgument, McpError]
  ensures {
    arguments_derived  "listed arguments are exactly those the prompt reads, required exactly when it cannot render without them"
    refusal_structured "a prompt that cannot render answers a JSON-RPC error whose data is an McpError"
    one_layout         "every prompt result is an instruction then a JSON payload, both user messages"
    fresh_graph        "a prompt renders the graph a tool call would see"
  }
  contract   """
    Every core prompt is one module: its name, description, typed arguments,
    the spec type its payload is and its typed payload are stated there
    once; prompts/list derives its arguments from the typed arguments
    prompts/get reads, and the payload is serialized from its type in the
    type's field order, so a listed argument is read
    and a required one is one the prompt cannot render without. MCP sends
    prompt arguments as strings; a count is read from a string. An
    argument the prompt does not declare MUST be refused as invalid input
    naming it (read_mcp_arguments_as_declared). Arguments
    that are not an object are -32602. A prompt that cannot render (an
    invalid argument, an unknown entity, an unusable project file) answers
    a JSON-RPC error, -32602 for input the client can fix and -32603 for a
    server-side failure, whose data is an McpError naming the prompt. A
    rendered prompt is its description and two user messages: the
    instruction, then the JSON payload. prompts/get first brings the
    served project up to date with disk, as tools/call does
    (bring_session_up_to_date). An unknown prompt is -32602 and is not an
    invocation. A prompt or the server's instructions name a tool only as
    tools/list names it, read from the core tool table.
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
  verify unit "every tool a prompt or the server's instructions name is a core tool, named as tools/list names it"
  verify unit "each core prompt is one module: its name, description, arguments, spec type and typed payload are stated there once"
  verify unit "each prompt's payload is the spec type its module names"
}

behavior serve_extension_prompt "Serve an Extension Prompt" {
  invariants [mcp_structured_error_responses, mcp_served_project_consistency]
  category   query
  types      [McpPromptDescriptor, McpPromptArgument, McpPromptResult, McpPromptReplyMessage, McpError]
  contract   """
    An extension contributes MCP prompts in its surfaces (ADR 0291): each is served as specforge.<short>.<id>
    after the core prompts, rendered by the extension's prompt export from typed arguments, with one host message
    appended (the prompt's version and digest, its attribution, each step and whether its gate passes now). A
    project whose extensions declare no prompt lists and serves the core prompts exactly as before. The core tool
    specforge.prompt renders any prompt by name from its arguments or an argv string, for the host the client named
    or the one it is given, and answers its messages and the host message's payload.
  """
  verify unit "a project whose extensions declare no prompt lists and serves the core prompts unchanged"
  verify unit "a prompt argument is read by its field type and a reference is sent as its full path"
  verify unit "a prompt argument outside its field type is invalid input on that argument"
  verify unit "a prompt whose entry gate fails is refused and its export is not called"
  verify unit "a rendered prompt shows its version, digest, attribution and each step with its gate"
  verify unit "a prompt export that traps is E028"
  verify unit "specforge prompt prints a rendered prompt, as JSON with its host message, and refuses a failing gate"
  verify unit "an extension's name routes a prompt when no command of it has that name"
  verify unit "a prompt declaration round-trips through the protocol with its args, steps and attribution"
  verify unit "an extension prompt is listed after the core ones and served by prompts/get"
  verify unit "a prompt whose when setting is off is not listed or served"
  verify unit "an agent renders any prompt through specforge.prompt, by name and arguments or by argv"
  verify unit "the client names the host a prompt renders for, and a host argument overrides it"
  verify unit "a prompt declared with the SDK is rendered by its export from the arguments and host the call carries"
  verify unit "the SDK refuses a prompt declared twice or without a handler"
  verify unit "the host sends a prompt export its typed arguments and host and reads back its messages"
  verify unit "a prompt export that answers no message is E028"
  verify unit "a pass input without a trigger reads a write and a gate trigger round-trips with its command and subject"
  verify unit "a prompt answer that is not a PromptOutput is E028"
  verify unit "a reference argument names an entity of its target kind and a quoted word is one word"
  verify unit "a prompt renders the same messages and payload on the command line and over MCP"
  verify unit "completions and help list an extension's prompts beside its commands, with their arguments"
  verify unit "a prompt message keeps its lines and tabs, and every other control character is escaped"
  verify unit "an unterminated quote is refused, a quoted word is a value, --name=value and -- are read, and null or an empty string is an unfilled optional"
  verify unit "specforge prompt refuses an unknown prompt, an unknown host or format and an option without its value, rendering nothing"
  verify unit "prompts/get of a prompt whose gate fails is a JSON-RPC error naming gate_failed, and its export is not called"
  verify unit "specforge.prompt refuses an unknown host, an unknown prompt (naming the served ones) and argv for a core prompt"
  verify unit "a prompt's multi-line message reaches the agent with its lines, and other controls escaped"
  verify unit "a prompt reads the state the host sends it"
  verify unit "a rendered step names the agent that runs it, and a step without one has no agent key"
  verify unit "a command step's cli names the host the prompt is rendered for"
  verify unit "an mcp prompt takes its own path argument through specforge.prompt, and prompts/get reads path as the project"
  verify unit "a prompt's free-text tail takes the words no option takes, joined by one space"
  verify unit "a prompt's tail given by name and by words is refused, an unknown option is not tail, and after -- every word is tail"
  verify unit "a named (MCP) tail is an ordinary string"
  verify unit "a prompt argument that is not a tail serializes as it did, and a tail round-trips"
  verify unit "the SDK declares a prompt's free-text tail as a string argument"
  verify unit "a prompt's own path and format arguments reach it on the command line"
  verify unit "the host reads its options before the first double dash only"
  verify unit "the fallback line an install writes leaves the host's options in the text to the prompt"
  verify unit "a path in the prompt's text never moves the project, and a leading path names it"
  verify unit "a prompt with an optional subject and none given renders for the subject its export picks"
  verify unit "a picked subject meets the prompt's gate, and a given subject wins over the pick"
  verify unit "a picked subject must be an entity of the subject argument's kind, else the answer is E028"
  verify unit "a prompt with an optional subject and no pick is project-wide, and the first word is the subject"
  verify unit "a prompt answer without a subject serializes as it did, and a picked subject round-trips"
  verify unit "a prompt that reads the slash commands is sent the ones the project serves, as an install writes them"
  verify unit "a prompt that does not ask for the slash commands is sent none"
}

behavior provide_mcp_context_prompt "Provide MCP Context Prompt" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpPromptDescriptor, Graph, McpContextPromptResult, McpContextConstraint]
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
    obligations as verification expectations (as the graph export writes them), and related entities.
    structural_constraints entities are included as additional context even if not
    directly connected. A structural constraint that names no entity is
    refused as a missing entity is everywhere (E003, with the closest
    entity), naming structural_constraints. If the entity does not exist,
    the prompt MUST return an error.
  """
  verify unit "specforge://prompts/context returns structured entity context"
  verify unit "response includes contract and related entities"
  verify unit "context includes every field, like an invariant's guarantee"
  verify unit "the context prompt carries the entity's methods and its fields' annotations, a structural constraint's too"
  verify unit "non-existent entity returns error"
  verify unit "context prompt works with zero extensions installed"
  verify contract "Provide MCP Context Prompt: MCP context prompt holds — graph_available, context_returned, hints_included, prompt_invoked_emitted"
}

behavior provide_mcp_review_prompt "Provide MCP Review Prompt" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpPromptDescriptor, McpCoverageResult, McpReviewPromptResult, McpReviewFinding]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    coverage_analysis_returned "Coverage analysis returned for entity and neighbors up to specified depth"
    gaps_identified            "Entities that declare no obligation, uncovered verify declarations, missing evidence links and unconnected entities identified"
    prompt_invoked_emitted     "mcp_prompt_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge://prompts/review
    prompt that accepts entity_id? (optional; the whole graph when omitted)
    and depth? (optional count, default 1). It renders the review
    (review_coverage_gaps) of its arguments: entity_id ("*" for the whole
    graph), findings (entity_id, severity, message) and coverage_summary
    (the coverage view's rows, as specforge.coverage lists them), after an
    instruction to prioritize uncovered and unconnected entities.
  """
  verify unit "specforge://prompts/review returns coverage analysis"
  verify unit "review coverage matches specforge.coverage obligation by obligation"
  verify contract "Provide MCP Review Prompt: MCP review prompt holds — graph_available, coverage_analysis_returned, gaps_identified, prompt_invoked_emitted"
}

behavior provide_mcp_trace_prompt "Provide MCP Trace Prompt" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpPromptDescriptor, TraceChain, McpTracePromptResult, McpTraceGap]
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
    prompt that accepts plan (an AgentPlan's JSON text, types/graph.spec) or
    entity_id (trace that one entity's chain), not both: both MUST be refused
    as invalid input on plan, as specforge.trace refuses them.
    The prompt MUST perform gap analysis against the current graph, identify
    entities affected by the plan, flag missing traceability links, and return
    identified gaps with deterministic gap context.
    A plan that is not an AgentPlan MUST be refused as invalid input on plan,
    naming every place it departs from the type (validate_agent_plan).
  """
  verify unit "specforge://prompts/trace identifies gaps in plan"
  verify unit "response returns identified gaps with gap context"
  verify unit "affected entities are listed"
  verify unit "malformed plan JSON returns validation error"
  verify unit "unverified entities are the ones the trace reaches that count toward coverage and are not proven"
  verify unit "the trace prompt refuses an entity_id given with a plan, as invalid input on plan"
  verify unit "the trace prompt lists no gap for a field that writes a reference that does not resolve"
  verify contract "Provide MCP Trace Prompt: MCP trace prompt holds — graph_available, gaps_returned, affected_entities_listed, prompt_invoked_emitted"
}

behavior provide_mcp_explore_prompt "Provide MCP Explore Prompt" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpPromptDescriptor, Graph, McpExplorePromptResult, McpRelationshipPath]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    exploration_returned   "Guided exploration returned: starting points, high-connectivity entities, unconnected entities"
    bfs_from_entity        "When entity_id provided, BFS traversal starts from that node"
    prompt_invoked_emitted "mcp_prompt_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge://prompts/explore
    prompt that accepts entity_id? (optional starting point), kind? (optional
    entity kind) and depth? (optional count; unbounded when omitted). It
    renders the exploration (explore_the_graph) of its arguments:
    matching_entities, relationship_paths, starting_points,
    high_connectivity, unconnected and notices, after an instruction to
    start from the most connected entities and investigate unconnected
    ones. If entity_id names no entity, the prompt MUST return an error.
  """
  verify unit "specforge://prompts/explore returns exploration starting points"
  verify unit "unknown entity_id returns error"
  verify contract "Provide MCP Explore Prompt: MCP explore prompt holds — graph_available, exploration_returned, bfs_from_entity, prompt_invoked_emitted"
}
