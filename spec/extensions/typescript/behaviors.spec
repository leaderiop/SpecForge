// @specforge/typescript extension behaviors -- TypeScript/JavaScript source intelligence

use "invariants/core"
use "extensions/typescript/invariants"
use "types/core"
use "types/diagnostics"
use "extensions/coverage/types"
use "extensions/typescript/types"
use "ports/outbound"
use "extensions/typescript/events"

// ── Source Scanning ────────────────────────────────────────────

behavior scan_typescript_project "Scan TypeScript Project" {
  types      [TsScanResult, TsSourceItem, TsSourceFile, TsMonorepoInfo, TsFrameworkDetection]
  category   query
  ports      [FileSystem, TsSourceScanner]
  invariants [ts_export_completeness, ts_file_role_accuracy, ts_react_component_detection]
  produces   [ts_project_scanned]

  contract """
    When specforge scan typescript is invoked, the system MUST walk the
    project directory tree, classify each file by role (production, test,
    story, config, etc.), extract all exported symbols from production
    files as TsSourceItems, detect monorepo structure if present, identify
    frameworks in use, and return a TsScanResult.
  """

  verify unit "production .ts files are scanned"
  verify unit "production .tsx files are scanned"
  verify unit "production .js/.jsx files are scanned"
  verify unit ".mjs and .cjs files are scanned"
  verify unit ".d.ts files are classified as declaration"
  verify unit "test files are classified and skipped"
  verify unit "story files are classified and skipped"
  verify unit "node_modules is excluded"
  verify unit "dist/build output is excluded"
  verify integration "end-to-end scan of a real TypeScript project"
}

behavior classify_source_file "Classify Source File" {
  types      [TsSourceFile, TsFileRole, TsFileRoleSignal, TsTestFilePattern]
  category   query
  invariants [ts_file_role_accuracy]

  contract """
    Given a file path, the system MUST classify it as production, test,
    story, config, script, declaration, or generated. Classification uses
    multiple signals: file extension (.test.ts, .spec.ts, .stories.tsx),
    directory name (__tests__, __mocks__, cypress/), file content
    patterns (jest.config, vitest.config), and package.json metadata.
    When signals conflict, the most specific signal wins.
  """

  verify unit "*.test.ts classified as test"
  verify unit "*.spec.ts classified as test"
  verify unit "*.stories.tsx classified as story"
  verify unit "__tests__/** classified as test"
  verify unit "__mocks__/** classified as test"
  verify unit "jest.config.ts classified as config"
  verify unit "vitest.config.ts classified as config"
  verify unit "playwright.config.ts classified as config"
  verify unit "*.e2e.ts classified as test"
  verify unit "*.d.ts classified as declaration"
  verify unit "*.generated.ts classified as generated"
  verify unit "src/index.ts classified as production"
  verify unit "conflicting signals resolved by specificity"
}

behavior extract_source_items "Extract Source Items" {
  types      [TsSourceItem, TsItemKind, TsExportKind, TsParameter, TsJsDocTag, TsDecorator]
  category   query
  ports      [TsSourceScanner]
  invariants [ts_export_completeness, ts_react_component_detection]

  contract """
    Given a production source file, the system MUST extract every exported
    symbol as a TsSourceItem. For each item it MUST capture: name, item kind
    (function, class, interface, type alias, enum, const, React component,
    hook, HOC, decorator), export kind (named, default, re-export, barrel),
    file location (line/column range), type signature, JSDoc documentation,
    decorators, and modifiers (async, abstract, readonly, accessibility).
  """

  verify unit "named export function is extracted with signature"
  verify unit "default export class is extracted with members"
  verify unit "interface is extracted with extends clause"
  verify unit "type alias is extracted with generics"
  verify unit "enum is extracted with members and values"
  verify unit "const export is extracted with type"
  verify unit "export { x } re-export is extracted"
  verify unit "export * from namespace re-export is tracked"
  verify unit "export default function is extracted"
  verify unit "module.exports = is extracted (CJS)"
  verify unit "JSDoc summary and tags are captured"
  verify unit "decorators are captured with arguments"
  verify unit "async/generator modifiers are captured"
  verify unit "generic type parameters are captured"
  verify unit "non-exported symbols are excluded"
}

