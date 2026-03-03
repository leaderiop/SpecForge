// Rust code generation types

use types/codegen
use types/coverage

type RustGenConfig {
  outDir          string
  testOutDir      string
  benchOutDir     string    @optional
  naming          RustNamingConvention
  resultStyle     RustResultStyle
  asyncSupport    boolean   @optional
  serdeDerive     boolean   @optional
  testProvider    string    @optional
}

type RustNamingConvention = snake_case | SCREAMING_SNAKE_CASE

type RustResultStyle = thiserror | anyhow | raw

type TestGuard {
  entityKind      string    @readonly
  entityId        string    @readonly
  testName        string    @readonly
  modulePath      string    @readonly
  file            string    @readonly
  line            integer   @readonly
}

type TestRegistry {
  entries         EntityMappingEntry[]
}

type EntityMappingEntry {
  entityId        string    @readonly
  testName        string
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

type DriftChecksum {
  algorithm       string    @readonly
  hash            string    @readonly
  path            string    @readonly
}
