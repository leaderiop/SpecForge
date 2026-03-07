// Compilation events — signals emitted during the compiler pipeline

use types/core
use types/graph
use types/diagnostics
use behaviors/parsing
use behaviors/resolution
use behaviors/graph
use behaviors/validation
use behaviors/error-reporting
use behaviors/incremental
use behaviors/zero-entity-registries
use behaviors/zero-entity-validation
use behaviors/formatting
use behaviors/output
use behaviors/output-schema
use behaviors/lsp
use behaviors/init
use behaviors/migration
use behaviors/extensions
use behaviors/mcp-server

event file_parsed "File Parsed" {
  trigger   parse_spec_file_to_ast
  channel   "compiler.file_parsed"

  payload {
    filePath   string
    entityCount integer
    errorCount  integer
    timestamp   timestamp
  }

  // recover_from_syntax_errors runs INSIDE the parser (Tree-sitter error
  // recovery), not as a post-parse event consumer.
  consumers []

  verify integration "emits file_parsed with correct entityCount and errorCount after successful parse"

}

event all_files_parsed "All Files Parsed" {
  trigger   parse_spec_file_to_ast
  channel   "compiler.all_files_parsed"

  payload {
    fileCount   integer
    entityCount integer
    timestamp   timestamp
  }

  // Phase 1 -> Extension Loader: registries must be populated before resolution.
  // resolve_use_imports is NOT a consumer here — it runs in Phase 2 after
  // registries_populated.
  // Co-consumer ordering: detect_duplicate_entity_ids runs on raw IDs (no registry
  // needed); load_extension_manifests loads extensions. These are independent —
  // no ordering required between them.
  consumers [detect_duplicate_entity_ids, load_extension_manifests]

  verify integration "emits all_files_parsed after every file in the project has been parsed"
  verify integration "consumer populates the KindRegistry from extension manifests before Phase 2"

}

event extension_manifests_loaded {
  trigger load_extension_manifests
  channel internal

  payload {
    extensionCount  integer
    manifestPaths   string[]
    timestamp       timestamp
  }

  consumers [register_extension_entity_types, load_provider_configurations]

  verify "Extension manifests MUST be loaded before populating the kind registry"
}

event registries_populated "Registries Populated" {
  // Fires after populate_kind_registry, populate_field_registry, and
  // populate_edge_registry all complete. The kind registry populator
  // orchestrates all three and fires this event last.
  trigger   populate_kind_registry_from_extensions
  channel   "compiler.registries_populated"

  payload {
    kindCount    integer
    fieldCount   integer
    edgeCount    integer
    extensionCount integer
    timestamp    timestamp
  }

  // Phase 2 begins: resolution and semantic validation run only after all
  // registries are populated. resolve_use_imports starts reference linking;
  // two_phase_validate_semantic checks keywords against the KindRegistry.
  consumers [resolve_use_imports, two_phase_validate_semantic, validate_extension_testability, resolve_soft_cross_extension_references, generate_schema_from_registries]

  verify integration "emits registries_populated after KindRegistry, FieldRegistry, and edge type set are fully loaded"
  verify integration "consumer resolve_use_imports starts Phase 2 only after this event"
  verify integration "consumer two_phase_validate_semantic begins keyword and field validation after this event"

}

event resolution_complete "Resolution Complete" {
  trigger   link_entity_references
  channel   "compiler.resolution_complete"

  payload {
    fileCount   integer
    entityCount integer
    timestamp   timestamp
  }

  consumers [build_in_memory_graph]

  verify integration "emits resolution_complete after all imports resolved"
  verify integration "consumer build_in_memory_graph receives event and constructs graph"

}

event graph_built "Graph Built" {
  trigger   build_in_memory_graph
  channel   "compiler.graph_built"

  payload {
    nodeCount  integer
    edgeCount  integer
    fileCount  integer
    timestamp  timestamp
  }

  // Core structural validators and the declarative validation engine all
  // run after the graph is built. detect_dangling_references verifies
  // resolver integrity. The remaining checks are core structural checks
  // (orphan refs, gherkin file existence, extension testability flag
  // consistency). execute_validation_pattern runs extension-defined
  // declarative rules.
  consumers [
    detect_dangling_references, execute_validation_pattern,
    detect_orphan_refs, validate_file_reference_paths,
  ]

  verify integration "emits graph_built with accurate nodeCount and edgeCount after graph construction"
  verify integration "consumers receive graph_built and trigger structural validation and declarative pattern execution"

}

