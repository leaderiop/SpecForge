// Compilation events — signals emitted during the compiler pipeline

use "types/core"
use "types/graph"
use "types/wasm"
use "types/diagnostics"
event file_parsed "File Parsed" {
  channel   "compiler.file_parsed"

  payload {
    filePath   string
    entityCount integer
    errorCount  integer
    timestamp   timestamp
  }

  // recover_from_syntax_errors runs INSIDE the parser (Tree-sitter error
  // recovery), not as a post-parse event consumer.

  verify integration "emits file_parsed with correct entityCount and errorCount after successful parse"

}

event all_files_parsed "All Files Parsed" {
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
  // needed); load_extension_manifests loads extensions; read_lock_file reads
  // specforge.lock for integrity verification. All three are independent.
  // Lock verification feeds into load_wasm_module via wasm_integrity_verified.

  verify integration "emits all_files_parsed after every file in the project has been parsed"
  verify integration "consumer populates the KindRegistry from extension manifests before Phase 2"

}

event structural_parse_complete "Structural Parse Complete" {
  // Phase 1 completion signal — all .spec files have been structurally parsed
  // into generic entity blocks. No keyword validation has occurred yet.
  // This event marks the handoff from Phase 1 (structural parsing) to
  // extension loading and registry population.
  channel   "compiler.structural_parse_complete"

  payload {
    fileCount     integer
    entityCount   integer
    timestamp     timestamp
  }

  // load_extension_manifests already consumes all_files_parsed. This event
  // is a Phase 1 completion signal for traceability — it does not add a
  // new consumer trigger.

  verify integration "emits structural_parse_complete after all files are structurally parsed"

}

event extension_manifests_loaded {
  channel internal

  payload {
    extensionCount  integer
    manifestPaths   string[]
    timestamp       timestamp
  }


  verify "Extension manifests MUST be loaded before populating the kind registry"
}

event registries_populated "Registries Populated" {
  // Fires after populate_kind_registry, populate_field_registry, and
  // populate_edge_registry all complete. The kind registry populator
  // orchestrates all three and fires this event last.
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

  verify integration "emits registries_populated after KindRegistry, FieldRegistry, and edge type set are fully loaded"
  verify integration "consumer resolve_use_imports starts Phase 2 only after this event"
  verify integration "consumer two_phase_validate_semantic begins keyword and field validation after this event"

}

event resolution_complete "Resolution Complete" {
  channel   "compiler.resolution_complete"

  payload {
    fileCount   integer
    entityCount integer
    timestamp   timestamp
  }


  verify integration "emits resolution_complete after all imports resolved"
  verify integration "consumer build_in_memory_graph receives event and constructs graph"

}

event graph_built "Graph Built" {
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
  // (orphan refs, file reference existence). execute_validation_pattern
  // runs extension-defined declarative rules.

  verify integration "emits graph_built with accurate nodeCount and edgeCount after graph construction"
  verify integration "consumers receive graph_built and trigger structural validation and declarative pattern execution"

}

event declarative_validation_executed "Declarative Validation Executed" {
  channel   "compiler.declarative_validation_executed"

  payload {
    ruleCount       integer
    violationCount  integer
    extensionCount  integer
    timestamp       timestamp
  }


  verify integration "emits declarative_validation_executed with correct ruleCount and violationCount"
  verify integration "consumer aggregate_diagnostic_summary collects pattern diagnostics into final summary"

}

event validation_complete "Validation Complete" {
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

  verify integration "emits validation_complete with correct error, warning, and info counts"
  verify integration "event signals Phase 2 completion — emitter phase may begin"

}

event file_changed "File Changed" {
  channel   "watch.file_changed"

  payload {
    filePath    string
    changeType  string
    timestamp   timestamp
  }


  verify integration "emits file_changed with correct filePath and changeType on filesystem modification"
  verify integration "consumer debounce_file_changes receives event and coalesces into batch"

}

