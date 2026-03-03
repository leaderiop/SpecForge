# SpecForge

A compiler for `.spec` files that builds a typed entity graph with full traceability — from user-facing capabilities down to runtime invariants. Write specifications in a structured DSL, and the compiler validates cross-references, detects orphans, and generates code.

## Quick Example

```spec
invariant INV-MS-1 "Data Persistence" {
  guarantee "All committed writes survive process restarts."
  enforced_by [persist_committed_writes, replay_write_ahead_log]
  risk high
}

behavior BEH-MS-001 "Create User" {
  invariants [INV-MS-1]
  contract """
    When a valid CreateUserCommand is received,
    the system MUST create a user record with unique email
    and MUST return Result<User, DuplicateEmailError>.
  """
  verify unit "insert user, retrieve by ID, assert equal"
}

feature FEAT-MS-001 "User Management" {
  behaviors [BEH-MS-001]
  problem  "Administrators need to manage user accounts."
  solution "CRUD operations backed by PostgreSQL with unique email constraints."
}
```

## Core + Plugins

```
┌────────────────────────────────────────────────────────────┐
│                      CORE (7 entities)                     │
│  spec · invariant · behavior · feature · event · type · port│
│  + meta-schema `define` mechanism                          │
├─────────────────────────────┬──────────────────────────────┤
│   @specforge/product (5)    │   @specforge/governance (3)  │
│  capability · deliverable   │  decision · constraint       │
│  roadmap · library · glossary│  failure_mode               │
└─────────────────────────────┴──────────────────────────────┘
```

Small stable core (7 entities) covers the universal specification chain. Plugins extend it for product planning and architecture governance. Start with core, add plugins as projects grow.

## Why SpecForge? — The AI Agent Cost Problem

AI coding agents waste **60-80% of their token budget** on exploration and disambiguation — reading files, guessing requirements, making wrong assumptions, and reworking failed attempts. Only 15-25% of tokens produce actual code.

SpecForge fixes this by giving agents **exactly what they need** in a structured, validated format:

| Without SpecForge | With SpecForge | Reduction |
|-------------------|----------------|-----------|
| Agent reads 20-50 files (50k-200k tokens) | Agent queries spec graph (1k-7k tokens) | **~90%** |
| Agent guesses requirements, asks questions | Contracts + types + ports are explicit | **~95%** |
| 30-50% of tasks need rework | 5-15% need rework | **~70%** |
| **115k-410k tokens/task** | **18k-57k tokens/task** | **75-86%** |

**At project scale (50 features):** ~58M fewer tokens, ~$900 saved on Claude Opus, ~50 fewer hours of developer wait time. The developer-time savings alone exceed token costs by 10x.

The most expensive token is the one spent discovering what should have been specified.

> See **[RES-18: AI Agent Token Economics](spec/research/RES-18-ai-agent-token-economics.md)** for the full analysis with industry data and citations.

## CLI Commands

```bash
specforge init                          # scaffold a new project (core only)
specforge check                         # validate all .spec files
specforge check --strict                # errors + warnings as errors
specforge trace FEAT-MS-001             # trace a feature through the graph
specforge add @specforge/product        # install product planning plugin (+5 entities)
specforge add @specforge/governance     # install governance plugin (+3 entities)
specforge plugins                       # list installed plugins
```

## Documentation

- **[Documentation Hub](docs/README.md)** — entity reference tables, traceability chain, validation codes
- **[Entity Model](docs/entity-model.md)** — full architecture: core + plugins, edges, validation rules, design principles
- **[Quick Reference](docs/quick-reference.md)** — single-page cheat sheet for all 15 entities, 19 edges, 23 validation codes

## Business Plan

A comprehensive 10-section business plan is available in **[business/](business/README.md)**, covering executive summary, financial projections, go-to-market, technical roadmap, sales & pricing, product strategy, community, investment thesis, operations, and strategic analysis.

## Project Status

SpecForge is in the **specification phase** — the entity model and DSL grammar are defined, the compiler architecture is designed, and implementation is next.

## Research Specs

| ID | Title |
|----|-------|
| [RES-11a](spec/research/RES-11a-spec-dsl-core-compiler.md) | Core compiler architecture |
| [RES-11b](spec/research/RES-11b-spec-dsl-codegen-plugins.md) | Code generation & test plugins |
| [RES-12](spec/research/RES-12-dsl-concept-expansion.md) | DSL concept expansion analysis |
| [RES-13](spec/research/RES-13-README.md) | Market landscape 2026 |
| [RES-18](spec/research/RES-18-ai-agent-token-economics.md) | **AI agent token economics — cost reduction analysis** |
| [RES-19](spec/research/RES-19-market-position-success-estimation.md) | **Market position & success estimation** |
