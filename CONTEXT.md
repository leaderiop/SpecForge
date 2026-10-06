# SpecForge domain terms

Terms the code, the specs and the docs use with one meaning. Architecture decisions are in
`docs/adr/`; ADR 0004 records the one-project migration these terms come from.

- **Environment**: everything derived from `specforge.json` and the loaded extensions before any
  `.spec` file is read: config, spec root, registries, rules, surfaces, and load diagnostics
  (`specforge_project::Environment`).
- **Compiled project**: an environment plus the resolved sources and the built graph. Its
  diagnostics are, by definition, what `specforge check` reports under the default policy
  (`specforge_project::CompiledProject`).
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
- **Extension declaration**: everything one extension declares — its handshake and every describe
  category — as the protocol types (`specforge_protocol_types::ExtensionDeclaration`). The SDK
  builds it, the guest serves it, the host loads it once, the Registry build reads it, a package
  registry stores it (ADR 0012).
- **Registry build**: the pure result of turning extension declarations into kind, field and edge
  registries, rules, pass order and derived graph inputs, and the diagnostics of those
  declarations (`specforge_registry::build_registries`).
- **Package registry client**: what talks to a package registry: search, resolve and publish over
  HTTP, credentials in the OS keyring, publisher trust and package signing
  (`specforge-registry-client`). Not the Registry build, which is pure and needs none of it.
  Operations reach it only through the `Registry` port; its adapter (`specforge-ops-registry`)
  is linked by the CLI and MCP, never the LSP (ADR 0010). Publish derives the stored declaration
  from the binary; `add` checks the binary declares what was published (ADR 0012).
- **Project view**: the read-only slice of a compiled project every operation reads: the graph, the
  registry build (kinds, fields, edges, rules, the extension declarations and their ordered
  passes) and the root the project was compiled from, borrowed (`specforge_ops::view::ProjectView`).
  It owns the project's recorded test report and its
  versioned schema, both read at that root and never an ancestor's. The CLI builds one from its
  compiled project (`ProjectView::of`); MCP from its call target (`ProjectRef::view`: the project
  session, or another project compiled for one call); the LSP from its session
  (`ProjectView::of_session`) (ADR 0015).
- **Read view**: an operation that only reads the project view: stats, trace, the coverage view, the
  model and outline diagrams, the versioned schema. Each returns a typed outcome; the CLI and MCP
  only render it.
- **Recorded test report**: `<root>/specforge-report.json`, what `specforge collect` last wrote. The
  project view reads it once per compile and per content
  (`specforge_project::coverage::RecordedCoverage`).
- **Obligation**: one `verify` statement on an entity. **Proven** when a passing test names its
  exact text, or a formal claim discharges it.
- **Unverified**: an entity that counts toward coverage and is not proven
  (`ProjectCoverage::is_unverified`).
- **Missing link**: an expected edge, from the registries, that a traced entity lacks
  (`MissingLink`). The only gap a trace reports.
- **Plan gap**: how an agent plan falls short of the graph: an unresolved entry, a missing entry for
  a testable entity with obligations, or an entry ordered before what it depends on (`PlanGap`). An
  edge to an entity that does not exist is neither: it is E003.
- **Verdict**: an entity's obligations, the proven ones, and the tests that bear on them. It gives
  both "proven" (the gate) and the covered/partial/uncovered status (the MCP view)
  (`specforge_coverage::Verdict`).
- **Operation**: one user-level command (init, add, remove, …) as a typed request and outcome,
  independent of surface. The CLI and MCP are adapters over it (`specforge-ops`).
- **Check**: the operation that turns what a compile reported into what a surface reports: the
  diagnostic policy (lint profiles, then strict), the verdict (no error among everything reported),
  the severity filter (what is shown, never the verdict) and the opt-in build-cache record
  (`specforge_ops::check`). `specforge check` and MCP `specforge.validate` are its adapters; watch and
  the LSP report a compile's diagnostics without a policy (ADR 0018).
- **Diagnostic policy**: lint profiles (a closed set: `inferred`, `pedantic`) and strict promotion
  (`specforge_project::DiagnosticPolicy`).
