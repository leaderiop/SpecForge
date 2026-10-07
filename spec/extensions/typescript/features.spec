// @specforge/typescript extension features

use "extensions/typescript/behaviors"

feature ts_source_scanning "TypeScript Source Scanning" {
  status   in_progress
  problem  """
    AI agents working on TypeScript/JavaScript codebases need structured
    knowledge of all exported symbols -- their names, signatures, locations,
    types, and documentation. Without this, agents must read entire files
    to understand what a module provides, wasting tokens and reducing
    accuracy. The TypeScript ecosystem's variety (classes, functions,
    interfaces, React components, hooks, decorators) makes manual
    extraction unreliable.
  """
  solution """
    specforge scan typescript walks the project, classifies files by role,
    and extracts every exported symbol as a structured TsSourceItem with
    full metadata: kind, signature, generics, JSDoc, decorators, React
    info, and inference signals. The output is a TsScanResult that agents
    consume directly via the graph protocol.
  """
}

feature ts_entity_mapping "TypeScript Entity ID Mapping" {
  status   in_progress
  problem  """
    TypeScript naming conventions (PascalCase classes, camelCase functions,
    SCREAMING_SNAKE constants) don't match specforge's snake_case entity
    IDs. Developers need automatic, deterministic conversion with explicit
    override support for edge cases.
  """
  solution """
    Two-level mapping precedence: (1) @specforge JSDoc tag, (2)
    PascalCase/camelCase to snake_case naming convention. Barrel re-exports trace to original source.
    Collisions produce diagnostics. React hooks drop the "use" prefix.
  """
}

feature ts_test_collection "TypeScript Test Collection" {
  status   done
  problem  """
    The TypeScript ecosystem has fragmented test tooling: Jest, Vitest,
    Playwright, Cypress, Mocha, Node.js test runner. Each produces
    different output formats. Monorepos often mix runners across packages.
    Specforge needs a single specforge-report.json regardless of runner.
  """
  solution """
    One runner extension per test runner (ADR 0002), not one parser per
    format inside this source analyzer. @specforge/vitest ships today:
    tests name their entity in vitest's test metadata, and `specforge
    collect` runs vitest with the user's consent. Jest, Playwright,
    Cypress, Mocha and the Node.js test runner are future runner
    extensions; several can be enabled in one project, and collect merges
    their results per runner.
  """
}

feature ts_monorepo_support "TypeScript Monorepo Support" {
  status   proposed
  problem  """
    Large TypeScript projects use monorepo tools (npm/yarn/pnpm workspaces,
    Nx, Turborepo, Lerna, Rush) that split code across packages. Scanning
    must understand package boundaries, internal dependencies, and
    per-package configuration to produce accurate results.
  """
  solution """
    The scanner auto-detects monorepo tooling from root config files,
    enumerates packages from workspace globs, resolves internal
    cross-package dependencies, and scans each package respecting its
    own tsconfig.json. Entity IDs are project-global (not package-scoped).
  """
}

feature ts_framework_inference "TypeScript Framework-Aware Inference" {
  status   in_progress
  problem  """
    Framework conventions (NestJS controllers, Angular services, React
    components) carry strong domain signals about what specforge entity
    kind a symbol should map to. Generic analysis misses these signals,
    reducing inference accuracy.
  """
  solution """
    The scanner detects frameworks from package.json and config files,
    then uses framework-specific heuristics to produce inference signals
    with confidence scores. NestJS @Controller -> behavior (0.95),
    interface -> port (0.85), Zod schema -> type (0.90), event class
    -> event (0.85). Agents consume signals to auto-suggest entity
    kinds during specforge init or spec authoring.
  """
}
