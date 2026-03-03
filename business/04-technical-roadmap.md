# TECHNICAL ROADMAP

## 1. Compiler Pipeline Architecture

SpecForge compiles `.spec` files through a five-stage pipeline implemented as a Rust workspace of 10 crates. Each stage is isolated behind a clean interface, enabling independent testing, parallel development, and incremental re-compilation in watch mode.

```
.spec files (disk)
       │
       ▼
┌──────────────────┐     crate: tree-sitter-specforge
│  Stage 1: PARSE  │     crate: specforge-parser
│  tree-sitter     │     ─────────────────────────────
│  grammar → CST   │     • Tree-sitter grammar (grammar.js → C → Rust binding)
│  CST → typed AST │     • Error-recovering incremental parser
│                  │     • Per-file AST with span information
└────────┬─────────┘     • String interning via lasso (specforge-common)
         │
         ▼
┌──────────────────┐     crate: specforge-resolver
│  Stage 2: RESOLVE│     ─────────────────────────────
│  import graph    │     • `use` path resolution relative to spec root
│  symbol binding  │     • Topological file ordering (cycle detection → E003)
│  scope analysis  │     • Symbol binding: references → declarations
│                  │     • Selective import narrowing
└────────┬─────────┘     • Levenshtein suggestions for unresolved refs (strsim)
         │
         ▼
┌──────────────────┐     crate: specforge-graph
│  Stage 3: BUILD  │     ─────────────────────────────
│  entity nodes    │     • petgraph directed graph (DiGraph<Entity, Edge>)
│  typed edges     │     • 16 node types, 20 edge types
│  mutable graph   │     • Field-name-based edge routing (EdgeType::from_field_name)
│                  │     • Soft reference bookkeeping for cross-plugin refs
└────────┬─────────┘     • Incremental graph mutation for watch mode / LSP
         │
         ▼
┌──────────────────┐     crate: specforge-validator
│  Stage 4: VALIDATE│    ─────────────────────────────
│  36 diagnostics  │     • 15 error codes (E001–E015)
│  structural rules│     • 17 warning codes (W001–W017)
│  orphan detection│     • 4 info codes (I001, I003–I005)
│                  │     • Plugin-scoped rules (fire only when plugin installed)
└────────┬─────────┘     • ariadne-powered rich diagnostics with spans
         │
         ▼
┌──────────────────┐     crate: specforge-emitter (built-in outputs)
│  Stage 5: EMIT   │     crate: specforge-codegen (generator framework)
│  JSON / markdown │     ─────────────────────────────
│  agent context   │     • Built-in: JSON graph, markdown docs, coverage reports
│  code generation │     • Generators: external executables receiving graph JSON on stdin
│  test scaffolding│     • Drift detection via @specforge-checksum headers
└──────────────────┘     • Test runner report consumption (specforge-report.json)
```

### Crate Dependency DAG

```
specforge-common (lasso, serde)
       │
       ├──→ tree-sitter-specforge (tree-sitter grammar)
       │           │
       │           ▼
       ├──→ specforge-parser (tree-sitter, tree-sitter-language)
       │           │
       │           ▼
       ├──→ specforge-resolver (strsim)
       │           │
       │           ▼
       ├──→ specforge-graph (petgraph)
       │           │
       │           ▼
       ├──→ specforge-validator (ariadne)
       │           │
       │           ▼
       ├──→ specforge-emitter
       │           │
       │           ▼
       └──→ specforge-codegen (sha2, which)
                   │
          ┌────────┴────────┐
          ▼                 ▼
  specforge-cli         specforge-lsp
  (clap, walkdir,       (tower-lsp, tokio,
   notify, ctrlc,        walkdir, strsim)
   inquire)
```

### Key Design Properties

| Property | Implementation | Benefit |
|----------|---------------|---------|
| Incremental parsing | Tree-sitter edit/reparse API | <5ms re-parse on file change |
| Error recovery | Tree-sitter ERROR/MISSING nodes | Multiple errors per compilation |
| String interning | `lasso::ThreadedRodeo` | O(1) identifier comparison, reduced memory |
| Mutable graph | `petgraph::DiGraph` with stable indices | Required for watch mode and LSP |
| Plugin isolation | Module-scoped validation rules | Plugins cannot break core validation |
| Span preservation | Every AST node carries byte offsets | Precise error locations, LSP navigation |

---

## 2. Phase 1: Core Compiler (Weeks 1–24)

The foundation. By the end of Phase 1, SpecForge ships as a working compiler that parses `.spec` files, builds a validated entity graph, and produces useful output. A developer can `brew install specforge`, run `specforge check`, and get value within 60 seconds.

### Phase 1A: Parser Foundation (Weeks 1–8)

| Week | Milestone | Deliverables | Crates Affected |
|------|-----------|-------------|-----------------|
| 1–2 | Tree-sitter grammar v1 | `grammar.js` covering all 8 core entity blocks, `use` statements, reference lists, string literals, triple-quoted strings, comments. Corpus tests for every node type. | `tree-sitter-specforge` |
| 3–4 | AST construction | Typed AST structs (`SpecFile`, `EntityDecl`, `Attribute`, `FieldValue`). CST-to-AST lowering with span preservation. `lasso::ThreadedRodeo` integration for all identifiers. Snapshot tests with `insta`. | `specforge-common`, `specforge-parser` |
| 5–6 | Import resolution | `use` path resolution, file graph construction, topological sort, cycle detection (E003). Selective import narrowing. Error collection (not fail-fast). | `specforge-resolver` |
| 7–8 | Graph construction | `petgraph::DiGraph` population from resolved AST. All 9 core edge types. `EdgeType::from_field_name()` routing. Node deduplication via interned keys. | `specforge-graph` |

