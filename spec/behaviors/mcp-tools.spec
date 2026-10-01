// MCP Tool behaviors — Core tools and navigation tools
//
// 15 behaviors:
//   - Core Tools (9): query, validate, export, trace, search, schema, coverage, stats, analyze
//   - Navigation Tools (5): inspect, find_definition, find_references, outline, suggest_fixes
//   - Dynamic kinds (1): specforge.list and the entities-by-kind resource

use "events/mcp"
use "invariants/core"
use "invariants/mcp"
use "invariants/validation"
use "ports/inbound"
use "ports/outbound"
use "types/diagnostics"
use "types/graph"
use "types/mcp"
use "types/output"

// ---------------------------------------------------------------------------
// Core Tools
// ---------------------------------------------------------------------------

behavior provide_mcp_query_tool "Provide MCP Query Tool" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [Graph, AgentExportConfig, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    subgraph_returned      "Subgraph rooted at entityId returned up to requested depth"
    unknown_kinds_reported "Unknown kind values silently filtered with I-level diagnostic in metadata"
    tool_invoked_emitted   "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.query tool that
    accepts entityId (required), depth? (optional integer), kind?[] (optional
    array of entity kinds to include), include_coverage? (optional boolean),
    and format? (optional: graph|context|brief). The tool MUST return the
    subgraph rooted at the specified entity up to the requested depth. When
    kind is specified, only nodes matching those kinds MUST be included.
    Unknown kind values in the kind[] array MUST be silently filtered out
    and an I-level diagnostic MUST be included in the response metadata
    listing the unrecognized kinds. When format is specified, the output
    MUST use that serialization format. If entityId does not exist, the
    tool MUST return an error response.
  """
  verify unit "specforge.query tool returns subgraph for valid entityId"
  verify unit "depth parameter limits traversal depth"
  verify unit "kind filter restricts returned node types"
  verify unit "format parameter changes output serialization"
  verify unit "non-existent entityId returns error response"
  verify unit "include_coverage parameter includes coverage status in response"
  verify contract "Provide MCP Query Tool: MCP query tool holds — graph_available, subgraph_returned, unknown_kinds_reported, tool_invoked_emitted"
  verify unit "unknown tool returns error"
}

// Idempotency here means result equivalence: the same input always produces
// the same output. It does NOT imply execution caching — each invocation
// performs a full compilation pass.
behavior provide_mcp_validate_tool "Provide MCP Validate Tool" {
  invariants [
    multi_error_collection,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   validation
  types      [DiagnosticBag, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    compiler_api_available "CompilerApi port is available for triggering compilation"
  }
  ensures {
    diagnostics_returned      "All diagnostics matching filter returned with severity, message, file path, and line number"
    strict_promotion_enforced "When strict is true, warnings promoted to errors in response"
    tool_invoked_emitted      "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.validate tool
    that triggers a full compilation and returns validation results as Graph
    Protocol diagnostics. The tool accepts severity_filter? (optional: error,
    warning, info), strict? (optional boolean, treat warnings as errors),
    lint? (optional list of lint profiles, as specforge check --lint takes),
    and use_cached? (optional boolean, default false). When use_cached is
    true and the MCP server has a warm compilation, the tool MUST return
    existing diagnostics without recompilation.
    The response MUST include all diagnostics matching the filter with their
    severity, message, file path, and line number. When strict is true,
    warnings MUST be promoted to errors in the response.

    Cache semantics: use_cached=true returns stale results if no compilation
    has occurred since the last invocation. On cold start (no prior compilation),
    use_cached=true MUST trigger a fresh compilation — it MUST NOT return an
    empty result. Cache is invalidated on any file change detected by the
    file watcher.
  """
  verify unit "specforge.validate tool triggers compilation"
  verify unit "response includes all diagnostics as Graph Protocol diagnostics"
  verify unit "severity_filter restricts returned diagnostics"
  verify unit "strict mode promotes warnings to errors"
  verify integration "validate with lint profiles reports what specforge check reports with the same profiles"
  verify unit "validate with use_cached=false triggers fresh compilation"
  verify unit "validate with use_cached=true returns existing diagnostics without recompilation"
  verify contract "Provide MCP Validate Tool: MCP validate tool holds — compiler_api_available, diagnostics_returned, strict_promotion_enforced, tool_invoked_emitted"
}

