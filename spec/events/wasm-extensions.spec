// Extension entity kinds, enhancements, contributions, collectors,
// discovery, lock files, and doctor events

use types/wasm
use behaviors/wasm-extensions
use behaviors/wasm-lifecycle
use behaviors/wasm-sandbox

// ── Query Extension Events ───────────────────────────────────

event query_extensions_loaded "Query Extensions Loaded" {
  trigger   provide_extension_query_extensions
  channel   "wasm.query_extensions_loaded"

  payload {
    extensionName     string
    queryFileKind     string
    patternCount      integer
    validationPassed  boolean
  }

  consumers [compose_query_files_from_extensions]

  verify integration "emits query_extensions_loaded with extension identity and pattern count"
  verify integration "consumer compose_query_files_from_extensions receives event"

}

event query_files_composed "Query Files Composed" {
  trigger   compose_query_files_from_extensions
  channel   "wasm.query_files_composed"

  payload {
    queryFileKind     string
    extensionCount    integer
    totalPatternSize  integer
    compositionTimeMs integer
  }

  verify integration "emits query_files_composed with correct extensionCount and totalPatternSize"

}

// ── Entity Kind Conflict Events ─────────────────────────────

event entity_kind_conflict_detected "Entity Kind Conflict Detected" {
  trigger   detect_entity_kind_collision
  channel   "wasm.entity_kind_conflict_detected"

  payload {
    kindName          string
    firstExtension    string
    secondExtension   string
    conflictType      string
  }

  consumers [resolve_entity_kind_conflict_via_config]

  verify integration "emits entity_kind_conflict_detected with both extension identities"
  verify integration "consumer resolve_entity_kind_conflict_via_config receives event"

}

event entity_kind_conflict_resolved "Entity Kind Conflict Resolved" {
  trigger   resolve_entity_kind_conflict_via_config
  channel   "wasm.entity_kind_conflict_resolved"

  payload {
    kindName          string
    winningExtension  string
    policy            string
    resolution        string
  }

  verify integration "emits entity_kind_conflict_resolved with winning extension and applied policy"

}

// ── Entity Kind Conflict Prevention Events ────────────────────

event reserved_entity_kind_rejected "Reserved Entity Kind Rejected" {
  trigger   reject_reserved_entity_kind
  channel   "wasm.reserved_entity_kind_rejected"

  payload {
    kindName          string
    extensionName     string
    reservedBy        string
  }

  consumers []

  verify integration "emits reserved_entity_kind_rejected with kind name and reserving party"

}

event entity_kind_qualified "Entity Kind Qualified" {
  trigger   qualify_entity_kind_inline
  channel   "wasm.entity_kind_qualified"

  payload {
    qualifiedName     string
    extensionName     string
    resolvedKind      string
  }

  consumers []

  verify integration "emits entity_kind_qualified with resolved kind details"

}

event extension_specifier_parsed "Extension Specifier Parsed" {
  trigger   parse_extension_specifier
  channel   "wasm.extension_specifier_parsed"

  payload {
    raw               string
    format            string
    name              string
  }

  consumers []

  verify integration "emits extension_specifier_parsed with correct format and name"

}

// ── Lock File & Source Resolution Events ─────────────────────

event lock_file_written "Lock File Written" {
  trigger   write_lock_file
  channel   "wasm.lock_file_written"

  payload {
    extensionCount  integer
    lockFilePath    string
  }

  verify integration "emits lock_file_written with correct extensionCount"

}

event lock_file_read "Lock File Read" {
  trigger   read_lock_file
  channel   "wasm.lock_file_read"

  payload {
    extensionCount    integer
    lockFilePath      string
    allEntriesMatched boolean
  }

  consumers [verify_wasm_integrity]

  verify integration "emits lock_file_read with correct extensionCount and match status"
  verify integration "consumer verify_wasm_integrity receives event after lock file read"

}

event wasm_integrity_check_failed "Wasm Integrity Check Failed" {
  trigger   verify_wasm_integrity
  channel   "wasm.integrity_check_failed"

  payload {
    extensionName   string
    expectedHash    string
    actualHash      string
  }

  consumers [handle_wasm_trap]

  verify integration "emits wasm_integrity_check_failed with expected and actual hashes"

}

// ── Manifest Loading Events ──────────────────────────────────

event manifest_loaded "Manifest Loaded" {
  trigger   load_extension_manifest
  channel   "wasm.manifest_loaded"

  payload {
    extensionName     string
    manifestPath      string
    entityKindCount   integer
    validationRuleCount integer
  }

  consumers [validate_extension_manifest]

  verify integration "emits manifest_loaded with correct extensionName and manifestPath"
  verify integration "consumer validate_extension_manifest receives event"

}

// ── Entity Enhancement Events ────────────────────────────────

event enhancement_registered "Enhancement Registered" {
  trigger   register_entity_enhancements
  channel   "wasm.enhancement_registered"

  payload {
    extensionName     string
    targetEntity    string
    fieldName       string
    fieldType       string
    isReference     boolean
  }

  consumers [run_doctor_check]

  verify integration "emits enhancement_registered with correct field details"

}

event enhancement_conflict_detected "Enhancement Conflict Detected" {
  trigger   detect_enhancement_conflicts
  channel   "wasm.enhancement_conflict_detected"

  payload {
    entityKind      string
    fieldName       string
    firstExtension    string
    secondExtension   string
    resolution      string
  }

  consumers [resolve_enhancement_conflicts]

  verify integration "emits enhancement_conflict_detected with both extension identities"
  verify integration "consumer resolve_enhancement_conflicts receives event"

}

