// Rust code generation behaviors

use invariants/core
use invariants/rust
use types/core
use types/graph
use types/codegen
use types/rust-codegen
use ports/outbound
use governance/rust-decisions

behavior generate_rust_structs_from_types "Generate Rust Structs from Types" {
  types      [Graph, GenOutput, GeneratedFile, RustGenConfig, RustNamingConvention, RustResultStyle]
  ports      [OutputRenderer, FileSystem]
  adrs       [generator_adapter_architecture, phased_rust_delivery]

  contract """
    When specforge gen rust is invoked, the system MUST generate Rust struct
    definitions from type blocks. String fields MUST map to String (not Cow).
    Optional fields MUST map to Option<T>. Array fields MUST map to Vec<T>.
    Union types MUST map to tagged enums with serde discriminants.
  """

  verify unit "struct type produces Rust struct with pub fields"
  verify unit "optional field maps to Option<T>"
  verify unit "array field maps to Vec<T>"
  verify unit "union type produces tagged enum"
  verify unit "readonly annotation produces doc comment, not Cow"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior generate_rust_traits_from_ports "Generate Rust Traits from Ports" {
  types      [Graph, GenOutput, GeneratedFile, RustGenConfig]
  ports      [OutputRenderer, FileSystem]

  contract """
    Port blocks MUST generate Rust trait definitions. Method signatures MUST
    use the configured Result style (thiserror, anyhow, or raw). Inbound ports
    MUST use &self. Outbound ports MUST use &self. Async ports MUST use
    async-trait or RPITIT when async is enabled.
  """

  verify unit "port generates Rust trait with method signatures"
  verify unit "Result types use configured style (thiserror/anyhow/raw)"
  verify unit "async ports use async-trait when enabled"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior generate_rust_test_stubs "Generate Rust Test Stubs" {
  types      [Graph, GenOutput, GeneratedFile, RustGenConfig]
  ports      [OutputRenderer, FileSystem]
  invariants [deterministic_rust_generation]

  contract """
    The system MUST generate one test module per testable entity and one
    #[test] function per verify statement. Function names MUST follow the
    {entity_id}__{slug} convention with double underscore separator.
    Test bodies MUST contain todo!() placeholders. Generated files MUST
    include @specforge-checksum headers.
  """

  verify unit "verify statement produces #[test] fn with todo!() body"
  verify unit "function name follows entity_id__slug convention"
  verify unit "generated file includes @specforge-checksum header"
  verify unit "scenario blocks produce multi-step test stubs with given/when/then comments"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior generate_rust_bench_stubs "Generate Rust Benchmark Stubs" {
  types      [Graph, GenOutput, GeneratedFile, RustGenConfig]
  ports      [OutputRenderer, FileSystem]

  contract """
    When verify load statements exist, the system MUST generate criterion
    benchmark stubs in the configured bench output directory.
  """

  verify unit "verify load produces criterion benchmark stub"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior generate_rust_module_tree "Generate Rust Module Tree" {
  types      [GenOutput, GeneratedFile, RustGenConfig]
  ports      [FileSystem]

  contract """
    The system MUST generate a single entry point tests/specforge_tests.rs
    that re-exports all generated test modules. This produces one test binary
    for faster linking. Module structure MUST mirror entity organization.
  """

  verify unit "entry point re-exports all test modules"
  verify unit "module structure mirrors entity organization"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior slugify_verify_descriptions "Slugify Verify Descriptions" {
  types      [GeneratedFile]

  contract """
    Verify description strings MUST be deterministically slugified to valid
    Rust function name suffixes. Rules: lowercase, spaces to underscore,
    < to lt, > to gt, >= to gte, <= to lte, strip non-alphanumeric except
    underscores. Result MUST be a valid Rust identifier suffix.
  """

  verify unit "spaces become underscores"
  verify unit "comparison operators become word equivalents"
  verify unit "non-alphanumeric characters are stripped"
  verify unit "result is a valid Rust identifier"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior detect_rust_code_drift "Detect Rust Code Drift" {
  types      [DriftResult, DriftChecksum, RustGenConfig]
  ports      [FileSystem]

  contract """
    When specforge gen rust --check is invoked, the system MUST compare
    each drift checksum in @specforge-checksum headers against current
    spec state. If any mismatch exists, the command MUST exit with code 1.
    No files MUST be written in check mode.
  """

  verify unit "matching checksums exit 0"
  verify unit "mismatched checksums exit 1 with stale file paths"
  verify unit "check mode writes no files"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior safe_rust_regeneration "Safe Rust Regeneration" {
  types      [GeneratedFile, RustGenConfig]
  ports      [FileSystem]

  contract """
    The generator MUST refuse to overwrite files containing user
    implementations (non-todo!() test bodies). --merge MUST preserve
    existing bodies while updating signatures. --force MUST overwrite
    without checking.
  """

  verify unit "file with implementation is skipped without --force"
  verify unit "--merge preserves existing test bodies"
  verify unit "--force overwrites all files"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