behavior provide_mcp_export_tool "Provide MCP Export Tool" {
  invariants [
    graph_traversal_integrity,
    graph_schema_completeness,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [Graph, AgentExportConfig, OutputFile, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    format_produced       "Graph returned in requested agent-optimized format conforming to Graph Protocol schema"
    token_budget_enforced "When max_tokens specified, output truncated to fit budget prioritizing high-connectivity nodes"
    tool_invoked_emitted  "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.export tool that
    accepts format (required: context|brief|graph), scope? (optional entityId
    to restrict to subgraph), max_tokens? (optional integer token budget),
    with_schema? and no_schema? (optional booleans). The tool MUST return the
    graph in the requested agent-optimized format. When max_tokens is
    specified, the output MUST be truncated to fit within the budget,
    prioritizing high-connectivity nodes. The output MUST conform to the
    Graph Protocol schema. The tool MUST produce the export specforge export
    produces for the same arguments, through the same function and schema
    policy: a full graph export is Graph Protocol 2.0 with the schema
    embedded, a scoped one carries a schema_ref, and context, brief and a
    budgeted export leave the schema out unless with_schema is true;
    no_schema leaves it out of a graph export (format 1.0).
  """
  verify unit "specforge.export tool returns graph in requested format"
  verify unit "scope parameter restricts to subgraph"
  verify unit "max_tokens truncates output to fit token budget"
  verify unit "all three formats (context, brief, graph) supported"
  verify contract "Provide MCP Export Tool: MCP export tool holds — graph_available, format_produced, token_budget_enforced, tool_invoked_emitted"
  verify unit "unknown format returns error"
  verify integration "the graph export is the document specforge export --format graph writes, Graph Protocol 2.0 with the schema embedded"
  verify integration "with_schema embeds the schema in a context, brief or budgeted export, and no_schema leaves it out of a graph export"
}

behavior provide_mcp_trace_tool "Provide MCP Trace Tool" {
  invariants [
    graph_traversal_integrity,
    reference_resolution_completeness,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [Graph, TraceChain, TraceLink, McpTracePlanResult, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    trace_result_returned "TraceChain or McpTracePlanResult returned depending on input parameter"
    gaps_identified       "Missing traceability links flagged in trace output"
    tool_invoked_emitted  "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.trace tool that
    accepts entityId? (optional) and plan? (optional inline JSON). When entityId
    is provided, the tool MUST delegate to compute_traceability_chain to traverse
    the graph upstream and downstream from the entity and return a TraceChain
    with TraceLink entries and gap indicators. When plan is provided, the tool
    MUST perform gap analysis against the graph and return a McpTracePlanResult
    containing affected entities, gaps, and suggestions. At least one of entityId
    or plan MUST be provided; otherwise the tool MUST return an error.
  """
  verify unit "specforge.trace tool returns traceability chain for valid entityId"
  verify unit "plan parameter triggers gap analysis"
  verify unit "trace without entity_id or plan returns error"
  verify unit "non-existent entityId returns error response"
  verify unit "response includes upstream and downstream links"
  verify unit "missing links flagged in trace output"
  verify contract "Provide MCP Trace Tool: MCP trace tool holds — graph_available, trace_result_returned, gaps_identified, tool_invoked_emitted"
}

behavior provide_mcp_search_tool "Provide MCP Search Tool" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [Graph, McpSearchResult, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    filtered_results_returned "Search results returned combining all filters with AND semantics"
    unknown_kinds_reported    "Unknown kind values silently filtered with I-level diagnostic in metadata"
    tool_invoked_emitted      "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.search tool that
    accepts kind?[] (entity kind filter), field? (field name), value? (field
    value), references? (entity_id to find referencing entities), text? (fuzzy
    text search across names and string fields), and limit? (max results, default
    50). The tool MUST combine these filters with AND semantics. Unknown
    kind values in the kind[] array MUST be silently filtered out and an
    I-level diagnostic MUST be included in the response metadata listing
    the unrecognized kinds. Fuzzy text search MUST use the same algorithm
    as LSP workspaceSymbol. An empty query with no filters MUST return
    all entities up to the limit.
  """
  verify unit "text search finds entities matching by name or contract"
  verify unit "kind filter restricts results to matching entity kinds"
  verify unit "field and value filter matches entity fields"
  verify unit "limit caps the number of returned results"
  verify unit "empty query returns all entities up to limit"
  verify unit "references filter returns entities referencing target"
  verify contract "Provide MCP Search Tool: MCP search tool holds — graph_available, filtered_results_returned, unknown_kinds_reported, tool_invoked_emitted"
  verify unit "missing query returns error"
}

behavior provide_mcp_schema_tool "Provide MCP Schema Tool" {
  invariants [
    graph_schema_completeness,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [GraphProtocolSchema, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    schema_returned      "GraphProtocolSchema returned, optionally filtered by kind"
    tool_invoked_emitted "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.schema tool that
    accepts kind? (optional entity kind), include_edges? (optional boolean,
    default true), and include_validation_rules? (optional boolean, default
    false). The tool MUST return the GraphProtocolSchema, optionally filtered
    to a single entity kind. When include_edges is false, edge type definitions
    MUST be omitted. When include_validation_rules is true, the response MUST
    include declared validation rules from loaded extensions.
  """
  verify unit "specforge.schema returns full GraphProtocolSchema"
  verify unit "kind filter restricts schema to single entity kind"
  verify unit "include_edges false omits edge type definitions"
  verify unit "include_validation_rules true includes validation rules"
  verify contract "Provide MCP Schema Tool: MCP schema tool holds — graph_available, schema_returned, tool_invoked_emitted"
}

behavior provide_mcp_coverage_tool "Provide MCP Coverage Tool" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
    testable_entity_classification,
  ]
  category   query
  types      [McpCoverageResult, McpToolDescriptor, CoverageStatus]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    coverage_returned     "Coverage status per entity returned including verify count and evidence count"
    testability_respected "Testability determined by extension manifests, not hardcoded"
    tool_invoked_emitted  "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.coverage tool
    that accepts entity_id? (optional, single entity), kind? (optional, filter
    by entity kind), and status_filter? (optional: covered|uncovered|partial).
    Status values are structurally computed from the entity's verify declarations
    and collected evidence — no extension input is required to determine coverage
    status (P2). An obligation is proven when a passing recorded test names its
    verify text, the rule analyze coverage applies: an entity is covered only
    when every obligation is proven and no recorded test fails, so it never
    reports covered while analyze reports A015 or A014. Each result MUST carry
    the obligation count, the proven count, and the unproven verify texts. A verify
    property obligation counts as proven only through a passing test, as in
    analyze coverage without --prove: the SMT discharge --prove adds is not
    run per call. The tool MUST return coverage status per entity including verify count,
    linked evidence count, and evidence status from specforge-report.json if available.
    A specforge-report.json that exists but cannot be read or parsed MUST be
    an error, an isError result carrying an McpError, as the CLI refuses it;
    it is never read as a project with no recorded tests.
    When no filters are provided, the tool MUST return coverage for all
    testable entities. Testability is determined by extension manifests.
  """
  verify unit "specforge.coverage returns coverage for all testable entities"
  verify unit "entity_id filter returns single entity coverage"
  verify unit "kind filter restricts to matching entity kinds"
  verify unit "status_filter restricts to matching coverage status"
  verify unit "an entity with an unproven obligation is partial, not covered"
  verify unit "a failing recorded test keeps an entity from being covered"
  verify unit "a field named verify does not hide an entity's verify statements"
  verify unit "a malformed specforge-report.json is an error result, not an empty report"
  verify integration "specforge.coverage reports covered exactly for the entities analyze coverage proves"
  verify contract "Provide MCP Coverage Tool: MCP coverage tool holds — graph_available, coverage_returned, testability_respected, tool_invoked_emitted"
}

behavior provide_mcp_stats_tool "Provide MCP Stats Tool" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpStatsResult, McpToolDescriptor, McpEntityCount, McpDiagnosticSummary]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    stats_returned         "Aggregate statistics returned: entity counts, edge count, coverage, orphans, diagnostics"
    latest_state_reflected "Response reflects the latest compilation state"
    tool_invoked_emitted   "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.stats tool with
    no required parameters. The tool MUST return aggregate statistics about the
    current graph: entity counts by kind, total edge count, the declared
    percentage (declared_pct; coverage_pct is its deprecated alias), the proof
    percentage (proof_pct, null without recorded test results), orphan node
    count, and a diagnostic summary (counts by severity). A
    specforge-report.json that exists but cannot be read is an error result. The
    response MUST reflect the latest compilation state.
  """
  verify unit "specforge.stats returns entity counts by kind"
  verify unit "response includes coverage percentage"
  verify integration "response includes the declared and proof percentages"
  verify unit "response includes orphan node count"
  verify unit "response includes diagnostic summary by severity"
  verify contract "Provide MCP Stats Tool: MCP stats tool holds — graph_available, stats_returned, latest_state_reflected, tool_invoked_emitted"
}

