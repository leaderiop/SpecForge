// @specforge/rust extension types — Rust test traceability

type TestGuard {
  entity_kind string  @readonly
  entity_id   string  @readonly
  test_name   string  @readonly
  module_path string  @readonly
  file        string  @readonly
  line        integer @readonly
  verify unit "TestGuard schema is valid"
}

type TestRegistry {
  entries EntityMappingEntry[]
  verify unit "TestRegistry schema is valid"
}

type EntityMappingEntry {
  entity_id  string  @readonly
  test_name  string
  file       string
  line       integer @optional
  resolution MappingResolutionLevel
  verify unit "EntityMappingEntry schema is valid"
}

type MappingResolutionLevel = proc_macro | convention

type RustFrameworkSupport {
  framework RustFramework
  support   RustSupportLevel
  mechanism string
  verify unit "RustFrameworkSupport schema is valid"
}

type RustFramework = builtin | nextest | proptest | criterion | tokio | rstest | trybuild

type RustSupportLevel = full | partial | unsupported