event declarative_validation_executed "Declarative Validation Executed" {
  trigger   execute_validation_pattern
  channel   "compiler.declarative_validation_executed"

  payload {
    ruleCount       integer
    violationCount  integer
    extensionCount  integer
    timestamp       timestamp
  }

  consumers [aggregate_diagnostic_summary]

  verify integration "emits declarative_validation_executed with correct ruleCount and violationCount"
  verify integration "consumer aggregate_diagnostic_summary collects pattern diagnostics into final summary"

}

event validation_complete "Validation Complete" {
  trigger   aggregate_diagnostic_summary
  channel   "compiler.validation_complete"

  payload {
    errorCount   integer
    warningCount integer
    infoCount    integer
    timestamp    timestamp
  }

  // Note: format_diagnostics_with_source_context and provide_did_you_mean_suggestions
  // run INLINE during resolution and validation (each diagnostic is formatted as it's
  // emitted). They are NOT post-validation consumers. This event signals pipeline
  // completion — consumers are output-phase behaviors that need the final diagnostic
  // counts, such as CLI exit code decisions and emitter triggers.
  consumers [serialize_json_graph, serialize_dot_visualization, compute_traceability_chain, serialize_traceability_data, validate_agent_plan, export_agent_context_format, export_agent_brief_format, export_agent_graph_format, query_graph_multi_resolution, enforce_token_budget, compute_project_statistics, print_diagnostics_structured, exit_code_reflects_diagnostic_severity, deterministic_output, check_mode_for_ci, export_diagnostics_as_json, negotiate_schema_version, embed_schema_in_export, serve_schema_resource, serve_graph_resource, publish_schema_specification, notify_diagnostics_delta_via_mcp]

  verify integration "emits validation_complete with correct error, warning, and info counts"
  verify integration "event signals Phase 2 completion — emitter phase may begin"

}

event file_changed "File Changed" {
  trigger   [watch_file_system_for_changes, handle_text_document_change, document_open_close]
  channel   "watch.file_changed"

  payload {
    filePath    string
    changeType  string
    timestamp   timestamp
  }

  consumers [debounce_file_changes]

  verify integration "emits file_changed with correct filePath and changeType on filesystem modification"
  verify integration "consumer debounce_file_changes receives event and coalesces into batch"

}

event file_changes_coalesced "File Changes Coalesced" {
  trigger   debounce_file_changes
  channel   "watch.file_changes_coalesced"

  payload {
    filePaths    string[]
    changeCount  integer
    timestamp    timestamp
  }

  consumers [invalidate_changed_files]

  verify integration "emits file_changes_coalesced with correct filePaths after debounce window expires"
  verify integration "consumer invalidate_changed_files receives coalesced batch and computes invalidation set"

}

event subgraph_invalidated "Subgraph Invalidated" {
  trigger   invalidate_changed_files
  channel   "watch.subgraph_invalidated"

  payload {
    invalidatedFiles string[]
    nodeCount        integer
    timestamp        timestamp
  }

  consumers [rebuild_affected_subgraph, track_import_dag_incrementally]

  verify integration "emits subgraph_invalidated with correct invalidatedFiles list and nodeCount"
  verify integration "consumer rebuild_affected_subgraph receives event and re-parses invalidated files"
  verify integration "consumer track_import_dag_incrementally updates file dependency edges"

}

event import_dag_updated "Import DAG Updated" {
  trigger   track_import_dag_incrementally
  channel   "watch.import_dag_updated"

  payload {
    addedEdges      integer
    removedEdges    integer
    cyclesDetected  integer
    timestamp       timestamp
  }

  consumers [rebuild_affected_subgraph]

  verify integration "emits import_dag_updated after file dependency edges are added or removed"
  verify integration "consumer rebuild_affected_subgraph waits for import DAG to be current before rebuilding"

}

event incremental_rebuild_complete "Incremental Rebuild Complete" {
  trigger   rebuild_affected_subgraph
  channel   "watch.incremental_rebuild_complete"

  payload {
    rebuiltFiles     string[]
    addedNodes       integer
    removedNodes     integer
    elapsedMs        integer
    timestamp        timestamp
  }

  consumers [emit_incremental_diagnostics, shared_incremental_pipeline, compute_graph_delta]

  verify integration "emits incremental_rebuild_complete with correct rebuiltFiles and node counts"
  verify integration "consumer emit_incremental_diagnostics refreshes diagnostics for rebuilt files"
  verify integration "consumer shared_incremental_pipeline updates LSP graph state"
  verify integration "consumer compute_graph_delta diffs previous and new graph states to produce GraphDelta"

}

