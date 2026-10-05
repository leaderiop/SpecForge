# SpecForge domain terms

Terms the code, the specs and the docs use with one meaning. Architecture decisions are in
`docs/adr/`; ADR 0004 records the one-project migration these terms come from.

- **Environment**: everything derived from `specforge.json` and the loaded extensions before any
  `.spec` file is read: config, spec root, registries, rules, surfaces, and load diagnostics
  (`specforge_project::Environment`).
- **Compiled project**: an environment plus the resolved sources and the built graph. Its
  diagnostics are, by definition, what `specforge check` reports (`specforge_project::CompiledProject`).
- **Project session**: a long-lived compiled project that knows what it is built from (its
  sources, its **environment inputs** — `specforge.json`, `specforge.lock`, the extension modules it
  loaded — and its check inputs, `specforge-cache.json` and the files `file_reference` fields name).
  It classifies any changed path, applies changes as an update, an environment reload or a re-check,
  and can bring itself up to date with disk without a watcher (`ensure_fresh`). Watch, the LSP and
  MCP each hold one (`specforge_project::ProjectSession`); watch and the LSP feed it watcher events,
  MCP asks it to be fresh before every request that reads the project (ADR 0014).
- **Call target**: the project one MCP call acts on, resolved from the call's optional `path` and its
  tool spec's target (reach and freshness) before the handler runs: the served session (brought up to
  date unless `use_cached`), another project compiled for that call only, or the directory `init`
  creates (`specforge_mcp::target::CallTarget`). Handlers read it as a `ProjectRef` (ADR 0014).
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
  Operations reach it only through the `Registry` port; its adapter (`specforge-ops-registry`)
  is linked by the CLI and MCP, never the LSP (ADR 0010).
- **Project view**: the read-only slice of a compiled project an operation analyses: the graph, the
  kind and field registries, the rules, the manifests and the project root, borrowed
  (`specforge_ops::analyze`'s input). The CLI builds one from its compiled project; MCP builds one from
  its call target (`ProjectRef::view` of its call target).
- **Obligation**: one `verify` statement on an entity. **Proven** when a passing test names its
  exact text, or a formal claim discharges it.
- **Verdict**: an entity's obligations, the proven ones, and the tests that bear on them. It gives
  both "proven" (the gate) and the covered/partial/uncovered status (the MCP view)
  (`specforge_coverage::Verdict`).
- **Operation**: one user-level command (init, add, remove, …) as a typed request and outcome,
  independent of surface. The CLI and MCP are adapters over it (`specforge-ops`).
- **Extension command**: a CLI command an extension declares in its surfaces (with the SDK, together
  with its handler: `ContributionsBuilder::command`), answered by its `cmd__` export over the graph
  the host passes (`CommandInput`: args, project root, graph, the
  command format and today's date, UTC). The CLI runs it as `specforge <ext_short> <command>`, MCP as
  the auto-promoted tool `specforge.<ext_short>.<id>`; neither knows any command
  (`specforge_ops::command`, ADR 0008).
- **Command format**: the output an extension command is asked for, `human` (the CLI default) or
  `json` (always, over MCP). The host owns the `--format` flag; the extension renders both, since
  only it knows its payloads (ADR 0011).
- **Tool spec**: the single definition of an MCP tool, from which its descriptor, typed arguments,
  output schema, annotations, its target (reach and freshness), mutation event and reply are derived
  (`specforge_mcp`'s `ToolSpec` table).
- **Stateless request**: an MCP request whose `_meta` names its protocol version (MCP 2026-07-28),
  answered on its own without `initialize`; every other request follows the revision `initialize`
  negotiated (`specforge_mcp::modern`).
- **Diagnostic catalog**: the one registry of diagnostic codes: each code's title, owner, level and
  explanation (`specforge_diagnostics::CATALOG`). `specforge explain`, MCP `specforge.explain`,
  diagnostics JSON titles, doctor and the LSP hover all read it; `docs/diagnostics.md` is generated
  from it.
- **Diagnostic data**: a diagnostic's optional typed payload, the values its message names
  (`specforge_common::DiagnosticData`, e.g. an E003's unresolved target). Consumers that act on a
  diagnostic, such as the LSP's quick fixes, read it; none parses the message, which is presentation.
- **Proof role**: what a field's value is to the prove pass, declared by its extension
  (`proof_role`): a **bound** the solver assumes (bounds must be consistent, E046) or a **claim**
  that must follow from the bounds (W139 when not; an entailed claim is a proved claim). A field
  with no role is not read by the prove pass (ADR 0009).
- **Risk grading**: the coverage owner's policy for one kind: its entities' risk is tallied, and
  one with no obligations is A002, an error at the grading's error level
  (`specforge_coverage::RiskGrading`, supplied by `@specforge/testing`; ADR 0009).
- **Lifecycle field**: the one field of a kind that holds its entities' lifecycle state
  (`lifecycle_field` on the kind). The build cache records its value so check-phase passes can
  compare against the previous build (ADR 0009).