behavior detect_react_components "Detect React Components" {
  types      [TsSourceItem, TsReactInfo, TsReactComponentType]
  category   query
  invariants [ts_react_component_detection]

  contract """
    The system MUST detect React components across all declaration styles:
    (1) function declarations returning JSX, (2) arrow functions with
    React.FC/FC type annotation, (3) React.forwardRef wrappers,
    (4) React.memo wrappers, (5) React.lazy wrappers, (6) class
    components extending React.Component/PureComponent, (7) higher-order
    components (functions returning components). For each component, the
    system MUST extract props type, forward ref usage, memo wrapping,
    hook usage (useState, useContext, useRef), and framework metadata
    (Server/Client Component directives in Next.js).
  """

  verify unit "function component returning JSX detected"
  verify unit "arrow function with React.FC detected"
  verify unit "forwardRef wrapper detected with ref type"
  verify unit "memo wrapper detected"
  verify unit "lazy wrapper detected"
  verify unit "class component extending React.Component detected"
  verify unit "class component extending PureComponent detected"
  verify unit "higher-order component detected"
  verify unit "custom hook detected (use* prefix)"
  verify unit "props type extracted from generic parameter"
  verify unit "props type extracted from function parameter"
  verify unit "useState calls captured as state_vars"
  verify unit "useContext calls captured as contexts_consumed"
  verify unit "useRef calls captured as refs_used"
  verify unit "use client directive marks client component"
  verify unit "use server directive in Next.js detected"
  verify unit "regular function not misidentified as component"
}

behavior detect_frameworks "Detect Frameworks" {
  types      [TsFrameworkDetection, TsFramework]
  category   query
  ports      [FileSystem]

  contract """
    The system MUST detect which frameworks are in use by examining
    package.json dependencies, config files, and import patterns. Detection
    MUST work for: React, Next.js, Remix, Angular, NestJS, Express,
    Fastify, Hono, Elysia, Svelte, Vue, SolidJS, Astro, Nuxt, Gatsby.
    Each detection includes confidence score and the signals used.
  """

  verify unit "React detected from package.json dependency"
  verify unit "Next.js detected from next.config.* presence"
  verify unit "Angular detected from angular.json presence"
  verify unit "NestJS detected from @nestjs/* imports"
  verify unit "Express detected from express import"
  verify unit "Svelte detected from svelte.config.* presence"
  verify unit "Vue detected from vue import"
  verify unit "multiple frameworks detected simultaneously"
}

// ── Monorepo Support ───────────────────────────────────────────

behavior detect_monorepo "Detect Monorepo Structure" {
  types      [TsMonorepoInfo, TsMonorepoTool, TsPackageInfo]
  category   query
  ports      [FileSystem]

  contract """
    The system MUST detect monorepo tooling by examining root config files:
    package.json "workspaces" field (npm/yarn), pnpm-workspace.yaml (pnpm),
    nx.json (Nx), turbo.json (Turborepo), lerna.json (Lerna), rush.json
    (Rush). When detected, it MUST enumerate all packages, resolve their
    entry points, and identify internal cross-package dependencies. The
    scan MUST respect per-package tsconfig.json boundaries.
  """

  verify unit "npm workspaces detected from package.json"
  verify unit "yarn workspaces detected from package.json"
  verify unit "pnpm workspaces detected from pnpm-workspace.yaml"
  verify unit "Nx detected from nx.json"
  verify unit "Turborepo detected from turbo.json"
  verify unit "Lerna detected from lerna.json"
  verify unit "Rush detected from rush.json"
  verify unit "packages enumerated from workspace globs"
  verify unit "internal dependencies resolved between packages"
  verify unit "private packages flagged correctly"
}

