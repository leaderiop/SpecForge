// Compilation events — signals emitted during the compiler pipeline

use types/core
use types/graph
use types/diagnostics
use behaviors/parsing
use behaviors/graph
use behaviors/validation
use behaviors/error-reporting
use behaviors/incremental
use behaviors/coverage

event file_parsed "File Parsed" {
  trigger   parse_spec_file_to_ast
  channel   "compiler.file_parsed"

  payload {
    filePath   string
    entityCount integer
    errorCount  integer
    timestamp   timestamp
  }

  consumers [recover_from_syntax_errors, build_in_memory_graph]

  verify integration "emits file_parsed with correct entityCount and errorCount after successful parse"
  verify integration "consumers receive file_parsed and proceed to error recovery and graph building"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
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

  consumers [detect_dangling_references, compute_subgraph_for_invalidation]

  verify integration "emits graph_built with accurate nodeCount and edgeCount after graph construction"
  verify integration "consumers receive graph_built and trigger reference detection and subgraph computation"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
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

  consumers [format_diagnostics_with_source_context, provide_did_you_mean_suggestions]

  verify integration "emits validation_complete with correct error, warning, and info counts"
  verify integration "consumers receive validation_complete and format diagnostics with suggestions"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event file_changed "File Changed" {
  trigger   watch_file_system_for_changes
  channel   "watch.file_changed"

  payload {
    filePath    string
    changeType  string
    timestamp   timestamp
  }

  consumers [invalidate_changed_files]

  verify integration "emits file_changed with correct filePath and changeType on filesystem modification"
  verify integration "consumer invalidate_changed_files receives event and triggers recompilation"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event subgraph_invalidated "Subgraph Invalidated" {
  trigger   invalidate_changed_files
  channel   "watch.subgraph_invalidated"

  payload {
    invalidatedFiles string[]
    nodeCount        integer
    timestamp        timestamp
  }

  consumers [rebuild_affected_subgraph, emit_incremental_diagnostics]

  verify integration "emits subgraph_invalidated with correct invalidatedFiles list and nodeCount"
  verify integration "consumers rebuild affected subgraph and emit incremental diagnostics"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event test_report_consumed "Test Report Consumed" {
  trigger   consume_specforge_report
  channel   "coverage.report_consumed"

  payload {
    entityCount    integer
    matchedCount   integer
    unmatchedCount integer
    timestamp      timestamp
  }

  consumers [compute_three_layer_coverage, render_test_traceability_matrix]

  verify integration "emits test_report_consumed with correct entityCount, matchedCount, and unmatchedCount"
  verify integration "consumers compute three-layer coverage and render traceability matrix"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
