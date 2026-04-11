// Shared surface conventions — error handling, format conventions, and cross-cutting surface behaviors
//
// This file specifies cross-cutting contracts that apply to ALL product surface
// contributions (CLI commands, MCP tools, and MCP resources). See surfaces-cli.spec
// for CLI commands and surfaces-mcp.spec for MCP resources.

use "extensions/product/types"
use "extensions/product/behaviors-registration"
use "extensions/product/behaviors-queries"
use "extensions/product/behaviors-operations"
use "extensions/product/features"

// ════════════════════════════════════════════════════════════════
// Surface Error Handling — common error contracts
// ════════════════════════════════════════════════════════════════

behavior surface_error_handling "Surface Error Handling" {
  category   command
  types      [ProductSurfaceError, ProductQueryError]
  contract   """
    All product surface contributions (CLI commands and MCP tools/resources)
    MUST follow consistent error handling:
    1. Entity-not-found: return ProductSurfaceError with code="ENTITY_NOT_FOUND",
       message including the entity kind and ID, and optional suggestion field
       with fuzzy-match (Levenshtein distance <= 2).
    2. Graph-not-ready: return ProductSurfaceError with code="GRAPH_NOT_READY",
       message indicating the graph is rebuilding.
    3. Invalid-input: return ProductSurfaceError with code="INVALID_INPUT",
       message describing the validation failure (e.g., invalid filter value,
       malformed date).
    4. CLI commands write errors to stderr (not stdout) and exit with code 1.
    5. MCP tools return JSON-RPC error responses with the ProductSurfaceError
       as the error data field.
    6. MCP resources return ProductSurfaceResponse with status=error.
  """
  ensures  {
    entity_not_found_code  "entity-not-found errors use code ENTITY_NOT_FOUND"
    graph_not_ready_code   "graph-not-ready errors use code GRAPH_NOT_READY"
    invalid_input_code     "invalid-input errors use code INVALID_INPUT"
    suggestion_on_typo     "ENTITY_NOT_FOUND includes suggestion when Levenshtein distance <= 2 match exists"
    cli_stderr             "CLI error messages are written to stderr, not stdout"
    cli_exit_one           "CLI commands exit with code 1 on any error"
    mcp_tool_jsonrpc       "MCP tools return JSON-RPC error response on error"
    mcp_resource_envelope  "MCP resources return ProductSurfaceResponse with status=error on error"
    no_panic               "no surface contribution panics on any input"
  }

  features [pe_surface_contributions]

  verify unit "entity-not-found returns ENTITY_NOT_FOUND code"
  verify unit "graph-not-ready returns GRAPH_NOT_READY code"
  verify unit "invalid filter value returns INVALID_INPUT code"
  verify unit "CLI errors go to stderr"
  verify unit "MCP tool errors are JSON-RPC error responses"
  verify unit "MCP resource errors use status=error envelope"
  verify unit "fuzzy-match suggestion present when close match exists"
  verify unit "no surface panics on null, empty, or malformed input"
}

// ════════════════════════════════════════════════════════════════
// Surface Format Conventions — shared output formatting
// ════════════════════════════════════════════════════════════════

behavior surface_format_conventions "Surface Format Conventions" {
  category   command
  types      [ProductListFilter]
  contract   """
    All product CLI commands MUST support a --format flag with three values:
    - json (default): machine-readable JSON on stdout, one root object
    - table: human-readable aligned columns on stdout
    - brief: minimal output — newline-delimited entity IDs for list commands,
      single-value output for query commands (e.g., completion ratio as plain number)
    The --format flag is passed through to MCP tools as a "format" field in
    the JSON Schema input. MCP resources always return JSON (no format flag).
  """
  ensures  {
    json_default       "--format defaults to json when omitted"
    json_valid         "json format produces valid JSON parseable by any JSON parser"
    table_aligned      "table format uses fixed-width columns aligned with spaces"
    table_header       "table format includes a header row with column names"
    brief_ids_only     "brief format for list commands outputs one entity ID per line"
    brief_single_value "brief format for query commands outputs a single value (ratio, count, boolean)"
    mcp_always_json    "MCP resources ignore format parameter and always return JSON"
    utf8_output        "all output is valid UTF-8"
  }

  features [pe_surface_contributions]

  verify unit "default format is json"
  verify unit "json output is valid JSON"
  verify unit "table output has header and aligned columns"
  verify unit "brief output for list is one ID per line"
  verify unit "brief output for query is single value"
}

