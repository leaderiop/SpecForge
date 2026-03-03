// Code generation features

use behaviors/codegen
use behaviors/wasm

feature type_and_port_code_generation "Type and Port Code Generation" {
  behaviors [
    generate_typescript_interfaces_from_types, generate_port_interfaces, generate_json_schema_from_types,
    respect_naming_conventions, generate_readonly_fields, generate_unique_constraints,
    call_package_generators, incremental_code_generation, support_multiple_languages,
  ]

  problem """
    Type and port definitions in .spec files must be manually translated
    to language-specific interfaces. This duplication leads to drift
    between the spec and the implementation.
  """

  solution """
    specforge gen produces typed interfaces from type and port blocks.
    Supports TypeScript, JSON Schema, and extensible generator plugins.
    Respects naming conventions, readonly annotations, and unique constraints.
    Wasm plugin protocol enables community generators via host functions.
  """
}

feature test_stub_generation_and_drift_detection "Test Stub Generation and Drift Detection" {
  behaviors [generate_test_stubs, detect_generated_code_drift, verify_adapter_implementations]

  problem """
    Developers need boilerplate test stubs for each behavior's verify
    statements. Generated code must stay in sync with spec changes —
    stale code is a silent bug source.
  """

  solution """
    Test stub generation creates framework-specific test files from
    verify statements. Drift detection (gen --check) fails CI if
    regeneration would produce different output. Adapter verification
    confirms hand-written code implements generated interfaces.
  """
}
