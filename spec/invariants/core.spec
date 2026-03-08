// Core compiler invariants — guarantees that must always hold

use behaviors/parsing
use behaviors/resolution
use behaviors/validation
use behaviors/error-reporting
use behaviors/graph
use behaviors/incremental
use behaviors/output
use behaviors/output-schema
use behaviors/init
use behaviors/lsp
use behaviors/extensions
use behaviors/mcp-operations
use behaviors/mcp-prompts
use behaviors/mcp-server
use behaviors/mcp-tools
use behaviors/migration
use behaviors/formatting

invariant spec_root_singleton "Spec Root Singleton" {
  guarantee """
    Exactly one project root MUST exist per project, identified by
    find_project_root() which locates specforge.json. The specforge.json
    file declares project identity (name, version), installed extensions,
    and provider configurations.
  """
  enforced_by [parse_spec_file_to_ast, link_entity_references, scaffold_new_project, interactive_extension_selection, non_interactive_init, add_extension_to_existing_project, graceful_zero_extension_init, find_project_root, provide_mcp_init_tool]
  risk medium

  verify property "a project with exactly one specforge.json is accepted"

}

invariant init_config_validity "Init Config Validity" {
  guarantee """
    Any specforge.json produced by specforge init or modified by
    specforge add MUST be syntactically valid JSON and semantically
    valid per the SpecForgeJsonConfig schema. The generated config
    MUST be parseable by the compiler pipeline without errors. This
    invariant ensures the seconds-to-value principle: init followed
    by check MUST never fail due to malformed configuration.
  """
  enforced_by [scaffold_new_project, scaffold_starter_spec_file, non_interactive_init, interactive_extension_selection, add_extension_to_existing_project, graceful_zero_extension_init, provide_mcp_init_tool, provide_mcp_add_extension_tool, provide_mcp_remove_extension_tool]
  risk high

  verify property "specforge init output is always valid SpecForgeJsonConfig JSON"
  verify unit "specforge init followed by specforge check produces zero config errors"

}

invariant multi_error_collection "Multi-Error Collection" {
  guarantee """
    The compiler MUST collect and report all diagnostics found during a
    compilation pass. It MUST NOT halt on the first error. Every error,
    warning, and info diagnostic MUST be emitted to the user.
  """
  // detect_dangling_references is a post-resolution integrity check (detects
  // resolver bugs), not a user-facing diagnostic emitter — E001 is emitted
  // by link_entity_references during resolution.
  enforced_by [link_entity_references, format_diagnostics_with_source_context, recover_from_syntax_errors, parse_spec_file_to_ast, parse_all_block_types, parse_triple_quoted_strings, parse_verify_statements, parse_ref_blocks, parse_define_blocks, aggregate_diagnostic_summary, emit_incremental_diagnostics, check_mode_for_ci, print_diagnostics_structured, export_diagnostics_as_json, exit_code_reflects_diagnostic_severity, live_diagnostics, resolve_registry_source, search_registry, publish_to_registry, detect_format_version_mismatch, migrate_spec_files_in_place, validate_post_migration_integrity, invoke_extension_migration_hooks, provide_mcp_validate_tool, authenticate_registry_request, retry_registry_request]
  risk high

  verify property "a file with N errors produces exactly N error diagnostics in one pass"
  verify unit "the compiler does not halt after the first error"

}

invariant string_interning_consistency "String Interning Consistency" {
  guarantee """
    All interned strings MUST resolve to the same symbol for the same input
    within a compilation session. Two equal strings MUST produce the same
    interned key. Comparison by interned key MUST be equivalent to
    comparison by string value.
  """
  enforced_by [parse_spec_file_to_ast, parse_use_imports, parse_all_block_types, parse_triple_quoted_strings, parse_verify_statements, parse_ref_blocks, parse_define_blocks, detect_duplicate_entity_ids, link_entity_references, build_in_memory_graph]
  risk high

  verify property "interning the same string twice returns the same key"
  verify unit "comparison by interned key is equivalent to comparison by string value"

}

invariant import_dag "Import DAG" {
  guarantee """
    The use import graph MUST form a directed acyclic graph (DAG).
    The compiler MUST detect and reject circular imports with an E003
    diagnostic that names the cycle participants.
  """
  enforced_by [detect_import_cycles, resolve_use_imports, parse_use_imports, track_import_dag_incrementally]
  risk medium

  verify property "an acyclic import graph is accepted without diagnostics"
  verify unit "a circular import produces E003 naming the cycle participants"

}