// ---------------------------------------------------------------------------
// Navigation Tools
// ---------------------------------------------------------------------------

behavior provide_mcp_inspect_tool "Provide MCP Inspect Tool" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
    testable_entity_classification,
  ]
  category   query
  types      [McpInspectResult, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    entity_details_returned "Full entity details returned: kind, fields, contract, references, verify, coverage, diagnostics"
    tool_invoked_emitted    "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.inspect tool that
    accepts entity_id (required). The tool MUST return full entity details
    including kind, fields, contract text, references, verify declarations,
    coverage status, and related diagnostics. The fields MUST include every
    field the entity declares, whatever its kind names them (an invariant's
    guarantee, a decision's rationale), not just contract. The coverage status
    MUST count the recorded test results in specforge-report.json exactly as
    specforge.coverage does. The related diagnostics are those whose span
    lies within the entity's source span, or, for a diagnostic without a
    span, whose message names the entity in quotes; an entity whose ID is a
    prefix of another's never collects the other's diagnostics. LSP equivalence: this tool
    mirrors textDocument/hover, providing the same entity detail an IDE shows
    on hover but over the MCP transport. If the entity does not exist, the
    tool MUST return an error response.
  """
  verify unit "specforge.inspect returns full entity details"
  verify unit "response includes references and verify declarations"
  verify unit "non-existent entity returns error response"
  verify unit "response includes every field, like an invariant's guarantee"
  verify unit "coverage status matches specforge.coverage obligation by obligation"
  verify unit "diagnostics are the entity's own, not those of an entity whose ID contains it"
  verify contract "Provide MCP Inspect Tool: MCP inspect tool holds — graph_available, entity_details_returned, tool_invoked_emitted"
}

behavior provide_mcp_find_definition_tool "Provide MCP Find Definition Tool" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpDefinitionResult, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    source_location_returned "Source location returned including file path, line number, and column"
    tool_invoked_emitted     "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.find_definition
    tool that accepts entity_id (required). The tool MUST return the source
    location where the entity is defined, including file path, line number,
    and column. LSP equivalence: this tool mirrors textDocument/definition
    (gotoDefinition), returning the same source location an IDE navigates to
    but over the MCP transport. If the entity does not exist, the tool MUST
    return an error.
  """
  verify unit "specforge.find_definition returns file, line, and column"
  verify unit "non-existent entity returns error response"
  verify contract "Provide MCP Find Definition Tool: MCP find definition tool holds — graph_available, source_location_returned, tool_invoked_emitted"
}

behavior provide_mcp_find_references_tool "Provide MCP Find References Tool" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpReferenceResult, McpToolDescriptor, McpReferenceLocation]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    references_returned         "All reference locations returned with entity id, file path, line, and column"
    empty_list_for_unreferenced "Entity with no references returns empty list, not an error"
    tool_invoked_emitted        "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.find_references
    tool that accepts entity_id (required). The tool MUST return all locations
    where the entity is referenced, including the referencing entity's id, file
    path, line number, and column. LSP equivalence: this tool mirrors
    textDocument/references (findReferences), returning the same location list
    an IDE shows but over the MCP transport. An entity with no references MUST
    return an empty list, not an error.
  """
  verify unit "specforge.find_references returns all reference locations"
  verify unit "entity with no references returns empty list"
  verify unit "non-existent entity returns error response"
  verify contract "Provide MCP Find References Tool: MCP find references tool holds — graph_available, references_returned, empty_list_for_unreferenced, tool_invoked_emitted"
}

behavior provide_mcp_outline_tool "Provide MCP Outline Tool" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpOutlineEntry, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    outline_returned     "All entities in file returned as McpOutlineEntry with id, kind, name, line range, and children"
    tool_invoked_emitted "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.outline tool that
    accepts file (required). The tool MUST return all entities defined in
    the file as McpOutlineEntry items, including entity id, kind, name, line
    range, and any nested children (an entity's method members). LSP equivalence: this tool mirrors
    textDocument/documentSymbol, returning the same outline structure an IDE
    shows in its symbol navigator but over the MCP transport. If the file
    does not exist, the tool MUST return an error.
  """
  verify unit "specforge.outline returns all entities defined in file"
  verify unit "nested entries included for complex entities"
  verify unit "non-existent file returns error response"
  verify contract "Provide MCP Outline Tool: MCP outline tool holds — graph_available, outline_returned, tool_invoked_emitted"
  verify unit "outline entries sorted by line number"
  verify unit "sorted by line number"
}