event enhancement_conflict_resolved "Enhancement Conflict Resolved" {
  trigger   resolve_enhancement_conflicts
  channel   "wasm.enhancement_conflict_resolved"

  payload {
    entityKind        string
    fieldName         string
    winningExtension  string
    resolution        string
  }

  verify integration "emits enhancement_conflict_resolved with winning extension and resolution strategy"

}

// ── Contribution Lifecycle Events ──────────────────────────

event contribution_exports_dispatched "Contribution Exports Dispatched" {
  trigger   dispatch_contribution_exports
  channel   "wasm.contribution_exports_dispatched"

  payload {
    extensionName       string
    contributionType    string
    exportName          string
    durationMs          integer
  }

  verify integration "emits contribution_exports_dispatched with correct contributionType and exportName"

}

event contribution_export_validation_failed "Contribution Export Validation Failed" {
  trigger   validate_contribution_exports
  channel   "wasm.contribution_export_validation_failed"

  payload {
    extensionName       string
    missingExports      string[]
    declaredContributions string[]
  }

  verify integration "emits contribution_export_validation_failed with missing export names"

}

event contribution_permission_denied "Contribution Permission Denied" {
  trigger   enforce_per_call_site_permissions
  channel   "wasm.contribution_permission_denied"

  payload {
    extensionName       string
    callSite            string
    hostFunction        string
    reason              string
  }

  verify integration "emits contribution_permission_denied with correct callSite and hostFunction"

}

event contribution_toggled "Contribution Toggled" {
  trigger   toggle_extension_contributions
  channel   "wasm.contribution_toggled"

  payload {
    extensionName       string
    contributionType    string
    enabled             boolean
  }

  consumers [dispatch_contribution_exports]

  verify integration "emits contribution_toggled with correct enabled state"

}

// ── Discovery Events ──────────────────────────────────────

event extensions_discovered "Extensions Discovered" {
  trigger   discover_extensions
  channel   "wasm.extensions_discovered"

  payload {
    source          string
    extensionCount  integer
    matchedVersions integer
  }

  verify integration "emits extensions_discovered with correct source and extensionCount"

}

// ── Collector Events ────────────────────────────────────────

event collector_registered "Collector Registered" {
  trigger   register_collector_contributions
  channel   "wasm.collector_registered"

  payload {
    extensionName   string
    collectorName   string
    inputFormats    string[]
    hasAutoDetect   boolean
  }

  consumers [auto_detect_collector]

  verify integration "emits collector_registered with correct collectorName and inputFormats"
  verify integration "consumer auto_detect_collector receives event"

}

event collector_dispatched "Collector Dispatched" {
  trigger   dispatch_collector
  channel   "wasm.collector_dispatched"

  payload {
    collectorName   string
    reportPath      string
    entityCount     integer
    durationMs      integer
    success         boolean
  }

  consumers [validate_collector_output]

  verify integration "emits collector_dispatched with correct collectorName and durationMs"
  verify integration "consumer validate_collector_output receives event"

}

event collector_output_validated "Collector Output Validated" {
  trigger   validate_collector_output
  channel   "wasm.collector_output_validated"

  payload {
    collectorName     string
    schemaValid       boolean
    unknownEntityCount integer
    statsConsistent   boolean
  }

  consumers [ingest_collector_report]

  verify integration "emits collector_output_validated with correct schemaValid and unknownEntityCount"
  verify integration "consumer ingest_collector_report receives event after validation"

}

event collector_report_ingested "Collector Report Ingested" {
  trigger   ingest_collector_report
  channel   "wasm.collector_report_ingested"

  payload {
    collectorName   string
    totalEntries    integer
    mappedEntries   integer
    unmappedEntries integer
    outputPath      string
  }

  // After collector report ingestion, the graph has new coverage metadata.
  // Consumers should re-export or refresh graph outputs to reflect updated
  // traceability and coverage data.
  consumers [dispatch_contribution_exports]

  verify integration "emits collector_report_ingested with correct entry counts"
  verify integration "consumer dispatch_contribution_exports re-renders outputs after ingestion"

}

// ── Lock File & Source Resolution Events (additional) ─────────

event lock_file_refreshed "Lock File Refreshed" {
  trigger  refresh_lock_file
  channel  "wasm.lock"

  payload {
    path        string
    entry_count integer
  }

  consumers []

  verify integration "emits lock_file_refreshed with correct path and entry_count"

}

event extension_source_resolved "Extension Source Resolved" {
  trigger  resolve_extension_source
  channel  "wasm.source_resolved"

  payload {
    extension_id  string
    source        ExtensionSource
    resolved_path string
  }

  consumers []

  verify integration "emits extension_source_resolved with correct extension_id and resolved_path"

}

event doctor_check_completed "Doctor Check Completed" {
  trigger   run_doctor_check
  channel   "wasm.doctor_check_completed"

  payload {
    issueCount      integer
    extensionCount  integer
    cacheHealthy    boolean
    timestamp       timestamp
  }

  consumers []

  verify integration "emits doctor_check_completed with correct issueCount after health check"

}

event batch_update_completed "Batch Update Completed" {
  trigger   update_all_extensions
  channel   "wasm.batch_update_completed"

  payload {
    updatedCount    integer
    failedCount     integer
    skippedCount    integer
    timestamp       timestamp
  }

  // After a batch update completes, the AOT cache for updated extensions
  // must be invalidated so that stale compiled artifacts are not served.
  // Downstream compilation should be re-triggered to pick up new versions.
  consumers [invalidate_aot_cache]

  verify integration "emits batch_update_completed with correct updatedCount after bulk update"
  verify integration "consumer invalidate_aot_cache receives event to clear stale AOT artifacts"

}
