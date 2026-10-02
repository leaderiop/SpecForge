# SpecForge domain terms

Terms the code, the specs and the docs use with one meaning. Architecture decisions are in
`docs/adr/`; ADR 0004 records the one-project migration these terms come from.

- **Environment**: everything derived from `specforge.json` and the loaded extensions before any
  `.spec` file is read: config, spec root, registries, rules, surfaces, and load diagnostics
  (`specforge_project::Environment`).
- **Compiled project**: an environment plus the resolved sources and the built graph. Its
  diagnostics are, by definition, what `specforge check` reports (`specforge_project::CompiledProject`).
- **Project session**: a long-lived compiled project that accepts source changes and environment
  reloads. Watch, the LSP and MCP each hold one (`specforge_project::ProjectSession`). MCP serves its
  project through it: every fresh compile (validate, analyze, doctor, collect, a mutation, a refresh
  after watch writes a newer snapshot) is an environment reload, which re-reads every source.
- **Update**: one change applied to a project session. It re-parses exactly the changed files (an
  importer parses the same, since references resolve without `use`), patches the graph, resolves
  every file's imports again and re-runs the checks (`specforge_project::Update`, ADR 0006).
- **Graph delta**: what an update or a reload changed in the graph: added, removed and modified
  nodes (source positions ignored) and edges. Watch prints it and MCP notifies it
  (`specforge_project::GraphDelta`).
- **Registry build**: the pure result of turning extension manifests into kind, field and edge
  registries, rules and derived graph inputs (`specforge_registry::build_registries`).
- **Package registry client**: what talks to a package registry: search, resolve and publish over
  HTTP, credentials in the OS keyring, publisher trust and package signing
  (`specforge-registry-client`). Not the Registry build, which is pure and needs none of it.
- **Project view**: the read-only slice of a compiled project an operation analyses: the graph, the
  kind and field registries, the rules, the manifests and the project root, borrowed
  (`specforge_ops::analyze`'s input). The CLI builds one from its compiled project; MCP builds one from
  its project session (`McpState::project_view`), or from another project compiled for one call.
- **Obligation**: one `verify` statement on an entity. **Proven** when a passing test names its
  exact text, or a formal claim discharges it.
- **Verdict**: an entity's obligations, the proven ones, and the tests that bear on them. It gives
  both "proven" (the gate) and the covered/partial/uncovered status (the MCP view)
  (`specforge_coverage::Verdict`).
- **Operation**: one user-level command (init, add, remove, …) as a typed request and outcome,
  independent of surface. The CLI and MCP are adapters over it (`specforge-ops`).
- **Extension command**: a CLI command an extension declares in its surfaces, answered by its
  `cmd__` export over the graph the host passes (`CommandInput`: args, project root, graph). The
  CLI runs it as `specforge <ext_short> <command>`, MCP as the auto-promoted tool
  `specforge.<ext_short>.<id>`; neither knows any command (`specforge_ops::command`, ADR 0008).
- **Tool spec**: the single definition of an MCP tool, from which its descriptor, typed arguments,
  output schema, annotations, mutation event and reply are derived (`specforge_mcp`'s `ToolSpec` table).
- **Stateless request**: an MCP request whose `_meta` names its protocol version (MCP 2026-07-28),
  answered on its own without `initialize`; every other request follows the revision `initialize`
  negotiated (`specforge_mcp::modern`).
- **Diagnostic catalog**: the one registry of diagnostic codes: each code's title, owner, level and
  explanation (`specforge_diagnostics::CATALOG`). `specforge explain`, MCP `specforge.explain`,
  diagnostics JSON titles, doctor and the LSP hover all read it; `docs/diagnostics.md` is generated
  from it.