behavior provide_mcp_suggest_fixes_tool "Provide MCP Suggest Fixes Tool" {
  invariants [
    graph_traversal_integrity,
    diagnostic_determinism,
    mcp_structured_error_responses,
    mcp_tool_idempotency,
  ]
  category   query
  types      [McpFixSuggestion, McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    fixes_returned       "Applicable fix suggestions returned as McpFixSuggestion items with title, edits, and diagnostic"
    empty_for_clean      "Clean entity with no diagnostics returns empty list"
    tool_invoked_emitted "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.suggest_fixes tool
    that accepts entity_id? (optional), file_path? (optional), and
    diagnostic_code? (optional). When all three parameters are omitted, the
    system MUST return all fix suggestions for the current project. The tool
    MUST return applicable fix suggestions as McpFixSuggestion items, each
    including a title, edit operations, and the diagnostic it resolves. LSP equivalence: this tool mirrors
    textDocument/codeAction, returning the same quick-fix suggestions an IDE
    offers but over the MCP transport. Fix suggestions derive from extension
    validation rules — the core does not hardcode any fix patterns. A clean
    entity with no diagnostics MUST return an empty list.
  """
  verify unit "specforge.suggest_fixes returns applicable fix suggestions"
  verify unit "clean entity with no diagnostics returns empty list"
  verify unit "diagnostic_code filter restricts to matching diagnostics"
  verify contract "Provide MCP Suggest Fixes Tool: MCP suggest fixes tool holds — graph_available, fixes_returned, empty_for_clean, tool_invoked_emitted"
}

behavior provide_mcp_analyze_tool "Provide MCP Analyze Tool" {
  invariants [diagnostic_determinism, mcp_structured_error_responses, mcp_tool_idempotency]
  category   query
  types      [McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    passes_run           "the requested analysis pass, or every pass, runs over the compiled project"
    results_structured   "each pass's findings and summary are returned with an overall ok flag"
    tool_invoked_emitted "mcp_tool_invoked event emitted"
  }
  contract   """
    In MCP server mode, the system MUST register a specforge.analyze tool
    that runs the same analysis passes as `specforge analyze`: the core
    `contracts` pass and every extension-owned pass (such as
    @specforge/testing's coverage), or only the one named by `pass`. It
    accepts `strict` (warnings become errors), `test_results` (a
    specforge-report.json path) and `use_cached` (analyze the last compiled
    graph instead of recompiling). Without `test_results` it MUST read the
    project's own specforge-report.json when one exists, as the CLI does,
    so proof coverage never silently drops. A test report that cannot be read
    or parsed, the project's own or the one `test_results` names, MUST be an
    isError result carrying an McpError, as the CLI exits 2 on it. The result MUST list each pass with its
    findings and summary, plus an `ok` flag that is false when any finding
    is an error. The tool does not run the prove pass, so extension passes
    MUST receive no proved claims, as `specforge analyze` without --prove.
  """
  verify unit "analyze reads the project's specforge-report.json by default"
  verify unit "a malformed test report is an error result"
  verify unit "extension passes receive no proved claims, as specforge analyze without --prove"
  verify contract "Provide MCP Analyze Tool: MCP analyze tool holds — graph_available, passes_run, results_structured, tool_invoked_emitted"
}

behavior provide_mcp_entities_by_kind "List Entities by Kind over MCP" {
  invariants [graph_traversal_integrity, mcp_tool_idempotency]
  category   query
  types      [McpToolDescriptor]
  ports      [McpProtocol, CompilerApi]
  produces   [mcp_tool_invoked]
  requires {
    graph_available "Compiled graph is available via CompilerApi"
  }
  ensures {
    tool_lists_by_kind     "specforge.list returns the entities of a kind, or all entities without one"
    resource_lists_by_kind "specforge://entities/{kind} returns the entities of that kind"
    unknown_kind_empty     "an unknown kind yields an empty list, not an error"
  }
  contract   """
    Because entity kinds come from extensions, MCP clients need a way to
    enumerate entities without knowing the kinds in advance. The system
    MUST register a specforge.list tool (optional `kind`) and a
    specforge://entities/{kind} resource template. Both MUST return each
    matching entity's id, kind and title. An unknown kind MUST yield an
    empty list.
  """
  verify unit "specforge.list returns entities filtered by kind"
  verify unit "specforge.list returns empty for unknown kind"
  verify unit "entity-by-kind resource returns entities"
  verify unit "specforge.list tool appears in tool list"
  verify unit "entities resource template in resource template list"
}