// ════════════════════════════════════════════════════════════════
// Surface List Command Contract — shared contract for all 9 list commands
// ════════════════════════════════════════════════════════════════

behavior surface_list_command_contract "Surface List Command Shared Contract" {
  category   command
  types      [ProductListFilter, ProductListResult, ProductSurfaceError]
  contract   """
    All 9 product list commands (product:features, product:journeys,
    product:deliverables, product:milestones, product:modules, product:terms,
    product:personas, product:channels, product:releases) MUST follow a
    uniform contract:

    Input: ProductListFilter with optional --status, --priority, --tags,
    --limit (default 100, max 1000), --offset (default 0), --sort-by
    (default "id"), --sort-order (default "asc"), and --format (default "json").

    Output: A typed *ListResult (e.g., FeatureListResult, JourneyListResult)
    containing entities[], total, offset, limit, has_more.

    Behavior:
    1. Filter phase: apply --status, --priority, --tags filters (AND logic).
       Invalid enum values produce INVALID_INPUT error. Empty filters match all.
    2. Sort phase: sort by --sort-by field (must exist on entity kind, else
       INVALID_INPUT). Tie-break by entity ID ascending for determinism.
    3. Paginate phase: apply --offset and --limit. Clamp --limit to [1, 1000].
       --offset beyond total returns empty entities[] with correct total.
    4. Serialize phase: apply --format (json|table|brief) per
       surface_format_conventions.

    Each list command delegates to the same query pipeline — only the entity
    kind and result type differ. Wasm export: cmd__product_{kind}s (plural).
    MCP tool auto-promotion: specforge.product.{kind}s.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    filter_and_logic      "multiple filters combine with AND logic"
    invalid_filter_error  "invalid enum filter value returns INVALID_INPUT"
    sort_deterministic    "tie-break sort by entity ID ascending"
    sort_field_validated  "invalid sort_by field returns INVALID_INPUT"
    limit_clamped         "limit clamped to [1, 1000] range"
    offset_beyond_total   "offset beyond total returns empty list with correct total"
    pagination_correct    "total reflects filtered count; has_more == (offset + entities.length < total)"
    empty_graph_ok        "empty graph returns entities=[], total=0, has_more=false"
    delegates_to_query    "each list command delegates to a common query pipeline"
  }

  features [pe_surface_contributions]

  verify unit "filter by status returns only matching entities"
  verify unit "filter by priority returns only matching entities"
  verify unit "combined status+priority filter uses AND logic"
  verify unit "invalid status filter returns INVALID_INPUT"
  verify unit "sort by priority with tie-break by ID is deterministic"
  verify unit "invalid sort_by field returns INVALID_INPUT"
  verify unit "limit=0 is clamped to 1"
  verify unit "limit=5000 is clamped to 1000"
  verify unit "offset beyond total returns empty list"
  verify unit "empty graph returns total=0 and has_more=false"
  verify property "for all list commands: entities.length <= limit"
  verify property "for all list commands: has_more == (offset + entities.length < total)"
}

// ════════════════════════════════════════════════════════════════
// Surface Query Command Contract — shared contract for all query commands
// ════════════════════════════════════════════════════════════════

behavior surface_query_command_contract "Surface Query Command Shared Contract" {
  category   command
  types      [ProductSurfaceError, ProductSurfaceResponse]
  contract   """
    All product query commands (22 original + 9 v1.1 + 5 analytics = 36 total)
    MUST follow a uniform contract:

    Entity-scoped queries (e.g., product:milestone-completion, product:feature-impact):
    1. Accept --id (entity ID, required) and --format flags.
    2. Validate entity exists and is the correct kind. Return ENTITY_NOT_FOUND
       with fuzzy-match suggestion if not found.
    3. Delegate to the corresponding ProductQueryPort method.
    4. Return the typed payload (e.g., MilestoneCompletionPayload).

    Project-wide queries (e.g., product:critical-path):
    1. Accept only --format flag (no --id).
    2. Delegate to the corresponding ProductQueryPort method.
    3. Return the typed payload.

    All query commands:
    - Return GRAPH_NOT_READY if graph is rebuilding.
    - Support --format (json|table|brief) per surface_format_conventions.
    - Are auto-promoted to MCP tools with the same input schema.
    - Emit observability events on completion per events.spec.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    entity_validated       "entity-scoped queries validate entity existence and kind"
    fuzzy_suggestion       "ENTITY_NOT_FOUND includes Levenshtein distance <= 2 suggestion"
    graph_not_ready_error  "queries during rebuild return GRAPH_NOT_READY"
    delegates_to_port      "each command delegates to a ProductQueryPort method"
    observability_event    "each query emits its observability event on completion"
    format_respected       "output respects --format flag"
  }

  features [pe_surface_contributions]

  verify unit "entity-scoped query with valid ID returns typed payload"
  verify unit "entity-scoped query with invalid ID returns ENTITY_NOT_FOUND"
  verify unit "entity-scoped query with close typo returns suggestion"
  verify unit "project-wide query returns typed payload"
  verify unit "query during rebuild returns GRAPH_NOT_READY"
  verify unit "query result respects --format=brief"
}