event render_complete "Render Complete" {
  trigger   [serialize_json_graph, serialize_dot_visualization, serialize_traceability_data, publish_schema_specification]
  channel   "compiler.render_complete"

  payload {
    outputFileCount  integer
    format           string
    timestamp        timestamp
  }

  consumers []

  verify integration "emits render_complete with correct outputFileCount after rendering"

}

event export_complete "Export Complete" {
  trigger   [export_agent_context_format, export_agent_brief_format, export_agent_graph_format]
  channel   "compiler.export_complete"

  payload {
    format          string
    entityCount     integer
    scopedToEntity  string
    timestamp       timestamp
  }

  consumers []

  verify integration "emits export_complete with correct format and entityCount after agent export"

}

event format_complete "Format Complete" {
  trigger   [format_spec_files, lsp_format_document, lsp_format_range, format_from_stdin]
  channel   "compiler.format_complete"

  payload {
    filesChecked  integer
    filesChanged  integer
    timestamp     timestamp
  }

  consumers []

  verify integration "emits format_complete with correct filesChecked and filesChanged"

}

event project_initialized "Project Initialized" {
  // Triggers are mutually exclusive — exactly one fires per init invocation depending on the code path.
  // interactive_extension_selection is a sub-step of scaffold_new_project, not an independent trigger.
  trigger   [scaffold_new_project, non_interactive_init, graceful_zero_extension_init]
  channel   "cli.project_initialized"

  payload {
    projectName     string
    extensionCount  integer
    specFilePath    string   // Path to the starter entity spec file created by init (e.g., hello.spec)
    timestamp       timestamp
  }

  consumers []

  verify integration "emits project_initialized with correct projectName and extensionCount"
  verify integration "specFilePath refers to the starter entity spec file, not specforge.json"

}

event extension_added "Extension Added" {
  trigger   add_extension_to_existing_project
  channel   "cli.extension_added"

  payload {
    extensionSpecifier  string
    totalExtensions     integer
    wasDuplicate        boolean
    timestamp           timestamp
  }

  consumers []

  verify integration "emits extension_added with correct extensionSpecifier after specforge add"
  verify unit "wasDuplicate is true when extension was already installed"

}

event extension_loading_failed {
  trigger   [load_extension_manifests, validate_peer_dependencies]
  channel   "compilation.extension_loading_failed"

  payload {
    extension_name string
    error_kind     string
    message        string
  }

  consumers []

  verify integration "manifest parse error emits extension_loading_failed event"

}

event custom_entity_type_defined "Custom Entity Type Defined" {
  // Emitted once per define block. After ALL define blocks are processed,
  // define_blocks_registered fires as the aggregate signal.
  trigger   custom_entity_types_via_define
  channel   "compiler.custom_entity_type_defined"

  payload {
    kindName        string
    fieldCount      integer
    sourceFile      string
    timestamp       timestamp
  }

  consumers []

  verify integration "emits custom_entity_type_defined when a define block registers a custom entity type"
  verify integration "fires after registries_populated (define blocks run in Phase 2)"

}

event define_blocks_registered "Define Blocks Registered" {
  // Fires after all define blocks in the project have been processed and
  // their custom entity kinds registered in the KindRegistry. This event
  // fires after registries_populated (extension kinds) but before resolution
  // begins, ensuring that both extension-defined and project-defined kinds
  // are available for semantic validation.
  trigger   custom_entity_types_via_define
  channel   "compiler.define_blocks_registered"

  payload {
    defineCount     integer
    kindCount       integer
    timestamp       timestamp
  }

  // Resolution and semantic validation may proceed after define blocks are
  // registered, since all entity kinds (extension + project) are now known.
  // Note: two_phase_validate_semantic waits for BOTH registries_populated AND
  // define_blocks_registered before running, ensuring all entity kinds
  // (extension-defined and project-defined) are available for validation.
  consumers [resolve_use_imports, two_phase_validate_semantic]

  verify integration "emits define_blocks_registered after all define blocks processed"
  verify integration "fires after registries_populated event"
  verify integration "kindCount includes both extension-defined and define-block kinds"

}

// ── LSP Lifecycle Events ─────────────────────────────────────

event lsp_initialized "LSP Initialized" {
  trigger   lsp_initialize
  channel   "lsp.initialized"

  payload {
    extensionCount  integer
    kindCount       integer
    timestamp       timestamp
  }

  consumers []

  verify integration "emits lsp_initialized after LSP server completes initialization"

}