**Exit criteria:** Parse and graph-build the SpecForge self-hosting spec (~50 entities) with zero panics. All snapshot tests green.

### Phase 1B: Validation & Diagnostics (Weeks 9–16)

| Week | Milestone | Deliverables | Crates Affected |
|------|-----------|-------------|-----------------|
| 9–10 | Core error validation | E001 (dangling refs with Levenshtein suggestions), E002 (duplicate IDs), E003 (import cycles — already implemented, add ariadne rendering), E006 (event trigger invalid). | `specforge-validator` |
| 11–12 | Core warning validation | W001 (orphan behavior), W003 (unused invariant), W004 (unverified testable entity), W007 (orphan event), W012 (orphan ref), W013 (vague name), W014 (wrong naming convention). | `specforge-validator` |
| 13–14 | Identifier validation | E013 (reserved word), E014 (invalid characters). Unicode NFC normalization. Bidirectional character rejection. `--ascii-only` lint flag. Backtick escaping support. | `specforge-parser`, `specforge-validator` |
| 15–16 | Plugin validation framework | Plugin manifest schema. Module-scoped rule registration. `@specforge/product` rules: E007, E008, E009, W002, W008, W009, W010, W011. `@specforge/governance` rules: E005, I001, W005, W006. | `specforge-validator` |

**Exit criteria:** All 36 validation codes implemented and snapshot-tested. `specforge check` produces ariadne-rendered diagnostics with colored output, span underlining, and help text.

### Phase 1C: CLI & Output (Weeks 17–24)

| Week | Milestone | Deliverables | Crates Affected |
|------|-----------|-------------|-----------------|
| 17–18 | CLI scaffolding | `specforge check`, `specforge init`, `specforge add/remove`, `specforge plugins`, `specforge providers`. clap derive API. Exit codes: 0 (clean), 1 (errors), 2 (usage error). | `specforge-cli` |
| 19–20 | Built-in emitters | JSON graph export (`specforge compile --format json`). Agent context export (`--format agent-context`). Markdown documentation generation. Traceability matrix (`specforge trace`). | `specforge-emitter` |
| 21–22 | Watch mode | `specforge watch` using `notify` crate. Incremental re-parse on file change. Debounced validation (50ms). Terminal clearing and re-render. `ctrlc` graceful shutdown. | `specforge-cli` |
| 23–24 | Distribution & packaging | Cross-platform binaries (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64). `npm` wrapper package for `npx specforge`. Homebrew formula. `cargo install specforge-cli`. GitHub Releases with checksums. | `specforge-cli` |

**Exit criteria:** End-to-end workflow: `brew install specforge && specforge init && specforge check && specforge trace` works on macOS, Linux, Windows. Binary size <15MB (release profile with LTO + strip). Compile time <100ms for 1000-entity project.

---

## 3. Phase 2: Developer Experience (Weeks 13–36)

Phase 2 overlaps with Phase 1C. The LSP server transforms SpecForge from a batch compiler into a real-time development companion. IDE integration is the single highest-leverage investment for developer adoption.

### Phase 2A: LSP Core (Weeks 13–20)

| Week | Milestone | Deliverables | Crates Affected |
|------|-----------|-------------|-----------------|
| 13–14 | LSP server skeleton | `tower-lsp` + `tokio` runtime. `initialize`/`shutdown` lifecycle. Document sync (`textDocument/didOpen`, `didChange`, `didClose`). Background compilation on document change. | `specforge-lsp` |
| 15–16 | Diagnostics push | `textDocument/publishDiagnostics`. All 36 validation codes mapped to LSP `Diagnostic` with severity, code, source, and related information. Debounced validation (100ms after last keystroke). | `specforge-lsp` |
| 17–18 | Go-to-definition & references | `textDocument/definition` for entity references (click `data_persistence` → jump to invariant declaration). `textDocument/references` for reverse lookup (show all behaviors referencing an invariant). Cross-file navigation. | `specforge-lsp` |
| 19–20 | Hover & completion | `textDocument/hover` showing entity summary (type, title, key fields). `textDocument/completion` for reference lists (type `[` → suggest entities of the expected kind). Field name completion inside blocks. | `specforge-lsp` |

### Phase 2B: Advanced LSP (Weeks 21–28)

