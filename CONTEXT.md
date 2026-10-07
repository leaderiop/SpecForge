# SpecForge domain terms

Terms the code, the specs and the docs use with one meaning. Architecture decisions are in
`docs/adr/`; ADR 0004 records the one-project migration these terms come from.

- **Environment**: everything derived from `specforge.json` and the loaded extensions before any
  `.spec` file is read: config, spec root, registries, rules, surfaces, and load diagnostics
  (`specforge_project::Environment`). A `specforge.json` that is there and can't be used is the
  default config (for the unusable file or key), with each reason kept (`config_problems`) and
  reported as the error E069. It also holds what `specforge.lock` held when it was read
  (`lock`: absent, read, or unreadable), once, for every operation over the project. A session
  opens in two steps (`ProjectSession::begin_open`, then `OpeningProject::finish`), so an editor
  answers what needs only the environment (keyword completion) while the sources are still being
  read.
- **Compiled project**: an environment plus the resolved sources and the built graph. Its
  diagnostics are, by definition, what `specforge check` reports under the default policy
  (`specforge_project::CompiledProject`).
- **Project session**: a long-lived compiled project that knows what it is built from (its
  sources, its **environment inputs** — `specforge.json`, `specforge.lock`, the extension modules it
  loaded — and its check inputs, `specforge-cache.json` and the files `file_reference` fields name).
  It classifies any changed path, applies changes as an update, an environment reload or a re-check,
  and can bring itself up to date with disk without a watcher (`ensure_fresh`). Watch, the LSP and
  MCP each hold one (`specforge_project::ProjectSession`; MCP's is always opened from disk, ADR
  0025); watch and the LSP feed it watcher events, MCP asks it to be fresh before every request that
  reads the project (ADR 0014).
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
- **Registry build**: the pure result of turning extension declarations into kind, field and
  edge registries, the rule set, pass order and derived graph inputs, and the diagnostics of those
  declarations (`specforge_registry::build_registries`). Its outcomes are the `registry_build_*`
  behaviors. Tests and every caller reach it only through `build_registries`; its steps are
  private.
- **Rule set**: the extensions' declared validation rules plus the host's E006 rules for required
  fields, each turned once per registry build into a typed rule that carries only what its check
  reads, resolved against the registries (a compiled regex, an edge rule's peer kind, the fields an
  edge type is written as). It runs itself over the entity snapshot's records (`RuleInput`, ADR
  0019), cycles included, and answers which rule applies to a kind, which the snapshot's standing
  reads (`specforge_registry::rules::Rules`, ADR 0020). A declared rule that cannot work is W112; a
  property its check does not read is W147.
- **Custom verdict**: an extension's answer, through its `wasm_function`, on one entity for a
  `check: "custom"` rule: pass, or fail naming a field and value. The rule set asks for it through
  the `CustomVerdicts` port; the project's adapter calls the extension (`ExtensionCalls::validate`),
  tests answer with a closure. A function that cannot answer is W112 at load, W148 at check.
- **Entity snapshot**: every entity of one built graph as every check after the build reads it, taken
  once per compile and per session check (`specforge_project::snapshot::EntitySnapshot`, ADR 0019).
  Each entity's record holds:
  - what it writes, as field text;
  - its references, obligations and methods;
  - its edge counts by peer kind;
  - what exempts it, if anything (a union body, an exempting flag, a kind that accepts no `verify`).

  The registry checks and the rules read the records (`specforge_registry::entity::EntityRecord`).
  The pass input, a custom validator's context and the coverage rule's entities are adapters over it.
  Never read a graph node to answer what an entity says or owes.
- **Field text**: the one string a field value is to a rule, a custom validator and a compiler pass
  (ADR 0019): scalars as written; lists of strings or references and mixed lists joined by `", "`;
  variant lists and type unions by `" | "`; expressions by `", "`; verify statements by `"; "`; a
  block's keys by `", "`. An empty value is `""`. Every written field has one, and none is null.
- **Standing**: how the obligation rule sees an entity (`specforge_project::snapshot::Standing`):
  - whether its kind is testable;
  - which `no_verify_statements` rule requires its kind to declare obligations (a rule without a
    target kind applies to every kind);
  - what exempts it (a union body, an exempting flag, a kind that accepts no `verify` statements);
  - how many obligations it declares.

  It **owes** obligations when a rule applies and nothing exempts it. It **counts** toward coverage
  when testable and owing or declaring. It is **exempt** when testable and neither. W004, the pass
  input's `exempt`, the coverage rule, stats, plan validation and the verify-stub fix read it.
