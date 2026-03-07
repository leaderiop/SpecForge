// @specforge/rust extension types — Rust test collection

use extensions/coverage/types

type TestGuard {
  entity_kind     string    @readonly
  entity_id       string    @readonly
  test_name       string    @readonly
  module_path     string    @readonly
  file            string    @readonly
  line            integer   @readonly
}

type TestRegistry {
  entries         EntityMappingEntry[]
}

type EntityMappingEntry {
  entity_id       string    @readonly
  test_name       string
  file            string
  line            integer   @optional
  resolution      MappingResolutionLevel
  status          TestResultStatus  @optional
}

type MappingResolutionLevel = tests_field | proc_macro | naming_convention

type CollectFormat = junit_xml | libtest_json | cargo_text

type RustFrameworkSupport {
  framework       RustFramework
  support         RustSupportLevel
  mechanism       string
}

type RustFramework = builtin | nextest | proptest | criterion | tokio | rstest | trybuild

type RustSupportLevel = full | partial | unsupported

type RustTestsCollectedPayload {
  totalTests      integer
  mappedTests     integer
  unmappedTests   integer
  format          CollectFormat
  timestamp       timestamp
}
