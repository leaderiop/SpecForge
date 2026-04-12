// Extension entity kinds, enhancements, contributions, collectors,
// discovery, lock files, and doctor events

use "types/wasm"
use "types/core"
// ── Query Extension Events ───────────────────────────────────

event query_extensions_loaded "Query Extensions Loaded" {
  channel   "wasm.query_extensions_loaded"

  payload {
    extensionName     string
    queryFileKind     string
    patternCount      integer
    validationPassed  boolean
  }


  verify integration "emits query_extensions_loaded with extension identity and pattern count"
  verify integration "consumer compose_query_files_from_extensions receives event"

}

event query_files_composed "Query Files Composed" {
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
  channel   "wasm.entity_kind_conflict_detected"

  // Payload corresponds to EntityKindConflict (types/zero-entity-core.spec)
  payload {
    kindName          string
    firstExtension    string
    secondExtension   string
    conflictType      string
  }


  verify integration "emits entity_kind_conflict_detected with both extension identities"

}

event reserved_entity_kind_rejected "Reserved Entity Kind Rejected" {
  channel   "wasm.reserved_entity_kind_rejected"

  payload {
    kindName          string
    extensionName     string
    reservedBy        string
  }


  verify integration "emits reserved_entity_kind_rejected with kind name and reserving party"

}

event extension_specifier_parsed "Extension Specifier Parsed" {
  channel   "wasm.extension_specifier_parsed"

  payload {
    raw               string
    format            string
    name              string
  }


  verify integration "emits extension_specifier_parsed with correct format and name"

}

// ── Lock File & Source Resolution Events ─────────────────────

event lock_file_written "Lock File Written" {
  channel   "wasm.lock_file_written"

  payload {
    extensionCount  integer
    lockFilePath    string
  }

  verify integration "emits lock_file_written with correct extensionCount"

}

event lock_file_read "Lock File Read" {
  channel   "wasm.lock_file_read"

  payload {
    extensionCount    integer
    lockFilePath      string
    allEntriesMatched boolean
  }


  verify integration "emits lock_file_read with correct extensionCount and match status"
  verify integration "consumer verify_wasm_integrity receives event after lock file read"

}

event wasm_integrity_check_failed "Wasm Integrity Check Failed" {
  channel   "wasm.integrity_check_failed"

  payload {
    extensionName   string
    expectedHash    string
    actualHash      string
  }


  verify integration "emits wasm_integrity_check_failed with expected and actual hashes"

}

// ── Manifest Loading Events ──────────────────────────────────

event manifest_loaded "Manifest Loaded" {
  channel   "wasm.manifest_loaded"

  payload {
    extensionName     string
    manifestPath      string
    entityKindCount   integer
    validationRuleCount integer
  }


  verify integration "emits manifest_loaded with correct extensionName and manifestPath"
  verify integration "consumer validate_extension_manifest receives event"

}

// ── Entity Enhancement Events ────────────────────────────────

event enhancement_registered "Enhancement Registered" {
  channel   "wasm.enhancement_registered"

  payload {
    extensionName     string
    targetEntity    string
    fieldName       string
    fieldType       string
    isReference     boolean
  }


  verify integration "emits enhancement_registered with correct field details"

}

event enhancement_conflict_detected "Enhancement Conflict Detected" {
  channel   "wasm.enhancement_conflict_detected"

  payload {
    entityKind      string
    fieldName       string
    firstExtension    string
    secondExtension   string
    resolution      string
  }


  verify integration "emits enhancement_conflict_detected with both extension identities"
  verify integration "consumer resolve_enhancement_conflicts receives event"

}

event enhancement_conflict_resolved "Enhancement Conflict Resolved" {
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
  channel   "wasm.contribution_exports_dispatched"

  payload {
    extensionName       string
    contributionType    string
    exportName          string
    durationMs          integer
  }

  verify integration "emits contribution_exports_dispatched with correct contributionType and exportName"

}

event contribution_exports_validated "Contribution Exports Validated" {
  channel   "wasm.contribution_exports_validated"

  payload {
    extensionName       string
    validatedExports    integer
  }


  verify integration "emits contribution_exports_validated after all declared exports verified"

}