invariant entity_id_uniqueness "Entity ID Uniqueness" {
  guarantee """
    Every entity ID raw string MUST be globally unique across all .spec files
    in a project, regardless of entity kind. Two entities with different kinds
    but the same raw ID are forbidden. The compiler MUST reject duplicate IDs
    with an E002 diagnostic that names both declaration sites.
  """
  enforced_by [detect_duplicate_entity_ids, rename_entity_id, prepare_rename, link_entity_references, custom_entity_types_via_define, provide_mcp_rename_tool, build_in_memory_graph]
  risk high

  verify property "all unique entity IDs across files are accepted"
  verify unit "a duplicate entity ID produces E002 naming both declaration sites"

}

invariant graph_traversal_integrity "Graph Traversal Integrity" {
  guarantee """
    Graph traversal operations (trace, subgraph extraction, delta
    computation) MUST produce complete and deterministic results. Every
    reachable node along a traversal path MUST be included. Traversal
    order MUST be deterministic for identical graph inputs.
  """

  enforced_by [compute_traceability_chain, serialize_traceability_data, validate_agent_plan, query_graph_multi_resolution, export_agent_context_format, export_agent_brief_format, export_agent_graph_format, embed_schema_in_export, serve_schema_resource, serve_graph_resource, compute_subgraph_for_invalidation, compute_graph_delta, expose_graph_as_mcp_resource, notify_graph_delta_via_mcp, provide_mcp_query_tool, provide_mcp_export_tool, provide_mcp_trace_tool, validate_post_migration_integrity, verify_graph_protocol_compatibility_after_migration, serialize_json_graph, serialize_dot_visualization, enforce_token_budget, rebuild_affected_subgraph, expose_context_as_mcp_resource, expose_brief_as_mcp_resource, expose_entity_as_mcp_resource, provide_mcp_search_tool, provide_mcp_coverage_tool, provide_mcp_stats_tool, provide_mcp_inspect_tool, provide_mcp_find_definition_tool, provide_mcp_find_references_tool, provide_mcp_outline_tool, provide_mcp_rename_tool, provide_mcp_render_tool, provide_mcp_context_prompt, provide_mcp_review_prompt, provide_mcp_trace_prompt, provide_mcp_explore_prompt, provide_mcp_suggest_fixes_tool, maintain_mutable_graph, invalidate_changed_files]
  risk high

  verify property "traversal from any node visits every reachable node exactly once"
  verify unit "identical graph inputs produce identical traversal results"
}

invariant incremental_correctness "Incremental Correctness" {
  guarantee """
    After an incremental recompilation triggered by a file change, the
    in-memory graph MUST be identical to the graph produced by a full
    cold rebuild of the same source files. No stale nodes or edges
    MUST remain from the previous compilation.
  """
  enforced_by [watch_file_system_for_changes, debounce_file_changes, rebuild_affected_subgraph, invalidate_changed_files, track_import_dag_incrementally, compute_subgraph_for_invalidation, maintain_mutable_graph, emit_incremental_diagnostics, shared_incremental_pipeline, incremental_document_sync, compute_graph_delta, dispatch_incremental_validators, validate_delta_correctness, notify_delta_subscribers, lsp_shutdown, document_open_close, notify_graph_delta_via_mcp, notify_diagnostics_delta_via_mcp, handle_text_document_change]
  risk high

  verify property "incremental recompilation produces the same graph as a full rebuild"
  verify unit "no stale nodes or edges remain after incremental recompilation"

}

invariant graph_delta_determinism "Graph Delta Determinism" {
  guarantee """
    Given identical previous and current graph states, compute_graph_delta
    MUST produce an identical GraphDelta. All arrays in GraphDelta
    (added_nodes, removed_nodes, modified_nodes, added_edges, removed_edges)
    MUST be sorted by EntityId.raw (lexicographic). The ordering MUST NOT
    depend on hash-map iteration order, filesystem order, or any
    non-deterministic source.
  """
  enforced_by [compute_graph_delta, validate_delta_correctness]
  risk medium

  verify property "identical graph pairs produce identical GraphDelta across 100 runs"
  verify unit "GraphDelta arrays are sorted by EntityId.raw"
}

