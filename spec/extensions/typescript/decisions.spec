// @specforge/typescript extension Architecture Decision Records
//
// Decisions specific to the TypeScript/JavaScript language integration:
// scanning approach, entity ID mapping, test collection, framework support.

use "extensions/typescript/invariants"

decision tree_sitter_for_scanning "Tree-sitter for TypeScript Scanning" {
  status   accepted
  date     2026-04-24

  context """
    The source scanner needs to parse TypeScript/JavaScript files to extract
    exported symbols. Options: (1) tree-sitter-typescript compiled to Wasm,
    (2) ts-morph (TypeScript compiler API) via wasm-bindgen, (3) SWC parser
    compiled to Wasm, (4) regex-based extraction. The scanner runs inside
    the Wasm sandbox, so native Node.js APIs are unavailable.
  """

  decision """
    Use tree-sitter-typescript compiled to Wasm. Tree-sitter provides
    error-tolerant, incremental parsing that works on incomplete files
    and is already used by the specforge core compiler. The TypeScript
    grammar handles .ts, .tsx, .js, .jsx. Type-level accuracy is
    sufficient for signature extraction without full type checking.
  """

  consequences [
    "Consistent with specforge core parser technology",
    "Error-tolerant: partial files still produce results",
    "Incremental: re-scanning after edits is fast",
    "No full type resolution (no inferred types from tsc)",
    "JSX/TSX grammar support is mature in tree-sitter",
    "Wasm binary size is manageable (~500KB for TS grammar)",
  ]
}

decision jsdoc_tag_for_entity_mapping "JSDoc @specforge Tag for Entity Mapping" {
  status   accepted
  date     2026-04-24

  context """
    TypeScript lacks a proc macro system like Rust's #[specforge::test].
    Developers need an explicit, non-invasive way to link source symbols
    to specforge entity IDs. Options: (1) JSDoc tag, (2) magic comments,
    (3) companion .specforge.json files, (4) TypeScript decorators.
  """

  decision """
    Use a JSDoc @specforge tag: /** @specforge entity_id */. JSDoc is
    already widely used in TypeScript projects, is preserved by all
    build tools, works in .js files without TypeScript, and is
    understood by IDEs for hover documentation. Decorators were rejected
    because they require runtime code and are not available in .js files.
  """

  consequences [
    "Zero runtime overhead: JSDoc is stripped at compile time",
    "Works in both .ts and .js files",
    "IDE hover shows the specforge entity link",
    "Does not require TypeScript decorators feature flag",
    "Established convention: similar to @see, @link, @module",
    "Less discoverable than decorators for developers unfamiliar with JSDoc",
  ]

  invariants [ts_entity_mapping_precedence]
}

decision pascal_to_snake_convention "PascalCase to snake_case Entity ID Convention" {
  status   accepted
  date     2026-04-24

  context """
    TypeScript uses PascalCase for classes/interfaces/components and
    camelCase for functions/variables. Specforge entity IDs are snake_case.
    The conversion must be deterministic and reversible-enough for humans.
  """

  decision """
    Apply standard PascalCase/camelCase to snake_case conversion:
    UserService -> user_service, validateInput -> validate_input,
    HTTPClient -> http_client, MAX_RETRIES -> max_retries. Acronyms
    follow Rust-style splitting: consecutive capitals are treated as
    a single word (HTTP -> http) except when followed by lowercase
    (HTTPClient -> http_client). React hooks drop the "use" prefix:
    useAuth -> auth, useUserProfile -> user_profile.
  """

  consequences [
    "Deterministic: same symbol always produces same entity ID",
    "Human-readable: snake_case IDs are easy to write in .spec files",
    "Acronym handling matches developer expectations (HTTP -> http)",
    "Hook prefix removal is opinionated but reduces noise",
    "May collide: Auth class and useAuth hook both map to 'auth' -- collision produces diagnostic",
  ]

  invariants [ts_entity_mapping_precedence]
}

decision multi_runner_test_collection "Multi-Runner Test Collection" {
  status   accepted
  date     2026-04-24

  context """
    The TypeScript ecosystem has many test runners: Jest, Vitest, Mocha,
    Playwright, Cypress, Node.js built-in test runner. A monorepo may use
    different runners for different packages. The extension must handle all
    of them.
  """

  decision """
    Support multiple test runners through format-specific parsers:
    jest-json, vitest-json, playwright-json, junit-xml (Cypress/Mocha),
    mocha-json, and TAP (Node.js test runner). Each parser is a separate
    Wasm export. The collect command auto-detects the runner from config
    files or accepts an explicit --runner flag. In monorepos, per-package
    runner detection is used.
  """

  consequences [
    "Covers the vast majority of TS/JS test setups",
    "Auto-detection reduces user friction",
    "Per-package runner detection handles mixed monorepos",
    "Each parser is independently testable",
    "New runners can be added without changing the collection pipeline",
  ]
}

decision framework_aware_inference "Framework-Aware Inference Signals" {
  status   accepted
  date     2026-04-24

  context """
    Generic TypeScript analysis misses domain signals that framework
    conventions provide. A NestJS @Controller is a stronger behavior
    signal than a plain exported function. An Angular @Injectable is
    a service pattern. The inference system needs framework context.
  """

  decision """
    The scanner detects frameworks (from package.json + config files)
    and uses framework-specific heuristics to boost inference signals.
    Framework detection is done once per project scan. Inference signals
    carry the framework as context. Signal confidence is adjusted based
    on framework presence (e.g., @Controller in a NestJS project gets
    0.95 confidence; in a non-NestJS project gets 0.5).
  """

  consequences [
    "Higher inference accuracy for framework-heavy codebases",
    "Framework detection is fast (file existence checks)",
    "Inference signals remain suggestions, never overrides",
    "New frameworks can be added without changing core inference logic",
    "Framework-unaware projects still get useful generic signals",
  ]
}
