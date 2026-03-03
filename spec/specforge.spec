// SpecForge — Meta-specification: SpecForge specifying itself
// Project root configuration

spec "specforge" {
  version "1.0"

  plugins [
    "@specforge/product",
    "@specforge/governance",
  ]

  providers {
    gh "specforge" {
      package "@specforge/gh"
      repo    "anthropics/specforge"
    }
  }

  persona developer "Specification Author" {
    description "Writes and maintains .spec files, runs the compiler, reviews diagnostics"
  }

  persona architect "System Architect" {
    description "Designs cross-cutting concerns, reviews traceability, manages governance entities"
  }

  persona ci "CI Pipeline" {
    description "Automated agent running specforge check, coverage gates, and drift detection"
  }

  persona contributor "Open Source Contributor" {
    description "Extends SpecForge via plugins, providers, or generators"
  }

  surface cli "Command Line Interface" {
    type terminal
  }

  surface ide "IDE with LSP" {
    type editor
  }

  surface ci_surface "CI/CD Pipeline" {
    type automation
  }

  test_dirs ["tests/"]

  coverage {
    threshold                90
    require_violation_tests  true
    fail_on_unknown_ids      true
  }

  gen typescript {
    out       "packages/generated/"
    result    "hex-di"
    readonly  true
    naming    "camelCase"
    tests     "@specforge/vitest"
  }

  gen rust {
    out       "src/generated/"
    test_out  "tests/spec/"
    result    "thiserror"
    naming    "snake_case"
    tests     "@specforge/rust"
  }
}
