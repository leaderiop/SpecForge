// SpecForge — Meta-specification: SpecForge specifying itself
// The structured context standard for AI agents.
// The compiled entity graph is consumed by any agent for any task.

spec "specforge" {
  version "1.0"

  extensions [
    "@specforge/software",
    "@specforge/product",
    "@specforge/governance",
  ]

  providers {
    gh "specforge" {
      extension "@specforge/gh"
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
    description "Automated agent running specforge check, coverage gates, and graph validation"
  }

  persona contributor "Open Source Contributor" {
    description "Extends SpecForge via extensions with entity, provider, or renderer contributions"
  }

  persona agent "AI Agent" {
    description "Any AI agent consuming the Graph Protocol for any task: coding, PM, compliance, docs, security"
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

  surface graph_protocol "Graph Protocol" {
    type api
  }

  surface mcp "MCP Server" {
    type api
  }

  test_dirs ["tests/"]

  coverage {
    threshold                90
    require_violation_tests  true
    fail_on_unknown_ids      true
  }
}