event contribution_export_validation_failed "Contribution Export Validation Failed" {
  channel   "wasm.contribution_export_validation_failed"

  payload {
    extensionName       string
    missingExports      string[]
    declaredContributions string[]
  }

  verify integration "emits contribution_export_validation_failed with missing export names"

}

event contribution_permission_denied "Contribution Permission Denied" {
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
  channel   "wasm.contribution_toggled"

  payload {
    extensionName       string
    contributionType    string
    enabled             boolean
  }


  verify integration "emits contribution_toggled with correct enabled state"

}

// ── Discovery Events ──────────────────────────────────────
// Terminal events (consumers []) are intentionally leaf events for
// observability, audit trails, and CLI output. Not every event requires
// a behavioral consumer — these events serve as integration points for
// external tooling, logging, and traceability.

event extensions_discovered "Extensions Discovered" {
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
  channel   "wasm.collector_registered"

  payload {
    extensionName   string
    collectorName   string
    inputFormats    string[]
    hasAutoDetect   boolean
  }


  verify integration "emits collector_registered with correct collectorName and inputFormats"
  verify integration "consumer auto_detect_collector receives event"

}

event collector_dispatched "Collector Dispatched" {
  channel   "wasm.collector_dispatched"

  payload {
    collectorName   string
    reportPath      string
    entityCount     integer
    durationMs      integer
    success         boolean
  }


  verify integration "emits collector_dispatched with correct collectorName and durationMs"
  verify integration "consumer validate_collector_output receives event"

}

event collector_output_validated "Collector Output Validated" {
  channel   "wasm.collector_output_validated"

  payload {
    collectorName     string
    schemaValid       boolean
    unknownEntityCount integer
    statsConsistent   boolean
  }


  verify integration "emits collector_output_validated with correct schemaValid and unknownEntityCount"
  verify integration "consumer ingest_collector_report receives event after validation"

}

event collector_report_ingested "Collector Report Ingested" {
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

  verify integration "emits collector_report_ingested with correct entry counts"
  verify integration "consumer dispatch_contribution_exports re-renders outputs after ingestion"

}

// ── Lock File & Source Resolution Events (additional) ─────────

event lock_file_refreshed "Lock File Refreshed" {
  channel  "wasm.lock"

  payload {
    lockFilePath string
    entryCount   integer
  }


  verify integration "emits lock_file_refreshed with correct lockFilePath and entryCount"

}

event extension_source_resolved "Extension Source Resolved" {
  channel  "wasm.source_resolved"

  payload {
    extension_id  string
    source        ExtensionSource
    resolved_path string
  }


  verify integration "emits extension_source_resolved with correct extension_id and resolved_path"

}

event doctor_check_completed "Doctor Check Completed" {
  channel   "wasm.doctor_check_completed"

  payload {
    issueCount      integer
    extensionCount  integer
    cacheHealthy    boolean
    timestamp       timestamp
  }


  verify integration "emits doctor_check_completed with correct issueCount after health check"

}

event batch_update_completed "Batch Update Completed" {
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

  verify integration "emits batch_update_completed with correct updatedCount after bulk update"
  verify integration "consumer invalidate_aot_cache receives event to clear stale AOT artifacts"

}

// -- Extension-Defined Grammar Events ----------------------------------------

event grammar_loaded "Grammar Loaded" {
  payload GrammarCacheEntry
  channel "wasm.grammar_loaded"

  contract """
    Emitted when a grammar .wasm binary is successfully loaded and cached.
    Consumers MAY use this to update LSP highlighting configuration.
  """


  verify integration "grammar_loaded emitted after successful grammar load"
}

event grammars_composed "Grammars Composed" {
  payload GrammarConflictPolicy
  channel "wasm.grammars_composed"

  contract """
    Emitted after all grammar contributions have been composed into a
    coherent grammar configuration. Consumers MAY use this to finalize
    LSP highlighting setup.
  """


  verify integration "grammars_composed emitted after composition completes"
}

event body_parsed "Body Parsed" {
  payload FieldMap
  channel "wasm.body_parsed"

  contract """
    Emitted when a body parser successfully transforms raw body text
    into structured JSON fields for an entity. The payload contains
    the resulting FieldMap.
  """


  verify integration "body_parsed emitted after successful body parse"
}