event lsp_shutdown_complete "LSP Shutdown Complete" {
  trigger   lsp_shutdown
  channel   "lsp.shutdown_complete"

  payload {
    timestamp       timestamp
  }

  consumers []

  verify integration "emits lsp_shutdown_complete after LSP server shuts down cleanly"

}

// ── Migration Events ──────────────────────────────────────────

// migration_started fires after the backup is created but before transforms run.
// Contrast with migration_starting, which fires before any file I/O begins.
event migration_started "Migration Started" {
  trigger   migrate_spec_files_in_place
  channel   "compiler.migration_started"

  payload {
    fileCount       integer
    targetVersion   string
    timestamp       timestamp
  }

  consumers [rollback_failed_migration]

  verify integration "emits migration_started with correct fileCount and targetVersion"

}

// migration_starting fires before any file I/O begins (pre-backup).
// Contrast with migration_started, which fires after the backup is created.
event migration_starting "Migration Starting" {
  trigger   migrate_spec_files_in_place
  channel   "compiler.migration_starting"

  payload {
    fileCount       integer
    targetVersion   string
    timestamp       timestamp
  }

  consumers [verify_graph_protocol_compatibility_after_migration]

  verify integration "emits migration_starting before any migration transforms are applied"

}

event migration_complete "Migration Complete" {
  trigger   migrate_spec_files_in_place
  channel   "compiler.migration_complete"

  payload {
    migratedCount   integer
    failedCount     integer
    // skippedCount: files already at the target version (idempotency).
    // A skipped file is one whose format_version already matches the
    // migration target — no transforms are applied and no backup is created.
    skippedCount    integer
    timestamp       timestamp
  }

  consumers [validate_post_migration_integrity, verify_graph_protocol_compatibility_after_migration, invoke_extension_migration_hooks]

  verify integration "emits migration_complete with correct migratedCount, failedCount, and skippedCount"
  verify integration "consumer validate_post_migration_integrity receives event"
  verify integration "consumer invoke_extension_migration_hooks receives event to run extension hooks"

}

event migration_diff_generated "Migration Diff Generated" {
  trigger   generate_migration_diff
  channel   "compiler.migration_diff_generated"

  payload {
    fileCount       integer
    totalDiffLines  integer
    timestamp       timestamp
  }

  consumers []

  verify integration "emits migration_diff_generated with correct fileCount and totalDiffLines after dry-run"

}

event extension_migration_hooks_complete "Extension Migration Hooks Complete" {
  trigger   invoke_extension_migration_hooks
  channel   "compiler.extension_migration_hooks_complete"

  payload {
    extensionsInvoked integer
    extensionsFailed  integer
    extensionsSkipped integer
    timestamp         timestamp
  }

  consumers [validate_post_migration_integrity, verify_graph_protocol_compatibility_after_migration]

  verify integration "emits extension_migration_hooks_complete after all extension hooks finish"
  verify integration "consumer validate_post_migration_integrity receives event"

}

event migration_rolled_back "Migration Rolled Back" {
  trigger   rollback_failed_migration
  channel   "compiler.migration_rolled_back"

  payload {
    restoredCount   integer
    skippedCount    integer
    failedCount     integer
    timestamp       timestamp
  }

  consumers []

  verify integration "emits migration_rolled_back with correct restored, skipped, and failed counts"

}

// ── Schema Events ────────────────────────────────────────────

event schema_generated "Schema Generated" {
  trigger   generate_schema_from_registries
  channel   "compiler.schema_generated"

  payload {
    entityKindCount integer
    edgeTypeCount   integer
    extensionCount  integer
    timestamp       timestamp
  }

  consumers [embed_schema_in_export, detect_breaking_schema_changes]

  verify integration "emits schema_generated with correct entityKindCount and edgeTypeCount"
  verify integration "consumer embed_schema_in_export receives event"
  verify integration "consumer detect_breaking_schema_changes receives event"

}

// Entity embedding events moved to spec/extensions/embeddings/events.spec

// ── Agent & Schema Events ─────────────────────────────────────

event plan_validated "Agent Plan Validated" {
  trigger  validate_agent_plan
  payload  PlanValidationResult
  channel  compilation
}

event token_budget_applied "Token Budget Applied" {
  trigger  enforce_token_budget
  payload  ExportResult
  channel  compilation
}

event schema_version_negotiated "Schema Version Negotiated" {
  trigger  negotiate_schema_version
  payload  ExportResult
  channel  compilation
}

// ── Output Query Events ──────────────────────────────────────

