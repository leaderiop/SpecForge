# SpecForge

**Compile human intent into a validated, typed entity graph that AI agents can actually consume.**

Intent is trapped in prose — scattered across docs, comments, tickets, and tribal knowledge. AI agents waste most of their token budget rediscovering what should have been specified. SpecForge gives them a structured, validated, machine-readable representation of what your team means.

**The graph is the product.** Not the compiler, not the DSL — the typed graph, exported as an open JSON schema (the Graph Protocol), is what makes agents reliable. SpecForge is *not* a code generator and *not* a test runner: it provides context, agents produce output, and it traces tests rather than executing them.

## Quick Example

```spec
spec "payments" {
  version "0.1.0"
}

type user "User account" {
  status draft
}

behavior authenticate_user "Authenticate a user with credentials" {
  status   draft
  category "auth"
  contract "Given valid credentials, returns an auth token"
  produces [user_logged_in]

  verify "rejects invalid password"
  verify "returns token on success"
}

event user_logged_in "User successfully logged in" {
  payload user
}
```

```bash
specforge init        # scaffold a project (interactive extension selection)
specforge check       # validate all .spec files and report diagnostics
specforge export      # emit the typed graph for an agent to consume
```

## Zero-Domain-Knowledge Core + Extensions

The compiler is a **pure typed-graph engine**. It knows how to parse `keyword name { fields }` blocks, resolve references, detect orphans and cycles, and emit a validated graph — but it carries **no domain vocabulary**. Every entity kind, edge type, and validation rule comes from an extension. If a new domain required a compiler change, the architecture would have failed.

Four extensions ship as builtins:

| Extension | Entity kinds | Purpose |
|-----------|-------------|---------|
| **`@specforge/software`** | behavior · invariant · event · type · port | The universal specification chain: contracts, guarantees, data, interfaces. |
| **`@specforge/product`** | journey · deliverable · milestone · module · term · feature · persona · channel · release | Product planning, roadmaps, and ubiquitous language. |
| **`@specforge/governance`** | decision · constraint · failure_mode | Architecture decisions, non-functional requirements, FMEA risk. |
| **`@specforge/formal`** | property · axiom · protocol · refinement · process | Formal methods: temporal properties, specification layering, event-graph linting. Enhances software entities. |

Start with one extension and a single spec file — that already improves agent output. Add more as your project grows. Anyone can author and publish their own extension; the open Graph Protocol is the moat, not the implementation.

## Why SpecForge? — The AI Agent Cost Problem

AI coding agents spend most of their token budget on exploration and disambiguation — reading files, guessing requirements, making wrong assumptions, and reworking failed attempts. SpecForge replaces that exploration with a structured query:

| Without SpecForge | With SpecForge |
|-------------------|----------------|
| Agent reads 20–50 files to reconstruct intent | Agent queries the spec graph in a few KB |
| Agent guesses requirements, asks questions | Contracts, types, and ports are explicit |
| 30–50% of tasks need rework | Rework drops sharply on first attempt |

The goal: lift AI agent first-attempt accuracy from ~30% (prose) toward 70–85% (graph). The most expensive token is the one spent discovering what should have been specified.

> See **[RES-18: AI Agent Token Economics](spec/research/RES-18-ai-agent-token-economics.md)** for the full analysis with industry data and citations.

## Agents Are First-Class Consumers

- **Multi-resolution queries** — `specforge query <id> --depth N` exposes the graph at multiple zoom levels.
- **Agent-optimized exports** — `specforge export --format=context|brief|graph|dot`.
- **Stable, open schema** — `specforge schema` emits the Graph Protocol (JSON Schema draft 2020-12).
- **MCP server** — `specforge mcp` exposes the graph and tools to agents over JSON-RPC.
- **Plan validation** — trace tests and entities end to end (Intent → Linkage → Proof).

## CLI Commands

```bash
# Project lifecycle
specforge init                       # scaffold a new project
specforge check                      # validate .spec files
specforge check --strict             # promote warnings to errors
specforge check --lint=pedantic      # include info-level diagnostics

# Graph access (for agents and humans)
specforge export --format=context    # token-efficient context for an agent
specforge query <id> --depth 2       # multi-resolution neighborhood query
specforge trace <id>                 # traceability chain for an entity
specforge schema                     # emit the Graph Protocol schema
specforge model                      # render the logical data model
specforge outline                    # render the extension architecture
specforge stats                      # project statistics

# Extensions & registry
specforge extensions                 # list installed extensions
specforge add @specforge/product     # install an extension
specforge remove <name>              # uninstall an extension
specforge search <query>             # search registries
specforge publish                    # publish an extension to a registry

# Tooling
specforge format                     # format .spec files
specforge collect                    # ingest test results (traces, never runs)
specforge doctor                     # health-check installed extensions
specforge mcp                        # start the MCP server (stdio)
specforge explain E001               # explain a diagnostic code
```

## Configuration

Projects are configured via **`specforge.json`** (like `tsconfig.json`):

```json
{
  "name": "payments",
  "version": "0.1.0",
  "spec_root": "spec",
  "extensions": ["@specforge/software"]
}
```

`specforge init` creates this for you with interactive extension selection.

## Architecture

- **Parser** — Tree-sitter grammar that parses any `keyword name { ... }` block generically, with error recovery (collects multiple diagnostics, never fails fast).
- **Graph** — petgraph-backed mutable graph, built incrementally to support watch mode and the LSP.
- **Plugin runtime** — Wasm (Extism) is the only extension runtime, with AOT caching for the CLI and warm engines for the LSP/MCP servers.
- **Surfaces** — CLI (`specforge-cli`), LSP (`specforge-lsp`), and MCP (`specforge-mcp`) all consume the same graph.

The implementation is a Rust workspace (edition 2024) under [`crates/`](crates/).

## Documentation

- **[Vision](vision/README.md)** — the manifesto, [principles](vision/principles.md), and [north star](vision/north-star.md). The source of truth for every product decision.
- **[Documentation Hub](docs/README.md)** — entity reference tables, traceability chain, validation codes.
- **[Entity Model](docs/entity-model.md)** — full architecture: core engine, extensions, edges, validation rules.
- **[Quick Reference](docs/quick-reference.md)** — single-page cheat sheet.
- **[Extension Protocol](docs/extension-protocol.md)** — how to author and publish your own extension.

## Business Plan

A comprehensive business plan lives in **[business/](business/README.md)**.

## Research Specs

| ID | Title |
|----|-------|
| [RES-11a](spec/research/RES-11a-spec-dsl-core-compiler.md) | Core compiler architecture |
| [RES-18](spec/research/RES-18-ai-agent-token-economics.md) | AI agent token economics — cost reduction analysis |
| [RES-19](spec/research/RES-19-market-position-success-estimation.md) | Market position & success estimation |
| [RES-25](spec/research/RES-25-formal-methods-integration.md) | Formal methods integration |
| [RES-26](spec/research/RES-26-zero-entity-core-architecture.md) | Zero-entity core architecture |
| [RES-27](spec/research/RES-27-software-eng-entity-redesign.md) | Software engineering entity redesign |