invariant graph_schema_completeness "Graph Schema Completeness" {
  guarantee """
    The schema section of any Graph Protocol export MUST include every
    entity kind and edge type registered in the KindRegistry and edge type
    set at the time of export. No registered kind or edge type MUST be
    omitted. The schema MUST accurately reflect the testable, singleton,
    and field definitions for each kind.
  """
  enforced_by [generate_schema_from_registries, embed_schema_in_export, persist_schema_cache, serve_schema_resource, serve_graph_resource, serialize_json_graph, export_agent_context_format, export_agent_brief_format, export_agent_graph_format, query_graph_multi_resolution, negotiate_schema_version, detect_breaking_schema_changes, publish_schema_specification, expose_graph_as_mcp_resource, expose_schema_as_mcp_resource, expose_context_as_mcp_resource, expose_brief_as_mcp_resource, expose_entity_as_mcp_resource, provide_mcp_export_tool, provide_mcp_schema_tool, verify_graph_protocol_compatibility_after_migration, compute_schema_version]
  risk medium

  verify property "schema contains every registered kind and edge type"
  verify unit "newly registered extension kind appears in schema"

}

invariant schema_version_backward_compatibility "Schema Version Backward Compatibility" {
  guarantee """
    The previous major version of the Graph Protocol schema MUST remain
    readable by the current compiler. Breaking changes to the schema MUST
    only occur on major version increments. The compiler MUST be capable of
    loading and interpreting Graph Protocol JSON produced by any version
    within the same major version range.
  """
  enforced_by [negotiate_schema_version, detect_breaking_schema_changes, embed_schema_in_export, persist_schema_cache, compute_schema_version]
  risk high

  verify property "Graph Protocol JSON from previous major version is readable"
  verify unit "breaking change on minor version increment is rejected"

}

invariant watch_mode_response_latency "Watch Mode Response Latency" {
  guarantee """
    File-change-to-diagnostics in watch mode MUST complete within 100ms
    for single-file changes.
  """
  enforced_by [emit_incremental_diagnostics, debounce_file_changes, watch_file_system_for_changes, shared_incremental_pipeline]
  risk medium

  verify performance "single-file change produces diagnostics within 100ms"

}

invariant token_budget_subgraph_consistency "Token Budget Subgraph Consistency" {
  guarantee """
    When enforce_token_budget truncates entities from export output, the
    remaining subgraph MUST NOT contain dangling edges referencing truncated
    nodes. Truncated entity IDs MUST be listed in the truncated_entities
    response field.
  """
  enforced_by [enforce_token_budget]
  risk medium

  verify property "truncated subgraph contains no dangling edges"
  verify unit "truncated_entities field lists all omitted entity IDs"

}

invariant query_file_grammar_consistency "Query File Grammar Consistency" {
  guarantee """
    All .scm query files (highlights.scm, folds.scm, indents.scm) MUST
    remain valid and consistent with the tree-sitter grammar after any
    grammar change. Node names, capture names, and pattern structures in
    query files MUST reference grammar rules that exist in the current
    grammar version. A grammar change that adds, removes, or renames a
    rule MUST trigger review of all .scm files for broken references.
  """
  enforced_by [provide_syntax_highlighting_queries, provide_code_folding_queries, provide_indentation_queries]
  risk medium

  verify integration "highlights.scm loads without error against current grammar"
  verify integration "folds.scm loads without error against current grammar"
  verify integration "indents.scm loads without error against current grammar"

}

invariant dry_run_side_effect_freedom "Dry-Run Side-Effect Freedom" {
  guarantee """
    Any command invoked with --dry-run or --check MUST NOT modify any files
    on disk. These flags guarantee read-only execution. If a code path
    reachable from a dry-run context writes to the filesystem, it is a P0
    bug. This applies to specforge migrate --dry-run, specforge format --check,
    specforge format --diff, and any future commands that support dry-run mode.
  """
  enforced_by [generate_migration_diff, check_formatting, show_formatting_diff, provide_mcp_format_tool, provide_mcp_rename_tool, provide_mcp_migrate_tool]
  risk high

  verify unit "--dry-run produces output without modifying files"
  verify unit "--check produces output without modifying files"
  verify property "no file write operations occur during dry-run execution"

}

invariant source_span_completeness "Source Span Completeness" {
  guarantee """
    Every AST node produced by the parser MUST carry a valid SourceSpan
    with start and end positions that accurately reflect the original
    source text. No AST node MUST have a zero-length span unless it
    represents a synthetic node inserted by error recovery.
  """
  enforced_by [parse_spec_file_to_ast, recover_from_syntax_errors, parse_all_block_types, parse_verify_statements, parse_ref_blocks, parse_define_blocks, go_to_definition, find_all_references, rename_entity_id]
  risk high

  verify property "all AST nodes have non-zero source spans"
  verify unit "source spans survive error recovery"

}

// Embedding invariants moved to spec/extensions/embeddings/invariants.spec
