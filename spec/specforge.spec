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

  // Personas, channels, and coverage are defined in their respective files:
  // product/personas.spec, product/channels.spec
}
