# SpecForge domain terms

Terms the code, the specs and the docs use with one meaning. Architecture decisions are in
`docs/adr/`; ADR 0004 records the one-project migration these terms come from.

- **Environment**: everything derived from `specforge.json` and the loaded extensions before any
  `.spec` file is read: config, spec root, registries, rules, surfaces, and load diagnostics
  (`specforge_project::Environment`).
- **Compiled project**: an environment plus the resolved sources and the built graph. Its
  diagnostics are, by definition, what `specforge check` reports (`specforge_project::CompiledProject`).
- **Project session**: a long-lived compiled project that accepts source changes and environment
  reloads. Watch, the LSP and MCP each hold one (`specforge_project::ProjectSession`).
- **Registry build**: the pure result of turning extension manifests into kind, field and edge
  registries, rules and derived graph inputs (`specforge_registry::build_registries`).
- **Obligation**: one `verify` statement on an entity. **Proven** when a passing test names its
  exact text, or a formal claim discharges it.
- **Verdict**: an entity's obligations, the proven ones, and the tests that bear on them. It gives
  both "proven" (the gate) and the covered/partial/uncovered status (the MCP view)
  (`specforge_coverage::Verdict`).
- **Operation**: one user-level command (init, add, remove, …) as a typed request and outcome,
  independent of surface. The CLI and MCP are adapters over it (`specforge-ops`).
