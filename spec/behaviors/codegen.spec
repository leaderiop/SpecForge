// Code generation behaviors — producing typed stubs from spec

use invariants/core
use types/core
use types/graph
use types/codegen
use types/errors
use ports/outbound

behavior generate_typescript_interfaces_from_types "Generate TypeScript Interfaces from Types" {
  types      [Graph, GenOutput, GeneratedFile, GenConfig]
  ports      [OutputRenderer, FileSystem]

  contract """
    When specforge gen typescript is invoked, the system MUST generate
    TypeScript interfaces from type blocks. Struct types MUST produce
    interfaces with readonly fields. Union types MUST produce union
    literal types. Error types with _tag MUST produce discriminated unions.
  """

  verify unit "struct type produces TypeScript interface"
  verify unit "readonly annotation produces readonly field"
  verify unit "union type produces literal union"
  verify unit "error type produces discriminated union"
}

behavior generate_port_interfaces "Generate Port Interfaces" {
  types      [Graph, GenOutput, GeneratedFile, GenConfig]
  ports      [OutputRenderer, FileSystem]

  contract """
    Port blocks MUST generate language-specific interface definitions.
    Method signatures MUST use the configured Result type style.
    Inbound and outbound ports MUST be distinguishable in the output.
  """

  verify unit "port generates interface with method signatures"
  verify unit "Result types use configured style"
  verify unit "inbound and outbound ports are distinguished"
}

behavior generate_test_stubs "Generate Test Stubs" {
  types      [Graph, GenOutput, GeneratedFile, GenConfig]
  ports      [OutputRenderer, FileSystem]

  contract """
    When test stub generation is enabled, the system MUST generate
    test file stubs for behaviors with verify statements. Each verify
    statement MUST produce a test case stub with the behavior ID
    and description as metadata.
  """

  verify unit "verify statement produces test stub"
  verify unit "stub includes behavior ID"
  verify unit "stub includes verify description"
}

behavior detect_generated_code_drift "Detect Generated Code Drift" {
  types      [DriftResult, DriftDiff, GenConfig]
  ports      [FileSystem]

  contract """
    When specforge gen --check is invoked, the system MUST compare
    what would be generated against existing files on disk. If any
    difference exists, the command MUST exit with code 1 and print
    the differing file paths. No files MUST be written in check mode.
  """

  verify unit "no drift exits with code 0"
  verify unit "drift detected exits with code 1"
  verify unit "check mode writes no files"
}

behavior verify_adapter_implementations "Verify Adapter Implementations" {
  types      [AdapterVerification]
  ports      [FileSystem]

  contract """
    When specforge verify typescript is invoked, the system MUST scan
    hand-written adapter files and verify they implement all methods
    from the corresponding generated port interfaces. Missing methods
    MUST be reported. Extra methods SHOULD be allowed.
  """

  verify unit "adapter implementing all methods passes"
  verify unit "adapter missing a method is reported"
  verify unit "adapter with extra methods is allowed"
}

behavior generate_json_schema_from_types "Generate JSON Schema from Types" {
  types      [Graph, GenOutput, GeneratedFile]
  ports      [OutputRenderer, FileSystem]

  contract """
    When specforge gen json-schema is invoked, the system MUST produce
    JSON Schema files from type blocks. Required fields MUST have
    required constraint. Optional fields MUST NOT be required.
    Union types MUST use oneOf/enum patterns.
  """

  verify unit "struct type produces JSON Schema object"
  verify unit "required fields appear in required array"
  verify unit "optional fields are not required"
  verify unit "union type uses enum pattern"
}

behavior respect_naming_conventions "Respect Naming Conventions" {
  types      [GenConfig, GeneratedFile]

  contract """
    Code generation MUST respect the naming convention specified in the
    gen config block. camelCase, PascalCase, snake_case, and kebab-case
    MUST be supported for generated file names and identifiers.
  """

  verify unit "camelCase naming applied to identifiers"
  verify unit "snake_case naming applied to identifiers"
}

behavior generate_readonly_fields "Generate Readonly Fields" {
  types      [GeneratedFile]

  contract """
    Type fields annotated with @readonly MUST produce immutable fields
    in generated code. TypeScript MUST use readonly modifier. The generator
    MUST NOT produce setters for readonly fields.
  """

  verify unit "readonly annotation produces readonly in TypeScript"
}

behavior generate_unique_constraints "Generate Unique Constraints" {
  types      [GeneratedFile]

  contract """
    Type fields annotated with @unique MUST produce uniqueness hints
    in generated code. JSON Schema MUST include uniqueItems or comments.
    The generator SHOULD emit validation helpers for unique fields.
  """

  verify unit "unique annotation produces hint in output"
}

behavior plugin_wasm_protocol "Plugin Wasm Protocol" {
  types      [PluginManifest]
  ports      [FileSystem]

  contract """
    Generator plugins MUST communicate via the Wasm host function protocol:
    the plugin accesses the graph via specforge.query_graph, emits generated
    files via specforge.emit_file, and emits diagnostics via
    specforge.emit_diagnostic. The compiler MUST handle Wasm traps
    gracefully with a PluginError.
  """

  verify unit "plugin accesses graph via query_graph host function"
  verify unit "plugin emits files via emit_file host function"
  verify unit "Wasm trap produces PluginError"
}

behavior incremental_code_generation "Incremental Code Generation" {
  invariants [incremental_correctness]
  types      [GenOutput, GeneratedFile]

  contract """
    The code generator SHOULD only regenerate files for entities that
    changed since the last generation run. Unchanged entities SHOULD
    produce identical output and MAY be skipped to save I/O.
  """

  verify unit "unchanged entity produces identical output"
  verify unit "changed entity is regenerated"
}

behavior support_multiple_languages "Support Multiple Languages" {
  types      [GenConfig, GenOutput]

  contract """
    The gen command MUST support multiple language targets in the same
    project. Each gen block in the spec root MUST configure an independent
    output directory and language-specific settings. Languages MUST NOT
    interfere with each other.
  """

  verify unit "multiple gen blocks produce independent outputs"
  verify unit "language-specific settings are isolated"
}
