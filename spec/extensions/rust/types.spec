// @specforge/rust extension types — Rust test collection

use "extensions/coverage/types"
type TestGuard {
  entity_kind     string    @readonly
  entity_id       string    @readonly
  test_name       string    @readonly
  module_path     string    @readonly
  file            string    @readonly
  line            integer   @readonly
  verify unit "TestGuard schema is valid"
}

type TestRegistry {
  entries         EntityMappingEntry[]
  verify unit "TestRegistry schema is valid"
}

type EntityMappingEntry {
  entity_id       string    @readonly
  test_name       string
  file            string
  line            integer   @optional
  resolution      MappingResolutionLevel
  status          TestResultStatus  @optional
  verify unit "EntityMappingEntry schema is valid"
}

type MappingResolutionLevel = tests_field | proc_macro | naming_convention

type CollectFormat = junit_xml | libtest_json | cargo_text

type RustFrameworkSupport {
  framework       RustFramework
  support         RustSupportLevel
  mechanism       string
  verify unit "RustFrameworkSupport schema is valid"
}

type RustFramework = builtin | nextest | proptest | criterion | tokio | rstest | trybuild

type RustSupportLevel = full | partial | unsupported

type RustTestsCollectedPayload {
  totalTests      integer
  mappedTests     integer
  unmappedTests   integer
  format          CollectFormat
  timestamp       timestamp
  verify unit "RustTestsCollectedPayload schema is valid"
}