event file_changes_coalesced "File Changes Coalesced" {
  channel   "watch.file_changes_coalesced"

  payload {
    filePaths    string[]
    changeCount  integer
    timestamp    timestamp
  }


  verify integration "emits file_changes_coalesced with correct filePaths after debounce window expires"
  verify integration "consumer invalidate_changed_files receives coalesced batch and computes invalidation set"

}

event subgraph_invalidated "Subgraph Invalidated" {
  channel   "watch.subgraph_invalidated"

  payload {
    invalidatedFiles string[]
    nodeCount        integer
    timestamp        timestamp
  }


  verify integration "emits subgraph_invalidated with correct invalidatedFiles list and nodeCount"
  verify integration "consumer rebuild_affected_subgraph receives event and re-parses invalidated files"
  verify integration "consumer track_import_dag_incrementally updates file dependency edges"

}

event import_dag_updated "Import DAG Updated" {
  channel   "watch.import_dag_updated"

  payload {
    addedEdges      integer
    removedEdges    integer
    cyclesDetected  integer
    timestamp       timestamp
  }


  verify integration "emits import_dag_updated after file dependency edges are added or removed"
  verify integration "consumer rebuild_affected_subgraph waits for import DAG to be current before rebuilding"

}

event incremental_rebuild_complete "Incremental Rebuild Complete" {
  channel   "watch.incremental_rebuild_complete"

  payload {
    rebuiltFiles     string[]
    addedNodes       integer
    removedNodes     integer
    elapsedMs        integer
    timestamp        timestamp
  }


  verify integration "emits incremental_rebuild_complete with correct rebuiltFiles and node counts"
  verify integration "consumer emit_incremental_diagnostics refreshes diagnostics for rebuilt files"
  verify integration "consumer shared_incremental_pipeline updates LSP graph state"
  verify integration "consumer compute_graph_delta diffs previous and new graph states to produce GraphDelta"

}

event render_complete "Render Complete" {
  channel   "compiler.render_complete"

  payload {
    outputFileCount  integer
    format           string
    timestamp        timestamp
  }


  verify integration "emits render_complete with correct outputFileCount after rendering"

}

event export_complete "Export Complete" {
  channel   "compiler.export_complete"

  payload {
    format          string
    entityCount     integer
    scopedToEntity  string
    timestamp       timestamp
  }


  verify integration "emits export_complete with correct format and entityCount after agent export"

}

event format_complete "Format Complete" {
  channel   "compiler.format_complete"

  payload {
    filesChecked  integer
    filesChanged  integer
    timestamp     timestamp
  }


  verify integration "emits format_complete with correct filesChecked and filesChanged"

}

event project_initialized "Project Initialized" {
  // Triggers are mutually exclusive — exactly one fires per init invocation depending on the code path.
  // interactive_extension_selection is a sub-step of scaffold_new_project, not an independent trigger.
  channel   "cli.project_initialized"

  payload {
    projectName     string
    extensionCount  integer
    specFilePath    string   // Path to the starter entity spec file created by init (e.g., hello.spec)
    timestamp       timestamp
  }


  verify integration "emits project_initialized with correct projectName and extensionCount"
  verify integration "specFilePath refers to the starter entity spec file, not specforge.json"

}

event extension_added "Extension Added" {
  channel   "cli.extension_added"

  payload {
    extensionSpecifier  string
    totalExtensions     integer
    wasDuplicate        boolean
    timestamp           timestamp
  }


  verify integration "emits extension_added with correct extensionSpecifier after specforge add"
  verify unit "wasDuplicate is true when extension was already installed"

}

event extension_loading_failed {
  channel   "compilation.extension_loading_failed"

  payload {
    extension_name string
    error_kind     string
    message        string
  }


  verify integration "manifest parse error emits extension_loading_failed event"

}