- **Package registry client**: what talks to a package registry: search, resolve and publish over
  HTTP, credentials in the OS keyring, publisher trust and package signing
  (`specforge-registry-client`). Not the Registry build, which is pure and needs none of it.
  Operations reach it only through the `Registry` port; its adapter (`specforge-ops-registry`)
  is linked by the CLI and MCP, never the LSP (ADR 0010). Publish derives the stored declaration
  from the binary; `add` checks the binary declares what was published (ADR 0012).
- **Project view**: the compiled project as one surface sees it, borrowed: the graph, the
  environment it was compiled in (config, what each `extensions` entry enabled, the registry build:
  kinds, fields, edges, rules, the extension declarations and their ordered passes), the root it was
  compiled from, and what the surface reports for it; its entity snapshot, through the per-compile
  memo (`ProjectView::entities`) (`specforge_ops::view::ProjectView`). It owns
  the project's recorded test report and its versioned schema, both read at that root and never an
  ancestor's. The CLI builds one from its compiled project (`ProjectView::of`); MCP from its call
  target (`ProjectRef::view`: the project session with its I017 notices, or another project
  compiled for one call); the LSP from its session (`ProjectView::of_session`) (ADR 0015).
- **Read view**: an operation that only reads the project view: stats, trace, the coverage view, the
  model and outline diagrams, the versioned schema, and inspect. Each returns a typed outcome; the
  CLI, MCP and the LSP only render it.
