// Extension authoring workflow events

use types/wasm
use behaviors/wasm-authoring

event extension_project_scaffolded "Extension Project Scaffolded" {
  trigger   scaffold_wasm_extension_project
  channel   "wasm.extension_project_scaffolded"

  payload {
    projectPath     string
    extensionName   string
    manifestCreated boolean
    buildScriptCreated boolean
  }

  consumers [build_wasm_extension]

  verify integration "emits extension_project_scaffolded with correct projectPath and extensionName"
  verify integration "consumer build_wasm_extension receives event after scaffold"

}

event extension_built "Extension Built" {
  trigger   build_wasm_extension
  channel   "wasm.extension_built"

  payload {
    extensionName   string
    wasmSize        integer
    buildTimeMs     integer
    success         boolean
  }

  consumers [validate_wasm_extension_locally]

  verify integration "emits extension_built with correct wasmSize and buildTimeMs"
  verify integration "consumer validate_wasm_extension_locally receives event after build"

}

event extension_fixtures_validated "Extension Fixtures Validated" {
  trigger   validate_wasm_extension_locally
  channel   "wasm.extension_fixtures_validated"

  payload {
    extensionName     string
    fixtureCount      integer
    passed_fixtures   integer
    failed_fixtures   integer
    validation_time_ms integer
  }

  consumers [publish_wasm_extension]

  verify integration "emits extension_fixtures_validated with correct passed_fixtures and failed_fixtures"
  verify integration "consumer publish_wasm_extension receives event after fixture validation"

}

event extension_published "Extension Published" {
  trigger   publish_wasm_extension
  channel   "wasm.extension_published"

  payload {
    extensionName   string
    version         string
    registry        string
    publishTimeMs   integer
  }

  verify integration "emits extension_published with correct extensionName, version, and registry"

}