| Week | Milestone | Deliverables | Crates Affected |
|------|-----------|-------------|-----------------|
| 21–22 | Rename & code actions | `textDocument/rename` with cross-file propagation. Code actions: "Add missing import", "Create entity stub", "Fix naming convention". | `specforge-lsp` |
| 23–24 | Document symbols & outline | `textDocument/documentSymbol` for file outline. `workspace/symbol` for project-wide entity search. Symbol kind mapping (entity type → LSP SymbolKind). | `specforge-lsp` |
| 25–26 | Semantic tokens | Full semantic highlighting: entity keywords, identifiers (by kind), field names, string literals, references (resolved vs. unresolved), comments. Token types and modifiers registered via `textDocument/semanticTokens`. | `specforge-lsp` |
| 27–28 | Incremental sync | Switch from full document sync to incremental sync (`TextDocumentSyncKind::Incremental`). Tree-sitter `edit()` + `parse()` for sub-millisecond re-parse. Graph patching (update only changed file's contribution to the graph). | `specforge-lsp` |

### Phase 2C: IDE Extensions (Weeks 29–36)

| Week | Milestone | Deliverables | Target |
|------|-----------|-------------|--------|
| 29–30 | VS Code extension | Extension wrapping `specforge-lsp` binary. Syntax highlighting (TextMate grammar). File icon for `.spec`. Configuration: spec root path, enabled plugins, lint severity overrides. Marketplace listing. | VS Code |
| 31–32 | Neovim & Zed support | Neovim: `nvim-lspconfig` recipe + Tree-sitter queries for highlighting. Zed: language extension with tree-sitter grammar and LSP config. | Neovim, Zed |
| 33–34 | IntelliJ plugin | LSP4IJ-based plugin for IntelliJ IDEA, WebStorm, RustRover. File type registration, syntax highlighting, LSP client configuration. JetBrains Marketplace listing. | JetBrains IDEs |
| 35–36 | Error recovery hardening | Tree-sitter grammar refinements for better error recovery (missing closing braces, incomplete blocks, unterminated strings). Fuzzing with `cargo-fuzz`. Performance profiling: target <5ms incremental re-parse. | All crates |

**Exit criteria:** VS Code extension with 4.5+ star rating. Sub-100ms response time for all LSP operations on 1000-entity projects. Zero-crash guarantee under adversarial input (fuzz-tested).

---

## 4. Phase 3: Ecosystem (Weeks 24–48)

Phase 3 builds the extension platform that enables SpecForge to grow beyond what one team can ship. The Terraform provider model is the architectural north star: small stable core, rich ecosystem of independently maintained extensions.

### Phase 3A: Plugin SDK (Weeks 24–32)

| Week | Milestone | Deliverables |
|------|-----------|-------------|
| 24–26 | Plugin manifest format | TOML-based manifest specifying: entity types (name, fields, required/optional), edge types (from, to, field mapping), validation rules (code, severity, message template), testability flag. Schema versioned for forward compatibility. |
| 27–28 | Plugin loader | Runtime plugin discovery from `~/.specforge/plugins/` and project-local `.specforge/plugins/`. Manifest parsing and validation. Entity type registration into the compiler's type registry. Edge type registration into `EdgeType::from_field_name()`. |
| 29–30 | Plugin validation integration | Plugin-defined validation rules executed after core validation. Scoped diagnostic codes (plugin namespace prefix). Cross-plugin soft reference resolution (I004). Plugin removal graceful degradation (soft refs revert to I004). |
| 31–32 | Plugin CLI & registry | `specforge add <plugin>`, `specforge remove <plugin>`, `specforge plugins list`. Plugin resolution from npm registry (`@specforge/` namespace) and git URLs. Lock file (`specforge-lock.json`) for reproducible builds. |

### Phase 3B: Provider System (Weeks 28–38)

| Week | Milestone | Deliverables |
|------|-----------|-------------|
| 28–30 | Provider interface | Provider trait: `register_schemes()`, `validate_target()`, `resolve_url()`. Multi-instance support with aliases. Configuration schema (provider-specific fields in `spec` root `providers` block). |
| 31–33 | `@specforge/gh` provider | GitHub provider: schemes `gh.issue`, `gh.pr`, `gh.discussion`, `gh.release`. Target validation via regex patterns. URL resolution via repo config. Optional API validation (rate-limited, cached). |
| 34–36 | `@specforge/jira` and `@specforge/figma` providers | Jira: `jira.epic`, `jira.story`, `jira.task`. Project key validation. Figma: `figma.frame`, `figma.component`. URL pattern matching. Both support multi-instance aliases. |
| 37–38 | Provider SDK documentation | Public trait documentation. Provider template repository. Integration test harness. Contribution guide. Provider submission process for community providers. |

### Phase 3C: Generator Framework (Weeks 34–44)

| Week | Milestone | Deliverables |
|------|-----------|-------------|
| 34–36 | Generator protocol | Generators are standalone executables. Protocol: receive graph JSON on stdin, emit file manifest on stdout. Configuration via `spec` root `gen` block. Generator discovery from PATH and `~/.specforge/generators/`. |
| 37–39 | `@specforge/gen-typescript` | TypeScript generator: interfaces from `type` blocks, abstract classes from `port` blocks, test scaffolding from `verify`/`scenario` declarations. Drift detection with `@specforge-checksum:sha256:...` headers. `specforge gen typescript --check` for CI. |
| 40–42 | `@specforge/gen-rust` | Rust generator: structs from `type` blocks, traits from `port` blocks. `specforge-test` runtime crate + `specforge-test-macros` proc macro crate. Convention-based test mapping (`mod entity_id`, `entity_id__description_slug`). JUnit XML → `specforge-report.json` via `specforge collect rust`. |
| 43–44 | `@specforge/gen-python` and `@specforge/gen-go` | Python: dataclasses from types, ABCs from ports, pytest fixtures from verify. Go: structs from types, interfaces from ports, table-driven tests from verify. Both with drift detection. |

### Phase 3D: Test Traceability (Weeks 40–48)

| Week | Milestone | Deliverables |
|------|-----------|-------------|
| 40–42 | `specforge-report.json` schema | Standardized test report format consumed by the compiler. Fields: entity_id, test_file, test_name, status (pass/fail/skip), duration, error_message. Schema versioned. |
| 43–44 | Test runner adapters | `@specforge/vitest`, `@specforge/pytest`, `@specforge/playwright`, `@specforge/go`, `@specforge/k6`. Each reads runner-native output (JUnit XML, JSON) and emits `specforge-report.json`. |
| 45–46 | Coverage reporting | `specforge coverage` command. Four coverage levels: declared (has verify/scenario) → linked (has tests field) → executed (in report) → passing. Per-entity and aggregate metrics. JSON and terminal output. |
| 47–48 | CI integration | `specforge ci` composite command (check + coverage + trace). GitHub Actions action (`specforge/action`). Exit code 1 if coverage below threshold. PR comment bot with coverage diff. |

**Exit criteria:** At least 3 official generators shipping. Plugin SDK documented and used by at least 2 community plugins. Test traceability chain working end-to-end for at least TypeScript and Rust.

---

## 5. Phase 4: Cloud & Enterprise (Weeks 36–96)

Phase 4 transitions SpecForge from a single-machine CLI tool to a collaborative platform. This phase introduces the cloud components that enable team-wide spec management, cross-repository traceability, and enterprise compliance workflows.

### Phase 4A: Cloud Platform Foundation (Weeks 36–52)

| Week | Milestone | Deliverables |
|------|-----------|-------------|
| 36–40 | API server | Rust web service (axum) hosting the spec graph as a queryable API. GraphQL endpoint for entity queries, edge traversal, and cross-repo reference resolution. Authentication via GitHub/GitLab OAuth. PostgreSQL for graph persistence. |
| 41–44 | Spec sync | `specforge push` and `specforge pull` commands. Conflict detection (concurrent edits to same entity). Server-side compilation and validation. Webhook integration for automatic sync on git push. |
| 45–48 | Web dashboard | Read-only web viewer for spec graphs. Entity explorer with click-to-navigate. Traceability visualization (Mermaid-rendered DAGs). Coverage dashboard with drill-down. Search across all entities. |
| 49–52 | Team management | Organization and project hierarchy. Role-based access control (owner, admin, member, viewer). Audit log for spec changes. SSO via SAML 2.0 / OIDC for enterprise customers. |

### Phase 4B: Collaboration Features (Weeks 48–68)

| Week | Milestone | Deliverables |
|------|-----------|-------------|
| 48–52 | Change proposals | PR-like workflow for spec changes. Diff view showing entity additions, modifications, deletions. Edge change impact analysis ("changing this invariant affects 12 behaviors"). Review and approval workflow. |
| 53–58 | Cross-repository traceability | Multi-repo spec graphs linked via `ref` entities. Organization-wide entity search. Dependency tracking across services ("which services depend on this shared invariant?"). Cross-repo orphan detection. |
| 59–64 | Notifications & workflows | Slack/Teams/email notifications for spec changes affecting your entities. Configurable automation: "when a behavior's tests fail, notify the feature owner." Custom dashboards per team. |
| 65–68 | AI agent API | REST/GraphQL API optimized for AI agent consumption. Token-efficient graph serialization. Context window budget parameter ("give me the most relevant 4000 tokens for implementing this behavior"). Agent session tracking for analytics. |

### Phase 4C: Enterprise & Compliance (Weeks 64–96)

| Week | Milestone | Deliverables |
|------|-----------|-------------|
| 64–72 | Compliance templates | Pre-built spec templates for ISO 27001, SOC 2 Type II, FDA 21 CFR Part 11, GDPR Article 30. Mapping from SpecForge entities to compliance evidence requirements. Automated evidence collection from spec graph + test reports. |
| 73–80 | Analytics & reporting | Historical trend analysis (spec coverage over time, orphan count trends, validation error rates). Executive summary reports (PDF/HTML). Custom metric definitions via spec-graph queries. |
| 81–88 | Advanced governance | Spec change approval gates (require N approvals before merge). Mandatory review for high-risk invariant changes. Breaking change detection and notification. Spec version tagging with release notes. |
| 89–96 | On-premises deployment | Helm chart for Kubernetes deployment. Air-gapped installation support. Data residency controls. Custom identity provider integration. Enterprise support SLA (99.9% uptime, 4-hour response). |

**Exit criteria:** Cloud platform serving 50+ organizations. SOC 2 Type II compliant infrastructure. Cross-repo traceability working across 10+ repository graphs. AI agent API handling 1000+ requests/day.

---

## 6. Performance Targets

### Compiler Performance

| Metric | Target | Measurement | Rationale |
|--------|--------|-------------|-----------|
| Parse time (1000 entities) | <30ms | `Instant::elapsed()` around parser | Tree-sitter is ~10x faster than hand-written; 30ms leaves headroom |
| Resolve time (1000 entities) | <15ms | Topological sort + symbol binding | Linear in file count; dominated by HashMap lookups on interned keys |
| Graph build time (1000 entities) | <10ms | petgraph node/edge insertion | petgraph `add_node`/`add_edge` is O(1) amortized |
| Validation time (1000 entities) | <20ms | All 36 rules over graph | Graph traversal is O(V+E); ariadne rendering deferred |
| Emit time (JSON, 1000 entities) | <15ms | serde_json serialization | Linear in graph size |
| **Total compile time (1000 entities)** | **<100ms** | End-to-end wall clock | Sub-human-perception latency for batch mode |
| Incremental re-compile (1 file change) | <10ms | Tree-sitter edit + graph patch | Critical for watch mode and LSP responsiveness |

### Memory Usage

| Metric | Target | Notes |
|--------|--------|-------|
| Base memory (empty project) | <5MB | Runtime + tree-sitter + petgraph allocations |
| Per-entity overhead | ~30–50KB | AST node + graph node + interned strings + spans |
| 1000-entity project | <50MB | 1000 x 45KB average + graph edges + index structures |
| 5000-entity project | <250MB | Linear scaling; no superlinear algorithms in pipeline |
| String intern table | <2MB for 10K unique identifiers | lasso `ThreadedRodeo` with 64-byte average string |

### Binary Size

| Binary | Target | Achieved Via |
|--------|--------|-------------|
| `specforge` (CLI) | <15MB | `lto = true`, `strip = true` in release profile |
| `specforge-lsp` | <18MB | Includes tokio runtime + tower-lsp |
| npm wrapper package | <500KB | Thin wrapper that downloads platform binary |

### LSP Performance

| Operation | Target Latency | Notes |
|-----------|---------------|-------|
| `textDocument/completion` | <50ms | Scoped to expected entity kind; pre-indexed |
| `textDocument/definition` | <10ms | Direct graph lookup via interned key |
| `textDocument/references` | <30ms | Reverse edge traversal in petgraph |
| `textDocument/hover` | <20ms | Pre-computed entity summaries |
| `textDocument/rename` | <200ms | Cross-file; requires graph mutation + re-validation |
| `textDocument/semanticTokens` | <50ms | Single-file tree-sitter traversal |
| Diagnostics after edit | <150ms | Incremental re-parse + targeted re-validation |

### Benchmark Suite

Automated benchmarks run on every PR using `criterion`:

```
benches/
  parse_1000_entities.rs      # Parser throughput
  resolve_1000_entities.rs    # Resolver throughput
  graph_build_1000_entities.rs # Graph construction
  validate_1000_entities.rs   # All validation rules
  compile_e2e_1000_entities.rs # Full pipeline
  incremental_single_file.rs  # Watch mode simulation
```

Regression threshold: 10% slowdown on any benchmark blocks merge.

---

## 7. Build vs. Buy Decisions

| # | Decision | Choice | Alternatives Considered | Rationale |
|---|----------|--------|------------------------|-----------|
| 1 | **Parser technology** | Tree-sitter (build grammar) | Hand-written recursive descent; LALR (pest/nom); Language Workbench (Spoofax) | Tree-sitter provides incremental parsing, error recovery, and editor integration out of the box. Hand-written parsers require reimplementing all three. LSP token highlighting comes free. |
| 2 | **Implementation language** | Rust | TypeScript; Go; Zig | Single binary distribution (no runtime). Memory safety without GC (critical for LSP long-running process). Cargo workspace maps cleanly to compiler pipeline stages. Ecosystem has tree-sitter, petgraph, tower-lsp, ariadne. |
| 3 | **Graph library** | petgraph | Custom adjacency list; Neo4j embedded; SQLite as graph | petgraph is the Rust standard. Stable node indices enable mutable graphs. Topological sort, cycle detection, and traversal algorithms are battle-tested. No external process or IPC overhead. |
| 4 | **Error diagnostics** | ariadne | codespan-reporting; miette; custom renderer | ariadne produces the highest-quality output (Rust-compiler-grade). Supports multi-file spans, inline labels, and color. Active maintenance. Smaller dependency tree than miette. |
| 5 | **LSP framework** | tower-lsp | lsp-server (rust-analyzer); custom JSON-RPC | tower-lsp integrates with tokio and tower ecosystem. Handles protocol details (initialization, shutdown, request routing). rust-analyzer's lsp-server is more low-level and tightly coupled to rust-analyzer's architecture. |
| 6 | **String interning** | lasso | string-interner; internment; custom FxHashMap | lasso's `ThreadedRodeo` is the fastest thread-safe interner in Rust benchmarks. Multi-threaded feature required for LSP server. Zero-cost key comparison (u32 equality). |
| 7 | **CLI framework** | clap (derive) | structopt (deprecated); argh; pico-args | clap derive API gives declarative subcommand definitions with automatic help generation. Industry standard for Rust CLIs. Shell completion generation built-in. |
| 8 | **Serialization** | serde + serde_json | manual serialization; simd-json; rkyv | serde is the Rust ecosystem standard. Derive macros eliminate boilerplate. serde_json handles the generator protocol (stdin/stdout JSON). No need for binary formats in the core pipeline. |
| 9 | **Snapshot testing** | insta | expect-test; goldentests; custom diffing | insta provides `cargo insta review` TUI for approving changes. JSON snapshot support for AST and graph testing. Inline snapshots for small outputs. Widely adopted in Rust ecosystem. |
| 10 | **File watching** | notify | inotify/kqueue direct; polling; watchman | notify abstracts platform differences (inotify on Linux, FSEvents on macOS, ReadDirectoryChanges on Windows). Debouncing support. Maintained and well-tested. |
| 11 | **Fuzzy matching** | strsim | fuzzy-matcher; sublime_fuzzy; ngrammatic | strsim provides Levenshtein, Jaro-Winkler, and other edit distance algorithms. Minimal dependency (no allocation-heavy crates). Sufficient for "did you mean?" suggestions. |
| 12 | **Async runtime** | tokio | async-std; smol; no async (threads) | tower-lsp requires tokio. tokio is the de facto standard Rust async runtime. Required only for LSP binary; CLI binary is fully synchronous. |
| 13 | **Database (cloud platform)** | PostgreSQL | SQLite; DuckDB; CockroachDB; graph DB (Neo4j) | Relational model maps well to typed entity graphs (entities table + edges table). JSONB for flexible entity attributes. Proven scale characteristics. Team SQL expertise assumed. Graph queries use recursive CTEs. |
| 14 | **Web framework (cloud API)** | axum | actix-web; warp; rocket | axum integrates with tokio/tower ecosystem (same as tower-lsp). Type-safe extractors. Built by tokio team. Growing adoption surpassing actix-web. Minimal macro magic. |

---

## 8. CI/CD Pipeline

### GitHub Actions Workflow Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Push / Pull Request                       │
└────────────────────────┬────────────────────────────────────┘
                         │
            ┌────────────┼────────────────┐
            ▼            ▼                ▼
      ┌──────────┐ ┌──────────┐   ┌──────────────┐
      │  Lint &   │ │  Test    │   │  Build       │
      │  Format   │ │  Matrix  │   │  Matrix      │
      │           │ │          │   │              │
      │ clippy    │ │ unit     │   │ linux-x86_64 │
      │ rustfmt   │ │ snapshot │   │ linux-aarch64│
      │ taplo     │ │ integration│ │ macos-x86_64 │
      │ typos     │ │ fuzz     │   │ macos-aarch64│
      └──────┬────┘ └────┬─────┘   │ windows-x86  │
             │            │         └──────┬───────┘
             ▼            ▼                ▼
      ┌───────────────────────────────────────────┐
      │              All Checks Pass               │
      └──────────────────────┬────────────────────┘
                             │
                    (on tag push: v*)
                             │
                             ▼
                  ┌─────────────────────┐
                  │   Release Pipeline  │
                  │                     │
                  │ • Build all targets │
                  │ • SHA-256 checksums │
                  │ • GitHub Release    │
                  │ • npm publish       │
                  │ • Homebrew update   │
                  │ • crates.io publish │
                  └─────────────────────┘
```

### Build Matrix

| Target | Runner | Toolchain | Notes |
|--------|--------|-----------|-------|
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | stable | Primary Linux target |
| `aarch64-unknown-linux-gnu` | `ubuntu-latest` + cross | stable | ARM Linux (Graviton, Raspberry Pi) |
| `x86_64-apple-darwin` | `macos-13` | stable | Intel Mac |
| `aarch64-apple-darwin` | `macos-14` | stable | Apple Silicon (M1+) |
| `x86_64-pc-windows-msvc` | `windows-latest` | stable | Windows |

### CI Jobs Detail

| Job | Trigger | Duration Target | Tools |
|-----|---------|----------------|-------|
| `clippy` | Every push | <2 min | `cargo clippy --workspace --all-targets -- -D warnings` |
| `rustfmt` | Every push | <30 sec | `cargo fmt --all -- --check` |
| `test-unit` | Every push | <5 min | `cargo test --workspace` |
| `test-snapshot` | Every push | <3 min | `cargo insta test --workspace` |
| `test-integration` | Every push | <5 min | `cargo test --workspace --test '*' -- --include-ignored` |
| `fuzz` | Nightly + weekly | 10 min | `cargo fuzz run parse_spec -- -max_total_time=600` |
| `benchmark` | PR only | <5 min | `criterion` benchmarks with regression detection |
| `build-release` | Tag push | <15 min/target | `cargo build --release --target <triple>` |
| `audit` | Weekly | <2 min | `cargo audit` + `cargo deny check` |
| `msrv` | Weekly | <5 min | Verify minimum supported Rust version |

### Release Automation

```yaml
# Triggered by: git tag v0.1.0 && git push --tags
release:
  steps:
    1. Build all 5 platform targets (parallel)
    2. Generate SHA-256 checksums for each binary
    3. Create GitHub Release with:
       - Release notes (auto-generated from conventional commits)
       - Binary artifacts per platform
       - Checksum file (SHA256SUMS)
    4. Publish to crates.io:
       - specforge-common
       - specforge-parser (depends on common)
       - specforge-graph (depends on parser)
       - ... (topological order)
       - specforge-cli (last)
    5. Publish npm wrapper: @specforge/cli
       - Detects platform, downloads correct binary
       - npx specforge works immediately
    6. Update Homebrew formula:
       - PR to homebrew-specforge tap
       - Updated SHA-256 and download URL
    7. Update VS Code extension marketplace (if extension changed)
```

### Quality Gates

| Gate | Condition | Blocks |
|------|-----------|--------|
| All tests pass | 0 failures across all test jobs | Merge to main |
| No clippy warnings | `-D warnings` flag on clippy | Merge to main |
| Benchmarks within threshold | <10% regression on any benchmark | Merge to main (PR only) |
| Snapshot review | All snapshot changes approved via `insta` | Merge to main |
| Binary size check | CLI <15MB, LSP <18MB | Release |
| Audit clean | No known vulnerabilities in dependencies | Release |

---

## 9. Infrastructure Cost Projections

### Year 1: Open-Source CLI Only ($1K–$3K)

| Resource | Provider | Monthly Cost | Annual Cost | Notes |
|----------|----------|-------------|-------------|-------|
| GitHub Actions CI | GitHub | $0 | $0 | Free for public repos (3,000 min/month) |
| GitHub Actions (overflow) | GitHub | $8 | $96 | Occasional large runner usage for ARM cross-compilation |
| Domain (specforge.dev) | Cloudflare | $1 | $12 | Domain registration + DNS |
| Documentation site | Cloudflare Pages | $0 | $0 | Free tier (500 builds/month) |
| npm registry | npmjs.com | $0 | $0 | Free for public packages |
| Homebrew tap | GitHub | $0 | $0 | Git repository |
| crates.io | crates.io | $0 | $0 | Free for all Rust crates |
| **Total** | | **$9/mo** | **$108** | |

Contingency buffer: $1,000. **Year 1 total: ~$1,100.**

### Year 2: Platform Foundation ($18K–$30K)

| Resource | Provider | Monthly Cost | Annual Cost | Notes |
|----------|----------|-------------|-------------|-------|
| API server (2x) | Fly.io / Railway | $50 | $600 | 2 instances, 1 vCPU, 512MB each |
| PostgreSQL | Neon / Supabase | $25 | $300 | Serverless Postgres, free tier + small paid |
| Redis (caching) | Upstash | $10 | $120 | Session cache, rate limiting |
| Object storage | Cloudflare R2 | $5 | $60 | Compiled graph snapshots |
| GitHub Actions CI | GitHub | $50 | $600 | Team plan for larger runners |
| Monitoring | Grafana Cloud | $0 | $0 | Free tier (10K series, 50GB logs) |
| Error tracking | Sentry | $26 | $312 | Team plan |
| CDN + WAF | Cloudflare | $20 | $240 | Pro plan |
| Email (transactional) | Resend | $20 | $240 | Notifications, onboarding |
| Documentation site | Cloudflare Pages | $0 | $0 | Still free tier |
| **Total** | | **$206/mo** | **$2,472** | |

Growth buffer (3x for scaling): $7,400. **Year 2 total: ~$10,000.**

### Year 3: Scale Platform ($60K–$92K)

| Resource | Provider | Monthly Cost | Annual Cost | Notes |
|----------|----------|-------------|-------------|-------|
| API servers (4x) | AWS ECS Fargate | $400 | $4,800 | 4 tasks, 2 vCPU, 4GB each |
| PostgreSQL | AWS RDS | $200 | $2,400 | db.r6g.large, Multi-AZ |
| Redis | AWS ElastiCache | $100 | $1,200 | cache.t4g.medium |
| Object storage | AWS S3 | $30 | $360 | Graph snapshots + report artifacts |
| CDN | CloudFront | $50 | $600 | Web dashboard + API caching |
| CI/CD | GitHub Actions | $200 | $2,400 | Large runners for build matrix |
| Monitoring | Datadog | $150 | $1,800 | APM + Infrastructure + Logs |
| Error tracking | Sentry | $80 | $960 | Business plan |
| Search | Typesense Cloud | $50 | $600 | Entity search across orgs |
| Auth | Auth0 | $100 | $1,200 | Enterprise SSO (SAML/OIDC) |
| Email | Resend / SES | $30 | $360 | Higher volume |
| Compliance | Vanta | $400 | $4,800 | SOC 2 automation |
| DNS + WAF | Cloudflare | $200 | $2,400 | Business plan |
| Backups | AWS Backup | $50 | $600 | Daily snapshots, 30-day retention |
| **Total** | | **$2,040/mo** | **$24,480** | |

Growth buffer (3x for enterprise traffic): $67,500. **Year 3 total: ~$92,000.**

### Cost Summary

| | Year 1 | Year 2 | Year 3 |
|--|--------|--------|--------|
| Monthly run rate | $9 | $206 | $2,040 |
| Annual base | $108 | $2,472 | $24,480 |
| Growth buffer | $1,000 | $7,500 | $67,500 |
| **Total budget** | **$1,100** | **$10,000** | **$92,000** |
| Revenue (base scenario) | $25,000 | $295,000 | $1,040,000 |
| Infrastructure as % of revenue | 4.4% | 3.4% | 8.8% |

---

## 10. Technical Risks & Mitigations

### Risk Register

| # | Risk | Probability | Impact | Severity | Mitigation | Contingency |
|---|------|-------------|--------|----------|------------|-------------|
| R1 | **Tree-sitter grammar complexity exceeds maintainability** — the 16-entity grammar with nested blocks, triple-quoted strings, type expressions, and Unicode identifiers may produce a grammar.js that is difficult to debug and extend. | Medium | High | **High** | Start with core 8 entities only. Comprehensive corpus test suite (one .txt per grammar rule). Grammar CI that tests parse/no-parse on 500+ examples. Avoid tree-sitter external scanner unless absolutely necessary. | Fall back to tree-sitter for tokenization only; hand-write block-level parser on top of tree-sitter token stream. |
| R2 | **petgraph stable indices limit graph mutation performance** — incremental graph updates (add/remove entities on file change) may cause index fragmentation or require expensive compaction in large graphs. | Low | Medium | **Low** | Benchmark graph mutation at 5000 nodes. Profile with `criterion` under watch-mode simulation (rapid add/remove cycles). Use `StableGraph` variant if index stability matters more than compaction. | Switch to `slotmap`-based custom graph with generation-counted handles. Petgraph remains for algorithms (toposort, cycle detection) on read-only snapshots. |
| R3 | **LSP responsiveness degrades on large projects** — projects with 5000+ entities may exceed the 150ms diagnostic latency target due to full re-validation on every keystroke. | Medium | High | **High** | Incremental validation: track which entities were affected by the file change, re-validate only those. Debounce edits (100ms). Cache unchanged file ASTs. Tree-sitter incremental parsing reduces re-parse to <5ms. | Tiered validation: fast pass (syntax errors only, <10ms) on every keystroke; full validation on save. User-configurable via LSP settings. |
| R4 | **Cross-platform binary distribution fails on edge cases** — `musl` vs `glibc` linking issues on Linux, Rosetta compatibility on macOS, antivirus false positives on Windows, npm postinstall script failures in corporate environments. | High | Medium | **High** | Test all 5 targets in CI on every release. Static linking on Linux (musl). Code-sign macOS and Windows binaries. npm wrapper uses platform detection with fallback to `cargo install`. Provide manual download as escape hatch. | Ship Docker image as universal fallback (`docker run specforge check`). Maintain detailed troubleshooting guide. |
| R5 | **Plugin ABI stability becomes a maintenance burden** — community plugins compiled against one version of the plugin manifest may break when the manifest schema evolves. | Medium | High | **High** | Version the plugin manifest schema (semver). Plugin loader validates manifest version and provides clear upgrade messages. Maintain backward compatibility for at least 2 major versions. Plugins are TOML manifests (not compiled code), reducing ABI concerns. | If compiled plugins become necessary, use Wasm (wasmtime) for sandboxed, ABI-stable plugin execution. |
| R6 | **Generator protocol is too restrictive** — stdin/stdout JSON may not support generators that need filesystem access, network calls, or interactive prompts. | Low | Medium | **Low** | JSON protocol covers 90% of use cases (pure transformations). For advanced generators, provide an optional gRPC sidecar protocol. Document escape hatches: generators can shell out to `specforge query` for additional graph data. | Evolve to a plugin-host model (like VS Code extensions) where generators run as child processes with IPC channels. |
| R7 | **Token-efficient agent context format is not token-efficient enough** — the `--format agent-context` output may still consume too many tokens for large projects, negating SpecForge's value proposition. | Medium | Critical | **Critical** | Design agent-context format as a compressed, hierarchical summary (not a dump). Implement `--budget` flag that limits output to N tokens. Use graph centrality algorithms to rank entities by importance. Provide `--focus` flag to scope output to specific entities and their transitive dependencies. | Implement MCP (Model Context Protocol) server in specforge-lsp that serves graph data on demand, eliminating the need to pre-load all context. |
| R8 | **Rust compile times slow down development velocity** — the 10-crate workspace may have long incremental build times, especially when modifying `specforge-common` (depended on by all crates). | Medium | Medium | **Medium** | Minimize `specforge-common` surface area (interning + core types only). Use `cargo check` for development. Enable `sccache` or `mold` linker in CI and dev machines. Profile compile times with `cargo timings`. Avoid proc macros in hot-path crates. | Split `specforge-common` into `specforge-types` (pure data, fast compile) and `specforge-interner` (lasso wrapper). Investigate dynamic linking for dev builds. |
| R9 | **Cloud platform security incident** — a vulnerability in the API server exposes customer spec graphs, which may contain proprietary business logic and architecture details. | Low | Critical | **High** | Security-first design: all data encrypted at rest (AES-256) and in transit (TLS 1.3). SOC 2 compliance from day one of cloud launch. Quarterly penetration testing. Dependency auditing via `cargo audit` + Dependabot. No PII in spec graphs by design. Minimal data retention. | Incident response plan with 24-hour customer notification. Ability to immediately rotate all API keys. Forensic logging sufficient for post-incident analysis. Cyber insurance from Year 2. |
| R10 | **Competitor ships first** — a well-funded competitor (Cursor, GitHub, JetBrains) ships a similar specification compiler before SpecForge reaches critical mass, leveraging their existing distribution advantage. | Medium | Critical | **Critical** | Move fast on Phase 1 (24 weeks to usable compiler). Open-source core to build community moat. Focus on differentiation: the typed entity graph with 36 validation codes is non-trivial to replicate. Build integrations with all major AI agents (Claude, GPT, Gemini) rather than betting on one. Plugin ecosystem creates switching costs. | Pivot to "SpecForge as library" — license the compiler core to the competitor as an embedded component. Partner rather than compete. The spec graph data model and validation rules are the durable IP, not the CLI chrome. |

### Risk Heatmap Summary

```
                   Low Impact    Medium Impact    High Impact    Critical Impact
High Prob    │                │      R4          │               │
Medium Prob  │                │      R8          │  R1, R3, R5   │  R7, R10
Low Prob     │                │      R2, R6      │  R9           │
```

### Risk Review Cadence

| Frequency | Action |
|-----------|--------|
| Weekly | Review R1 (grammar complexity) and R8 (compile times) during development |
| Monthly | Review all High/Critical severity risks; update probability based on progress |
| Quarterly | Full risk register review; add/retire risks; update mitigations |
| Per release | Review R4 (distribution) and R5 (plugin ABI) before every version bump |