- **Entity facts**: what inspect returns for one entity: its node and kind entry, headline
  statement, standing (the snapshot's own, borrowed), obligations, references in both directions,
  coverage, and the reported diagnostics about it (`specforge_ops::inspect::EntityFacts`). MCP
  `specforge.inspect` renders it as JSON and the LSP hover as markdown, so the two cannot disagree.
- **Management operation**: an operation about a project's setup and tooling rather than its
  graph: the extensions and providers listings, doctor, remove, collect, inference progress and
  gaps. Like a read view it takes the project view and a request and returns a typed outcome; unlike
  one it also reads what the view does not own (installed binaries, source files),
  and remove and collect write, at the view's root only. `add`, `update`, `init` and `migrate` are
  operations but not over a view: they run before or instead of a compile (ADR 0015); `add` and
  `update` read `specforge.json` through the compile's own reader and refuse an unusable one with
  the refusal `remove` gives.
- **Recorded test report**: `<root>/specforge-report.json`, what `specforge collect` last wrote. The
  project view reads it once per compile and per content
  (`specforge_project::coverage::RecordedCoverage`).
- **Obligation**: one `verify` statement on an entity. **Proven** when a passing test names its
  exact text, or a formal claim discharges it. Who owes obligations is the entity's standing.
- **Unverified**: an entity that counts toward coverage and is not proven
  (its standing counts and its verdict is not proven; `ProjectCoverage::is_unverified`).
- **Delivery evidence**: what the recorded test report proves of a feature, beside the `status`
  its author declares (a claim): the behaviors that implement it (name it in `features`), how many
  the coverage rule counts proven, their obligations and failing tests. A feature is **proven** when
  at least one behavior implements it and every one is proven. The host passes each entity's score
  to an extension command (`CommandInput.evidence`: none, unreadable, or recorded;
  `specforge_ops::command::evidence`); `@specforge/product` aggregates it per feature
  (`milestone-completion`'s `proven_count`) and its `delivery_evidence` pass reports a done feature
  that is not proven (I071). Status stays the input of every status query (ADR 0039).
- **Lifecycle consistency**: what the product kinds' statuses claim across entities, checked by
  `@specforge/product`'s `lifecycle` pass with every compile: a completed milestone with a feature
  neither done nor deprecated (W154), a done feature depending on an unfinished one (I063), a
  milestone due before its dependency (I064), a shipped deliverable tracking an incomplete milestone
  (I065). Its declarative rules check one entity at a time (ADR 0039).
- **Missing link**: an expected edge, from the registries, that a traced entity lacks
  (`MissingLink`). The only gap a trace reports.
- **Plan gap**: how an agent plan falls short of the graph: an unresolved entry, a missing entry for
  a testable entity with obligations, or an entry ordered before what it depends on (`PlanGap`). An
  edge to an entity that does not exist is neither: it is E003.
- **Verdict**: an entity's obligations, the proven ones, and the tests that bear on them. It gives
  both "proven" (the gate) and the covered/partial/uncovered status (the MCP view)
  (`specforge_coverage::Verdict`).
- **Operation**: one user-level command (init, add, remove, …) as a typed request and outcome,
  independent of surface. An operation that writes names the files it changed on disk in its
  outcome (`specforge_ops::Writes`), recorded where it wrote. It fails with an `OpError` whose kind
  (`OpErrorKind`: invalid input, not found, conflict, …) is decided where it is raised; its code is
  what the CLI prints. The CLI and MCP are adapters over it (`specforge-ops`).
- **Mutation outcome**: what one MCP mutation call wrote: the files its operation changed, the
  entities it changed and the domain event it produces (`specforge_mcp::mutation::Written`), or
  nothing for a preview. The request pipeline's tools adapter alone turns it into the call target's
  refresh, the domain event and `mcp_mutation_completed` (ADR 0022), and the reply's `files_written`, the one place a
  client learns which files the call wrote.
- **Project sources**: the `.spec` files under the spec root (`spec_root`, else the project root) that
  discovery keeps — no skipped directory, no `exclude` entry. What a compile reads, and what format
  and migrate rewrite (`specforge_common::ProjectConfig::spec_files`).
- **Format configuration**: how one `.spec` document is laid out: inside a project, the nearest
  `.specforgefmt.toml` from the document's directory up to its own project root, else the defaults;
  outside any project, the editor's tab settings. `specforge format`, MCP `specforge.format` and the
  LSP all format through `specforge_ops::format::document` (ADR 0021).
- **Option table**: one enumerated argument an operation takes — its listed names in order, the
  aliases it also accepts, a one-line help per name, and its default — beside that operation
  (`specforge_ops::options::OptionTable`; `export::FORMAT`, `model::MODEL_FORMAT`, …). The CLI's
  possible values and MCP's input-schema `enum`/`default` are built from it and both parse with it
  (ADR 0027). A set the project decides (analysis passes, entity kinds) is not one.
- **Check**: the operation that turns what a compile reported into what a surface reports: the
  diagnostic policy (lint profiles, then strict), the verdict (no error among everything reported),
  the severity filter (what is shown, never the verdict) and the opt-in build-cache record
  (`specforge_ops::check`). `specforge check` and MCP `specforge.validate` are its adapters; watch and
  the LSP report a compile's diagnostics without a policy (ADR 0018).
- **Diagnostic policy**: lint profiles (a closed set: `inferred`, `pedantic`) and strict promotion
  (`specforge_project::DiagnosticPolicy`). It is the only thing that changes a diagnostic's severity
  after the diagnostic is built.
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
- **Sandbox**: what the host holds an extension to. It is granted no capability (no preopened
  directory, environment, arguments, stdin, socket or name lookup), and it is held to two limits its
  handshake's `sandbox_policy` may declare, each at most the host's ceiling (30 000 ms per call,
  512 MB of linear memory) and the ceiling when undeclared; every call also gets the whole fuel
  budget. Reading the handshake applies them; the component runtime enforces them, and a call that
  crosses one traps with the limit's kind (E028). What a declaration asks for that the host does not
  give is W153 (`specforge_wasm::sandbox`, ADR 0037).
- **In-process runtime**: the test adapter of the `WasmRuntime` port that runs an SDK-declared
  extension in the host process through the guest's own routing (`guest_call`), unsandboxed
  (it records the limits the host applies and enforces none;
  `specforge_wasm::testing::InProcessRuntime`). Host tests declare their extensions with it; the
  component runtime is the production adapter, and both keep one contract
  (`assert_runtime_contract`). MCP's tests serve every project from a temporary directory through
  it (`tests/support`): no test writes a registry, a graph or a diagnostic into a server (ADR 0025).
- **Command format**: the output an extension command is asked for, `human` (the CLI default) or
  `json` (always, over MCP). The host owns the `--format` flag; the extension renders both, since
  only it knows its payloads (ADR 0011).
- **Tool spec**: the single definition of an MCP tool, from which its descriptor, typed arguments,
  output schema, annotations, its target (reach and freshness, which declares the `path` and
  `use_cached` arguments) and its handler, a tool's (a reply)
  or a mutation's (a reply and its mutation outcome), are derived (`specforge_mcp`'s `ToolSpec`
  table).
- **Prompt spec**: the single definition of an MCP prompt, from which its descriptor, typed arguments
  and reply are derived; it renders over the call target and refuses with an McpError, sent as a
  JSON-RPC error's data since prompts have no isError (`specforge_mcp`'s `PromptSpec` table).