- **Extension command**: a CLI command an extension declares in its surfaces (with the SDK, together
  with its handler: `ContributionsBuilder::command`), answered by its `cmd__` export over the graph
  the host passes (`specforge_protocol_types::CommandInput`: args, project root, graph, the
  command format and today's date, UTC). The CLI runs it as `specforge <short> <command>`, MCP as
  the auto-promoted tool `specforge.<short>.<id>`, `short` being the declaration's (`ext_short`,
  else its name's last segment); neither knows any command (ADR 0008). One derivation serves both
  surfaces (`specforge_ops::command::ExtensionCommand`): its CLI name, its tool name, its args'
  command-line shapes, its input schema and the args both send, normalized by the one arg rule the
  SDK also runs (`specforge_protocol_types::command_args`): declared defaults applied by the host,
  an unset flag `false`, each value its declared type (ADR 0017).
- **Extension surface table**: what MCP serves from the project's extensions, built once per
  reload from their declarations: each tool once (an explicit `mcp__` tool, or an extension
  command), each resource with its URI template; listings are the core tables plus it, and a call
  is one lookup in it. A contribution it does not serve is reported with I017
  (`specforge_mcp`'s `ExtensionSurfaceTable`, ADR 0017).
- **Extension call**: one typed operation the host performs on a loaded extension — handshake,
  describe, command, MCP tool, MCP resource, compiler pass, collector, custom validator, scanner,
  migration hook — over the `WasmRuntime` port. Its input and answer are protocol types
  (`specforge_protocol_types`) the SDK shares; every failure is one `CallError`, E028, naming the
  operation, the export and the extension (`specforge_wasm::calls::ExtensionCalls`, ADR 0013).
- **In-process runtime**: the test adapter of the `WasmRuntime` port that runs an SDK-declared
  extension in the host process through the guest's own routing (`guest_call`), unsandboxed
  (`specforge_wasm::testing::InProcessRuntime`). Host tests declare their extensions with it; the
  component runtime is the production adapter, and both keep one contract
  (`assert_runtime_contract`).
- **Command format**: the output an extension command is asked for, `human` (the CLI default) or
  `json` (always, over MCP). The host owns the `--format` flag; the extension renders both, since
  only it knows its payloads (ADR 0011).
- **Tool spec**: the single definition of an MCP tool, from which its descriptor, typed arguments,
  output schema, annotations, its target (reach and freshness), mutation event and reply are derived
  (`specforge_mcp`'s `ToolSpec` table).
- **Prompt spec**: the single definition of an MCP prompt, from which its descriptor, typed arguments
  and reply are derived; it renders over the call target and refuses with an McpError, sent as a
  JSON-RPC error's data since prompts have no isError (`specforge_mcp`'s `PromptSpec` table).
- **Stateless request**: an MCP request whose `_meta` names its protocol version (MCP 2026-07-28),
  answered on its own without `initialize`; every other request follows the revision `initialize`
  negotiated (`specforge_mcp::modern`).
- **Diagnostic catalog**: the one registry of diagnostic codes: each code's title, owner, level and
  explanation (`specforge_diagnostics::CATALOG`). `specforge explain`, MCP `specforge.explain`,
  diagnostics JSON titles, doctor and the LSP hover all read it; `docs/diagnostics.md` is generated
  from it.
- **Diagnostic data**: a diagnostic's optional typed payload, the values its message names
  (`specforge_common::DiagnosticData`, e.g. an E003's unresolved target, a W061's cycle, the entity
  an extension pass named). Consumers that act on a diagnostic (the LSP's quick fixes, MCP's
  suggest_fixes, and navigation's attribution of a diagnostic to the entities it is about) read it;
  none parses the message, which is presentation.
- **Reference**: an occurrence of an entity's ID in another entity's field that resolves to it (one
  edge, written where its token is). The references *to* an entity are incoming; what an entity
  *refers to* are its outgoing references. "Find references" means incoming, with the declaration
  only on request (`specforge_ops::navigate`).
- **Navigation**: where an entity is declared, its references, entity lookup and ranking, which
  entities a diagnostic is about, and the fixes a diagnostic's data names. The LSP and MCP answer
  from one module (`specforge_ops::navigate`) in source spans. The LSP converts them to UTF-16
  ranges through each text's line index, MCP renders them as JSON (ADR 0016, ADR 0023).
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