event custom_entity_type_defined "Custom Entity Type Defined" {
  // Emitted once per define block. After ALL define blocks are processed,
  // define_blocks_registered fires as the aggregate signal.
  channel   "compiler.custom_entity_type_defined"

  payload {
    kindName        string
    fieldCount      integer
    sourceFile      string
    timestamp       timestamp
  }


  verify integration "emits custom_entity_type_defined when a define block registers a custom entity type"
  verify integration "fires after registries_populated (define blocks run in Phase 2)"

}

event define_blocks_registered "Define Blocks Registered" {
  // Fires after all define blocks in the project have been processed and
  // their custom entity kinds registered in the KindRegistry. This event
  // fires after registries_populated (extension kinds) but before resolution
  // begins, ensuring that both extension-defined and project-defined kinds
  // are available for semantic validation.
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

  verify integration "emits define_blocks_registered after all define blocks processed"
  verify integration "fires after registries_populated event"
  verify integration "kindCount includes both extension-defined and define-block kinds"

}

// ── LSP Lifecycle Events ─────────────────────────────────────

event lsp_initialized "LSP Initialized" {
  channel   "lsp.initialized"

  payload {
    extensionCount  integer
    kindCount       integer
    timestamp       timestamp
  }


  verify integration "emits lsp_initialized after LSP server completes initialization"

}

event lsp_shutdown_complete "LSP Shutdown Complete" {
  channel   "lsp.shutdown_complete"

  payload {
    timestamp       timestamp
  }


  verify integration "emits lsp_shutdown_complete after LSP server shuts down cleanly"

}

// ── Migration Events ──────────────────────────────────────────

// migration_started fires after the backup is created but before transforms run.
// Contrast with migration_starting, which fires before any file I/O begins.
event migration_started "Migration Started" {
  channel   "compiler.migration_started"

  payload {
    fileCount       integer
    targetVersion   string
    timestamp       timestamp
  }


  verify integration "emits migration_started with correct fileCount and targetVersion"

}

// migration_starting fires before any file I/O begins (pre-backup).
// Contrast with migration_started, which fires after the backup is created.
event migration_starting "Migration Starting" {
  channel   "compiler.migration_starting"

  payload {
    fileCount       integer
    targetVersion   string
    timestamp       timestamp
  }


  verify integration "emits migration_starting before any migration transforms are applied"

}

event pre_migration_snapshot_captured "Pre-Migration Snapshot Captured" {
  channel   "compiler.pre_migration_snapshot_captured"

  payload {
    nodeKindCount   integer
    edgeTypeCount   integer
    fieldCount      integer
    timestamp       timestamp
  }


  verify integration "emits pre_migration_snapshot_captured after schema snapshot is taken"

}

event migration_complete "Migration Complete" {
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


  verify integration "emits migration_complete with correct migratedCount, failedCount, and skippedCount"
  verify integration "consumer invoke_extension_migration_hooks receives event to run extension hooks"

}

event migration_diff_generated "Migration Diff Generated" {
  channel   "compiler.migration_diff_generated"

  payload {
    fileCount       integer
    totalDiffLines  integer
    timestamp       timestamp
  }


  verify integration "emits migration_diff_generated with correct fileCount and totalDiffLines after dry-run"

}

event extension_migration_hooks_complete "Extension Migration Hooks Complete" {
  channel   "compiler.extension_migration_hooks_complete"

  payload {
    extensionsInvoked integer
    extensionsFailed  integer
    extensionsSkipped integer
    timestamp         timestamp
  }


  verify integration "emits extension_migration_hooks_complete after all extension hooks finish"
  verify integration "consumer validate_post_migration_integrity receives event"

}

event migration_validation_complete "Migration Validation Complete" {
  channel   "compiler.migration_validation_complete"

  payload {
    structuralDifferences  integer
    newDiagnostics         integer
    timestamp              timestamp
  }


  verify integration "emits migration_validation_complete after post-migration validation finishes"

}