// ── Entity ID Mapping ──────────────────────────────────────────

behavior map_typescript_entity_ids "Map TypeScript Entity IDs" {
  types      [TsEntityIdMapping, TsMappingResolution, TsSourceItem]
  category   query
  invariants [ts_entity_mapping_precedence]

  contract """
    The system MUST convert TypeScript symbol names to specforge entity IDs
    using the following rules: PascalCase -> snake_case (UserService ->
    user_service), camelCase -> snake_case (validateInput -> validate_input),
    SCREAMING_SNAKE -> lower_snake (MAX_RETRIES -> max_retries), React
    hooks -> snake_case without "use" prefix (useAuth -> auth). Resolution
    precedence: (1) tests field in .spec, (2) @specforge JSDoc tag,
    (3) naming convention. Barrel re-exports MUST trace to the original
    defining file for canonical ID assignment.
  """

  verify unit "PascalCase class maps to snake_case"
  verify unit "camelCase function maps to snake_case"
  verify unit "SCREAMING_SNAKE const maps to lower_snake"
  verify unit "React hook useAuth maps to auth"
  verify unit "React component UserCard maps to user_card"
  verify unit "@specforge JSDoc tag overrides convention"
  verify unit "tests field overrides JSDoc tag"
  verify unit "barrel re-export traces to original source"
  verify unit "name collision across files produces diagnostic"
}

// ── Inference ──────────────────────────────────────────────────

behavior infer_entity_kinds "Infer Entity Kinds from TypeScript Signals" {
  types      [TsInferenceSignal, TsSignalKind, TsSuggestedEntity, TsSourceItem]
  category   query

  contract """
    For each TsSourceItem, the system MUST compute inference signals that
    suggest which specforge entity kind it maps to. Signals are framework-
    aware: NestJS @Controller suggests behavior, interfaces suggest port,
    Zod schemas suggest type, event classes suggest event. Each signal has
    a confidence score (0.0-1.0) and a suggested entity kind. Multiple
    signals on the same item MUST be returned; the consuming agent resolves
    conflicts.
  """

  verify unit "service class suggests behavior"
  verify unit "controller class suggests behavior"
  verify unit "handler function suggests behavior"
  verify unit "factory function suggests behavior"
  verify unit "middleware function suggests behavior"
  verify unit "interface suggests port"
  verify unit "abstract class suggests port"
  verify unit "Zod schema suggests type"
  verify unit "io-ts codec suggests type"
  verify unit "type alias suggests type"
  verify unit "enum suggests type"
  verify unit "event class suggests event"
  verify unit "subscriber class suggests event consumer"
  verify unit "error class with invariant-like name suggests invariant"
  verify unit "config schema suggests invariant"
  verify unit "guard/pipe/interceptor suggests behavior"
  verify unit "GraphQL resolver suggests behavior"
  verify unit "React component suggests behavior"
  verify unit "custom hook suggests behavior"
}

// ── Source Anchoring ───────────────────────────────────────────

behavior anchor_entity_to_source "Anchor Entity to Source" {
  types      [TsSourceAnchor, TsSecondaryLocation, TsSecondaryRole]
  category   query
  ports      [TsSourceScanner]
  invariants [ts_barrel_resolution_correctness]

  contract """
    Given a specforge entity ID, the system MUST locate the primary source
    definition across all .ts/.tsx/.js/.jsx files. When a symbol is
    re-exported through barrels, the primary location MUST be the original
    definition, with barrel files listed as secondary locations. Interface
    merging (declaration merging across files) and module augmentation
    MUST produce secondary locations.
  """

  verify unit "entity ID resolves to primary source file and line"
  verify unit "barrel re-export resolves to original definition"
  verify unit "interface merging produces secondary locations"
  verify unit "module augmentation produces secondary location"
  verify unit "missing entity ID returns not-found diagnostic"
}

// ── Test Collection ────────────────────────────────────────────