event graph_queried "Graph Queried" {
  trigger   query_graph_multi_resolution
  channel   "output"

  payload {
    entityId        string
    scope           string
    depth           integer
    resultNodeCount integer
    timestamp       timestamp
  }

  consumers []

  verify integration "emits graph_queried with correct entityId, scope, and resultNodeCount"
  verify integration "payload includes depth and scope used for the query"

}

// entity_search_performed moved to spec/extensions/embeddings/events.spec

// ── Incremental Pipeline Terminal Events ──────────────────────

event incremental_diagnostics_complete "Incremental Diagnostics Complete" {
  trigger   emit_incremental_diagnostics
  channel   "watch.incremental_diagnostics_complete"

  payload {
    errorCount      integer
    warningCount    integer
    infoCount       integer
    affectedFiles   integer
    timestamp       timestamp
  }

  consumers []

  verify integration "emits incremental_diagnostics_complete after incremental diagnostic pipeline finishes"

}

event delta_subscribers_notified "Delta Subscribers Notified" {
  trigger   notify_delta_subscribers
  channel   "watch.delta_subscribers_notified"

  payload {
    subscriberCount integer
    timestamp       timestamp
  }

  consumers []

  verify integration "emits delta_subscribers_notified after all subscribers receive delta"

}

// ── Rename Events ───────────────────────────────────────────

event entity_renamed "Entity Renamed" {
  trigger   rename_entity_id
  channel   "lsp.entity_renamed"

  payload {
    oldId           string
    newId           string
    affectedFiles   integer
    timestamp       timestamp
  }

  consumers [notify_delta_subscribers]

  verify integration "emits entity_renamed with correct oldId, newId, and affectedFiles"

}

// ── Graph Delta Events ──────────────────────────────────────

event graph_delta_computed "Graph Delta Computed" {
  trigger   compute_graph_delta
  channel   "watch.graph_delta_computed"

  payload {
    addedNodes      integer
    removedNodes    integer
    modifiedNodes   integer
    addedEdges      integer
    removedEdges    integer
    affectedFiles   integer
    timestamp       timestamp
  }

  consumers [dispatch_incremental_validators, notify_delta_subscribers, validate_delta_correctness, emit_incremental_diagnostics, notify_graph_delta_via_mcp]

  verify integration "emits graph_delta_computed with correct node and edge counts"
  verify integration "consumer dispatch_incremental_validators receives delta event"
  verify integration "consumer notify_delta_subscribers receives delta event"
  verify integration "consumer validate_delta_correctness receives delta event in debug mode"

}

event incremental_validators_dispatched "Incremental Validators Dispatched" {
  trigger   dispatch_incremental_validators
  channel   "watch.incremental_validators_dispatched"

  payload {
    incrementalExtensions integer
    fullRebuildExtensions integer
    deltaNodeCount        integer
    timestamp             timestamp
  }

  consumers [emit_incremental_diagnostics]

  verify integration "emits incremental_validators_dispatched with correct extension counts"
  verify integration "payload distinguishes incremental vs full-rebuild extension counts"

}

// ── Delta Validation Events ──────────────────────────────────

event delta_validation_failed "Delta Validation Failed" {
  // Debug-only event — emitted when validate_delta_correctness detects
  // a discrepancy between the incremental and full-rebuild graph states.
  trigger   validate_delta_correctness
  channel   "watch.delta_validation_failed"

  payload {
    inconsistentNodes   integer
    inconsistentEdges   integer
    timestamp           timestamp
  }

  consumers []

  verify unit "emits delta_validation_failed when incremental rebuild diverges from full rebuild"

}

// ── MCP Graph Resource Events ─────────────────────────────────

event graph_resource_served "Graph Resource Served" {
  trigger   serve_graph_resource
  channel   "mcp"

  payload {
    resourceUri     string
    format          string
    entityCount     integer
    scopedToEntity  string
    timestamp       timestamp
  }

  consumers []

  verify integration "emits graph_resource_served with correct resourceUri and format after MCP resource read"

}

// ── Schema Breaking Change Events ─────────────────────────────

event schema_breaking_change_detected "Schema Breaking Change Detected" {
  trigger   detect_breaking_schema_changes
  channel   "internal"

  payload {
    previousVersion string
    currentVersion  string
    breakingCount   integer
    nonBreakingCount integer
    timestamp       timestamp
  }

  consumers [negotiate_schema_version]

  verify integration "emits schema_breaking_change_detected with correct previousVersion and currentVersion"
  verify integration "consumer negotiate_schema_version receives event for version compatibility checks"

}