// ════════════════════════════════════════════════════════════════
// Surface MCP Resource Contract — shared contract for all MCP resources
// ════════════════════════════════════════════════════════════════

behavior surface_mcp_resource_contract "Surface MCP Resource Shared Contract" {
  category   query
  types      [ProductSurfaceResponse, ProductSurfaceError]
  produces  [pe_mcp_resource_accessed]
  contract   """
    All MCP resources (28 total: 13 original + 6 v1.1 + 9 analytics) MUST
    return a ProductSurfaceResponse envelope:
      { "status": "ok"|"error",
        "data": <typed payload>,
        "error": <ProductSurfaceError when status=error>,
        "_resource": "<URI that produced the response>",
        "_timestamp": "<ISO 8601 datetime>" }

    Contract:
    1. URI template parameters map directly to query-port method arguments.
    2. Missing URI parameters produce INVALID_INPUT error.
    3. Entity-scoped resources return ENTITY_NOT_FOUND with suggestion for
       invalid entity IDs.
    4. All resources are read-only (no side effects, no diagnostics emitted).
    5. _timestamp is always present and valid ISO 8601.
    6. _resource is always the URI that was accessed.
    7. Response Content-Type is application/json.
  """
  ensures  {
    envelope_always_valid  "every response is a valid ProductSurfaceResponse"
    timestamp_iso8601      "_timestamp is valid ISO 8601 datetime"
    resource_uri_present   "_resource matches the accessed URI"
    content_type_json      "response Content-Type is application/json"
    read_only_default      "resources are read-only with no side effects"
    not_found_suggestion   "ENTITY_NOT_FOUND includes fuzzy suggestion when match exists"
  }

  features [pe_surface_contributions]

  verify unit "every resource returns valid ProductSurfaceResponse envelope"
  verify unit "_timestamp is valid ISO 8601 in every response"
  verify unit "_resource matches the accessed URI"
  verify unit "missing URI parameter returns INVALID_INPUT"
  verify unit "invalid entity ID returns ENTITY_NOT_FOUND with suggestion"
}
