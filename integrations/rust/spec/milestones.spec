// specforge-test crate milestones
// Phased delivery: start with pure annotation, evolve to integrated feedback.

use "features"
use "behaviors"
use "invariants"

milestone phase_1_annotation "Phase 1: Annotation and Collection" {
  status active

  features  [test_annotation, result_collection]
  behaviors [
    expand_test_attribute,
    record_test_result_on_drop,
    register_atexit_handler,
    emit_binary_report,
    resolve_convention_mapping
  ]

  exit_criteria {
    proc_macro_works      "#[specforge::test(behavior = 'id')] compiles and injects guard"
    drop_records          "TestGuard Drop records pass/fail correctly"
    atexit_writes         "target/specforge/<binary>.json is written at process exit"
    convention_resolves   "double-underscore and module name conventions match entity IDs"
    multi_binary          "workspace with multiple test binaries produces separate reports"
    published             "specforge-test and specforge-test-macros published to crates.io"
    zero_compiler_dep     "cargo tree shows no specforge compiler crates"
  }
}

milestone phase_2_build_integration "Phase 2: build.rs Graph Export" {
  status planned

  features  [build_integration]
  behaviors [invoke_specforge_export, generate_entity_constants]

  exit_criteria {
    build_rs_calls_export   "build.rs invokes specforge export and produces graph.json"
    graceful_without_sf     "build succeeds with cargo warning when specforge not installed"
    rerun_on_spec_change    "cargo rebuilds when spec/ files change"
    entity_constants_opt_in "generated entity constants compile behind cfg attribute"
    typo_is_compile_error   "referencing a non-existent entity ID fails at compile time"
  }
}

milestone phase_3_coverage_summary "Phase 3: Coverage Summary at Test Exit" {
  status planned

  features  [coverage_summary]
  behaviors [load_graph_at_exit, compute_coverage_diff, print_coverage_summary]

  exit_criteria {
    summary_after_test    "coverage table prints to stderr after cargo test"
    timestamp_visible     "graph export age is shown in summary header"
    no_graph_silent       "no output when graph.json is missing"
    no_tests_silent       "no output when zero specforge tests ran"
    diff_accurate         "expected/covered counts match graph and collected results"
    sub_second            "coverage diff computation adds < 50ms to test exit"
  }
}