behavior collect_jest_results "Collect Jest Test Results" {
  types      [SpecforgeReport, TestResultEntry, TsTestFormat, TsTestsCollectedPayload]
  category   command
  ports      [TsTestOutputParser, FileSystem]
  invariants [ts_entity_mapping_precedence]
  produces   [ts_tests_collected]

  contract """
    The system MUST parse Jest JSON output (--json flag) and map test
    results to specforge entity IDs. Jest organizes tests as describe/it
    blocks; the entity mapping uses the describe block name as a hint
    and the @specforge JSDoc tag or naming convention for resolution.
    The output MUST conform to the SpecforgeReport schema.
  """

  verify unit "Jest JSON output is parsed"
  verify unit "describe block maps to entity context"
  verify unit "it/test block maps to individual test"
  verify unit "passed/failed/skipped statuses are captured"
  verify unit "test duration is captured"
  verify unit "output conforms to SpecforgeReport schema"
}

behavior collect_vitest_results "Collect Vitest Test Results" {
  types      [SpecforgeReport, TestResultEntry, TsTestFormat, TsTestsCollectedPayload]
  category   command
  ports      [TsTestOutputParser, FileSystem]
  invariants [ts_entity_mapping_precedence]
  produces   [ts_tests_collected]

  contract """
    The system MUST parse Vitest JSON output (--reporter=json). Vitest
    output format is compatible with Jest JSON but includes additional
    metadata (benchmark results, type-check results). The system MUST
    handle both vitest run and vitest bench output.
  """

  verify unit "Vitest JSON output is parsed"
  verify unit "Vitest benchmark results are handled"
  verify unit "output conforms to SpecforgeReport schema"
}

behavior collect_playwright_results "Collect Playwright Test Results" {
  types      [SpecforgeReport, TestResultEntry, TsTestFormat, TsTestsCollectedPayload]
  category   command
  ports      [TsTestOutputParser, FileSystem]
  invariants [ts_entity_mapping_precedence]
  produces   [ts_tests_collected]

  contract """
    The system MUST parse Playwright JSON reporter output. Playwright
    organizes tests with spec files, describe blocks, and test.step
    annotations. Entity mapping uses file path and describe name as
    signals. The system MUST handle multi-browser test results (same
    test run in chromium, firefox, webkit).
  """

  verify unit "Playwright JSON reporter output is parsed"
  verify unit "multi-browser results are deduplicated"
  verify unit "test.step annotations are captured"
  verify unit "output conforms to SpecforgeReport schema"
}

behavior collect_cypress_results "Collect Cypress Test Results" {
  types      [SpecforgeReport, TestResultEntry, TsTestFormat]
  category   command
  ports      [TsTestOutputParser, FileSystem]
  invariants [ts_entity_mapping_precedence]
  produces   [ts_tests_collected]

  contract """
    The system MUST parse Cypress test results from JUnit XML or
    Mochawesome JSON reporter output. Cypress organizes tests as
    describe/it blocks within spec files.
  """

  verify unit "Cypress JUnit XML is parsed"
  verify unit "Cypress Mochawesome JSON is parsed"
  verify unit "output conforms to SpecforgeReport schema"
}

behavior merge_monorepo_reports "Merge Monorepo Test Reports" {
  types      [SpecforgeReport, TestResultEntry, TsMonorepoInfo]
  category   command
  ports      [FileSystem]

  contract """
    In a monorepo, the system MUST merge test results from multiple
    packages into a single SpecforgeReport. Each package may use a
    different test runner (Jest in one, Vitest in another, Playwright
    for e2e). Duplicate entity results across packages MUST be merged
    with the most recent result winning. Package-scoped entity IDs
    MUST be resolved to project-global IDs.
  """

  verify unit "results from multiple packages are merged"
  verify unit "different test runners per package are handled"
  verify unit "duplicate entities take most recent result"
  verify unit "package-scoped IDs resolved to global IDs"
}