event migration_rolled_back "Migration Rolled Back" {
  channel   "compiler.migration_rolled_back"

  payload {
    restoredCount   integer
    skippedCount    integer
    failedCount     integer
    timestamp       timestamp
  }


  verify integration "emits migration_rolled_back with correct restored, skipped, and failed counts"

}

// ── Schema Events ────────────────────────────────────────────

event schema_generated "Schema Generated" {
  channel   "compiler.schema_generated"

  payload {
    entityKindCount integer
    edgeTypeCount   integer
    extensionCount  integer
    timestamp       timestamp
  }

  // embed_schema_in_export now waits for schema_version_computed (via the
  // schema pipeline), not schema_generated directly.

  verify integration "emits schema_generated with correct entityKindCount and edgeTypeCount"
  verify integration "consumer persist_schema_cache receives event"
  verify integration "consumer detect_breaking_schema_changes receives event"

}

event schema_cache_persisted "Schema Cache Persisted" {
  channel   "compiler.schema_cache_persisted"

  payload {
    schemaVersion   string
    hash            string
    timestamp       timestamp
  }


  verify integration "emits schema_cache_persisted after writing .specforge/schema-cache.json"

}

event schema_version_computed "Schema Version Computed" {
  channel   "compiler.schema_version_computed"

  payload {
    major           integer
    minor           integer
    patch           integer
    bumpType        string
    timestamp       timestamp
  }


  verify integration "emits schema_version_computed with correct major, minor, patch after schema comparison"

}

// Entity embedding events moved to spec/extensions/embeddings/events.spec

// ── Agent & Schema Events ─────────────────────────────────────

event plan_validated "Agent Plan Validated" {
  payload  PlanValidationResult
  channel  compilation

  verify integration "Agent Plan Validated emitted correctly"
}

event token_budget_applied "Token Budget Applied" {
  payload  ExportResult
  channel  compilation

  verify integration "Token Budget Applied emitted correctly"
}

event schema_version_negotiated "Schema Version Negotiated" {
  payload  SchemaCompatibility
  channel  compilation

  verify integration "Schema Version Negotiated emitted correctly"
}

// ── Output Query Events ──────────────────────────────────────

event graph_queried "Graph Queried" {
  channel   "output"

  payload {
    entityId        string
    scope           string
    depth           integer
    resultNodeCount integer
    timestamp       timestamp
  }


  verify integration "emits graph_queried with correct entityId, scope, and resultNodeCount"
  verify integration "payload includes depth and scope used for the query"

}

// entity_search_performed moved to spec/extensions/embeddings/events.spec

// ── Incremental Pipeline Terminal Events ──────────────────────
// Terminal events (consumers []) are intentionally leaf events for
// observability, audit trails, and CLI output. Not every event requires
// a behavioral consumer — these events serve as integration points for
// external tooling, logging, and traceability without mandating an
// in-process consumer.

event incremental_diagnostics_complete "Incremental Diagnostics Complete" {
  channel   "watch.incremental_diagnostics_complete"

  payload {
    errorCount      integer
    warningCount    integer
    infoCount       integer
    affectedFiles   integer
    timestamp       timestamp
  }


  verify integration "emits incremental_diagnostics_complete after incremental diagnostic pipeline finishes"

}

event delta_subscribers_notified "Delta Subscribers Notified" {
  channel   "watch.delta_subscribers_notified"

  payload {
    subscriberCount integer
    timestamp       timestamp
  }


  verify integration "emits delta_subscribers_notified after all subscribers receive delta"

}

// ── Rename Events ───────────────────────────────────────────

event entity_renamed "Entity Renamed" {
  channel   "lsp.entity_renamed"

  payload {
    oldId           string
    newId           string
    affectedFiles   integer
    timestamp       timestamp
  }


  verify integration "emits entity_renamed with correct oldId, newId, and affectedFiles"

}

// ── Graph Delta Events ──────────────────────────────────────

