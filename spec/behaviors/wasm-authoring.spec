// Extension development workflow — scaffold, build, test, publish

use invariants/wasm
use invariants/extensions
use types/wasm
use types/errors
use ports/outbound
use events/wasm-authoring

behavior scaffold_wasm_extension_project "Scaffold Wasm Extension Project" {
  invariants [wasm_sandbox_integrity]
  types      [ManifestV2]
  ports      [FileSystem]

  contract """
    When specforge extension init is invoked, the system MUST scaffold a
    new Wasm extension project with: a manifest file, a src/ directory
    with a skeleton implementing initialize/validate/export exports,
    a build script targeting wasm32-wasi, and a README with PDK docs.
  """

  produces [extension_project_scaffolded]

  verify unit "scaffold creates manifest file"
  verify unit "scaffold creates src/ with skeleton exports"
  verify unit "scaffold creates build script for wasm32-wasi"

}

behavior build_wasm_extension "Build Wasm Extension" {
  invariants [wasm_sandbox_integrity]
  types      [ManifestV2, ExtensionError]
  ports      [FileSystem]

  contract """
    When specforge extension build is invoked, the system MUST compile
    the extension source to a .wasm binary using the configured toolchain.
    The output .wasm MUST be placed alongside the manifest. Build
    errors MUST be reported as ExtensionError diagnostics.
  """

  produces [extension_built]

  verify unit "build produces .wasm binary"
  verify unit "build errors reported as ExtensionError diagnostics"

}

behavior validate_wasm_extension_locally "Validate Wasm Extension Locally" {
  invariants [wasm_sandbox_integrity]
  types      [ManifestV2, SandboxPolicy, ExtensionError]
  ports      [WasmRuntime, FileSystem]

  contract """
    When specforge extension validate is invoked, the system MUST load
    the locally built .wasm binary and exercise its declared contribution
    exports (validators, renderers, collectors) against fixture .spec
    files in a sandbox environment. This exercises declared contribution
    exports (validators, renderers, collectors) against fixture .spec
    files shipped with the extension — it is NOT test execution. No user
    test suites are invoked, no test frameworks are loaded, and no test
    results are produced. The output is a pass/fail validation report for
    each contribution export, not a test report. The extension MUST run in the same sandbox as
    production to catch permission errors early. Export failures MUST be
    reported as ExtensionError diagnostics.
  """

  produces [extension_fixtures_validated]

  verify unit "validation loads local .wasm binary"
  verify unit "validation runs against fixtures"
  verify unit "validation uses production sandbox policy"
  verify unit "validation failure reported as ExtensionError"

}

// Implementation detail for publish_to_registry in behaviors/extensions.spec.
// Handles Wasm binary packaging and upload.
behavior publish_wasm_extension "Publish Wasm Extension" {
  invariants [registry_integrity, registry_api_openness]
  types      [ManifestV2, ExtensionError]
  ports      [FileSystem, RegistryClient]

  contract """
    When specforge extension publish is invoked, the system MUST bundle
    the .wasm binary and manifest, then publish to the configured
    registry (npm, OCI, or GitHub Releases). The manifest MUST be
    validated before publishing. Validation or publishing failures MUST
    be reported as ExtensionError diagnostics.
  """

  produces [extension_published]

  verify unit "publish bundles .wasm and manifest"
  verify unit "manifest validated before publish"
  verify unit "publish failure reported as ExtensionError"

}