- **Surface call**: one MCP request that invokes a named tool, resource or prompt (`tools/call`,
  `resources/read`, `prompts/get`), run through one pipeline: read the request, find the entry (the
  core table, then, with the served project brought up to date, the extension surface table), record
  the invocation, resolve the call target, run the handler, record its events, answer with the kind's
  envelope. Each kind is one adapter; a refusal without `isError` (resources, prompts) is a JSON-RPC
  error whose data is its McpError (`specforge_mcp::surface_call`, ADR 0024).
- **Resource spec**: the single definition of a core MCP resource: its URI or template, descriptor,
  target and read. A graph view (`graph`, `context`, `brief`, scoped or not, and `graph/{entity_id}`)
  is `specforge export` over the call's project view; the entity list and the diagnostics read what
  `specforge.list` and `specforge.validate` read (`specforge_mcp`'s `ResourceSpec` table).
- **Stateless request**: an MCP request whose `_meta` names its protocol version (MCP 2026-07-28),
  answered on its own without `initialize`; every other request follows the revision `initialize`
  negotiated (`specforge_mcp::modern`).
- **Diagnostic catalog**: the one table of diagnostic codes (`specforge_diagnostics`'s `catalog!`):
  each code's title, owner, level and explanation. It generates `CATALOG`, which `specforge explain`,
  MCP `specforge.explain`, diagnostics JSON titles, doctor and the LSP hover read and from which
  `docs/diagnostics.md` is generated, and a **code constant** for every core code.
- **Code constant**: a core diagnostic code as a typed value, `specforge_diagnostics::codes::W112`
  (`Code`, at a fixed level; `GradedCode` for an `A###` finding whose pass sets the severity). Its
  level is the catalog's: a table entry whose `E`/`W`/`I`/`A` prefix contradicts its level does not
  compile. The host builds a diagnostic from one (`specforge_common::Diagnostic::new`, or
  `Diagnostic::graded` with the pass's severity), so its severity is the code's catalog level. Codes
  an extension reports cross the protocol as text and have no constant (`Diagnostic::from_extension`); they
  are checked against the catalog where they enter the host (`check_extension_code`, W150).
- **Diagnostic origin**: the extension that reported a diagnostic (`Diagnostic::origin`, JSON `origin`;
  a rule's declaring extension, a pass's extension; absent for the host's own). The catalog describes
  a diagnostic (title, explanation, docs link) only for the host's own or for the code's owner
  (`specforge_diagnostics::describes`), so an extension's finding with a code it may not use (W150) is
  kept but never presented as that code's owner's.
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
  from one module (`specforge_ops::navigate`) in source spans. A span is a position in the text the
  project was compiled from (`ProjectSession::source_text`), not in the buffer typed since nor the disk
  now. The LSP converts spans to UTF-16 ranges through that text's line index (a file the compile holds
  no text of has no range), MCP renders them as JSON (ADR 0016, ADR 0023).
- **Lexeme**: one token of a `.spec` text as the language's one lexer reads it without a parse
  (`specforge_parser::lex`): an identifier, a scheme ref ID (one lexeme), a number, a string (read as
  the grammar reads it, across lines; one the grammar would not close is marked unclosed and, if a
  `"…"`, ended at its line's end), a comment or a punctuation character. Navigation, the LSP's
  document, the parser's recovery from unclosed strings and the formatter read text through it; a test
  holds it to the grammar on the repository's spec. The prove pass's expression tokenizer is the one
  other reader (ADR 0023, ADR 0038).
- **Cursor**: what the LSP knows about a position in an open document, read from the document's lexemes
  (`specforge_parser::lex`) and their block structure, never from the graph: the word under it, whether
  it is in code, a string or a comment, the entity block, field and reference list around it, and the
  `use` statement it is on. The entity a cursor names is the declaration or reference token under it,
  else an identifier at a reference position (a header's name, a value or item of a field the
  registry does not type as a non-reference, a `use` binding's imported name) that names an entity;
  hover, definition, references and rename all ask the cursor, completion asks it what completes
  there, and semantic tokens mark the same reference positions (`specforge_lsp::document`, ADR 0023).
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
