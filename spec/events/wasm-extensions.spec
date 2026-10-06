// Extension entity kinds, enhancements, contributions, collectors,
// discovery, lock files, and doctor events

use "types/core"
use "types/wasm"

// ── Query Extension Events ───────────────────────────────────

// ── Entity Kind Conflict Events ─────────────────────────────

event extension_specifier_parsed "Extension Specifier Parsed" {
  channel "wasm.extension_specifier_parsed"
  payload {
    raw    string
    format string
    name   string
  }
  verify integration "emits extension_specifier_parsed with correct format and name"
}

// ── Lock File & Source Resolution Events ─────────────────────

event lock_file_written "Lock File Written" {
  channel "wasm.lock_file_written"
  payload {
    extensionCount integer
    lockFilePath   string
  }
  verify integration "emits lock_file_written with correct extensionCount"
}

event lock_file_read "Lock File Read" {
  channel "wasm.lock_file_read"
  payload {
    extensionCount    integer
    lockFilePath      string
    allEntriesMatched boolean
  }
  verify integration "emits lock_file_read with correct extensionCount and match status"
}

// ── Manifest Loading Events ──────────────────────────────────

event manifest_loaded "Manifest Loaded" {
  channel "wasm.manifest_loaded"
  payload {
    extensionName       string
    manifestPath        string
    entityKindCount     integer
    validationRuleCount integer
  }
  verify integration "emits manifest_loaded with correct extensionName and manifestPath"
  verify integration "consumer validate_extension_manifest receives event"
}

// ── Entity Enhancement Events ────────────────────────────────

event enhancement_registered "Enhancement Registered" {
  channel "wasm.enhancement_registered"
  payload {
    extensionName string
    targetEntity  string
    fieldName     string
    fieldType     string
    isReference   boolean
  }
  verify integration "emits enhancement_registered with correct field details"
}

// ── Contribution Lifecycle Events ──────────────────────────

event contribution_permission_denied "Contribution Permission Denied" {
  channel "wasm.contribution_permission_denied"
  payload {
    extensionName string
    callSite      string
    hostFunction  string
    reason        string
  }
  verify integration "emits contribution_permission_denied with correct callSite and hostFunction"
}

// ── Discovery Events ──────────────────────────────────────
// Terminal events (consumers []) are intentionally leaf events for
// observability, audit trails, and CLI output. Not every event requires
// a behavioral consumer — these events serve as integration points for
// external tooling, logging, and traceability.

// ── Collector Events ────────────────────────────────────────

event collector_registered "Collector Registered" {
  channel "wasm.collector_registered"
  payload {
    extensionName string
    collectorName string
    inputFormats  string[]
    hasAutoDetect boolean
  }
  verify integration "emits collector_registered with correct collectorName and inputFormats"
  verify integration "consumer auto_detect_collector receives event"
}

event collector_dispatched "Collector Dispatched" {
  channel "wasm.collector_dispatched"
  payload {
    collectorName string
    reportPath    string
    fileCount     integer
    success       boolean
  }
  verify integration "emits collector_dispatched with correct collectorName and fileCount"
  verify integration "consumer ingest_collector_report receives event"
}

event collector_report_ingested "Collector Report Ingested" {
  channel "wasm.collector_report_ingested"
  payload {
    collectorName   string
    totalEntries    integer
    mappedEntries   integer
    unmappedEntries integer
    outputPath      string
  }
  // After collector report ingestion, the graph has new coverage metadata.
  verify integration "emits collector_report_ingested with correct entry counts"
}

// ── Lock File & Source Resolution Events (additional) ─────────

event extension_source_resolved "Extension Source Resolved" {
  channel "wasm.source_resolved"
  payload {
    extension_id  string
    source        ExtensionSource
    resolved_path string
  }
  verify integration "emits extension_source_resolved with correct extension_id and resolved_path"
}

event doctor_check_completed "Doctor Check Completed" {
  channel "wasm.doctor_check_completed"
  payload {
    issueCount     integer
    extensionCount integer
    cacheHealthy   boolean
    timestamp      timestamp
  }
  verify integration "emits doctor_check_completed with correct issueCount after health check"
}

event batch_update_completed "Batch Update Completed" {
  channel "wasm.batch_update_completed"
  payload {
    updatedCount integer
    failedCount  integer
    skippedCount integer
    timestamp    timestamp
  }
  // After a batch update completes, the new binary hashes are recorded in
  // specforge.lock. The engine compile cache keys on binary content, so
  // updated extensions never serve stale compiled artifacts. No behavior
  // consumes it: `specforge update --format json` reports it under
  // `batch_update_completed`, for whoever ran the update.
  verify integration "emits batch_update_completed with correct updatedCount after bulk update"
}