event graph_delta_computed "Graph Delta Computed" {
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


  verify integration "emits graph_delta_computed with correct node and edge counts"
  verify integration "consumer dispatch_incremental_validators receives delta event"
  verify integration "consumer notify_delta_subscribers receives delta event"
  verify integration "consumer validate_delta_correctness receives delta event in debug mode"

}

event incremental_validators_dispatched "Incremental Validators Dispatched" {
  channel   "watch.incremental_validators_dispatched"

  payload {
    incrementalExtensions integer
    fullRebuildExtensions integer
    deltaNodeCount        integer
    timestamp             timestamp
  }


  verify integration "emits incremental_validators_dispatched with correct extension counts"
  verify integration "payload distinguishes incremental vs full-rebuild extension counts"

}

// ── Delta Validation Events ──────────────────────────────────

event delta_validation_passed "Delta Validation Passed" {
  // Success-path counterpart to delta_validation_failed — emitted when
  // validate_delta_correctness confirms incremental and full-rebuild
  // graph states are consistent.
  channel   "watch.delta_validation_passed"

  payload {
    nodeCount   integer
    edgeCount   integer
  }


  verify integration "emits delta_validation_passed when incremental rebuild matches full rebuild"

}

event trace_chain_computed "Trace Chain Computed" {
  channel   "output.trace_chain_computed"

  payload {
    entityId    string
    chainDepth  integer
    linkCount   integer
  }


  verify integration "emits trace_chain_computed with correct entityId, chainDepth, and linkCount"

}

event delta_validation_failed "Delta Validation Failed" {
  // Debug-only event — emitted when validate_delta_correctness detects
  // a discrepancy between the incremental and full-rebuild graph states.
  channel   "watch.delta_validation_failed"

  payload {
    inconsistentNodes   integer
    inconsistentEdges   integer
    timestamp           timestamp
  }


  verify unit "emits delta_validation_failed when incremental rebuild diverges from full rebuild"

}

// ── MCP Graph Resource Events ─────────────────────────────────

event graph_resource_served "Graph Resource Served" {
  channel   "mcp"

  payload {
    resourceUri     string
    format          string
    entityCount     integer
    scopedToEntity  string
    timestamp       timestamp
  }


  verify integration "emits graph_resource_served with correct resourceUri and format after MCP resource read"

}

// ── Schema Breaking Change Events ─────────────────────────────

event schema_breaking_change_detected "Schema Breaking Change Detected" {
  channel   "internal"

  payload {
    previousVersion string
    currentVersion  string
    breakingCount   integer
    nonBreakingCount integer
    timestamp       timestamp
  }


  verify integration "emits schema_breaking_change_detected with correct previousVersion and currentVersion"
  verify integration "consumer negotiate_schema_version receives event for version compatibility checks"

}

event graph_protocol_compatibility_verified "Graph Protocol Compatibility Verified" {
  channel   "compiler.graph_protocol_compatibility_verified"

  payload {
    breakingChanges   integer
    nonBreakingChanges integer
    timestamp         timestamp
  }


  verify integration "emits graph_protocol_compatibility_verified after post-migration schema comparison"

}

event grammar_contribution_registered "Grammar Contribution Registered" {
  payload GrammarContribution
  channel "compilation.grammar_contribution_registered"

  contract """
    Emitted when a grammar contribution from an extension manifest is
    successfully registered. Consumers use this to track which entity
    kinds have custom grammars.
  """

  verify integration "event emitted per registered grammar contribution"
}

event body_parser_contribution_registered "Body Parser Contribution Registered" {
  payload BodyParserContribution
  channel "compilation.body_parser_contribution_registered"

  contract """
    Emitted when a body parser contribution from an extension manifest
    is successfully registered. Consumers use this to track which entity
    kinds have custom body parsers for Phase 1.5 dispatch.
  """

  verify integration "event emitted per registered body parser contribution"
}
