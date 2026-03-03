# SPECFORGE PRODUCT STRATEGY

## 1. Product Vision (3-Year Horizon)

**Vision Statement:** By 2029, SpecForge is the standard interface between human intent and AI-generated software — the specification layer that makes AI agents build the right thing, every time.

**Year 1 (2026): The Compiler.** SpecForge ships as a developer tool that replaces unstructured context files with a validated, graph-aware specification language. Developers adopt it because their AI agents produce dramatically better code on the first pass. The CLI is rock-solid, the LSP makes authoring frictionless, and the token reduction (75-86%) is measurable and real. We win the AI-first developer persona by being obviously better than CLAUDE.md within 15 minutes of first use.

**Year 2 (2027): The Platform.** SpecForge becomes the connective tissue across the software development lifecycle. Generators emit test scaffolding, architecture diagrams, and API contracts from the same spec graph. Providers bridge SpecForge to GitHub, Jira, Figma, and CI systems. Engineering managers open a dashboard and see which features are specified, implemented, tested, and passing — derived entirely from the spec graph and test reports. We win the tech lead and engineering manager personas by being the single source of truth they never had.

**Year 3 (2028): The Standard.** SpecForge specs are checked into every serious repository the way package.json or Cargo.toml are today. AI agents read .spec files natively. Cloud-hosted spec graphs enable cross-repository traceability for organizations. Compliance teams use SpecForge to satisfy FDA, ISO 27001, and SOC 2 audit requirements without separate tooling. The plugin ecosystem is self-sustaining, with community-contributed generators for every major framework and language. We win the compliance persona by being cheaper, faster, and more developer-friendly than DOORS or Jama.

**The North Star Metric:** Percentage of AI-generated code that requires zero human correction when the agent has access to a SpecForge spec graph. We believe this number moves from roughly 30% (plain-text context) to over 70% (structured spec graph) in Year 1, and to over 85% with test traceability feedback loops in Year 2.

---

## 2. Product Principles

These seven principles govern every feature decision, prioritization trade-off, and design review.

### P1: Developer-First, Always
SpecForge is a developer tool that happens to satisfy enterprise needs — never the reverse. Every feature must pass the test: "Would a solo developer with no manager choose to use this?" If the answer is no, the feature is either redesigned or deprioritized. Enterprise value emerges from developer adoption, not the other way around.

### P2: Seconds to Value
A developer should go from `brew install specforge` to seeing validated output in under 60 seconds. First-run experience is sacred. No accounts, no configuration files, no YAML ceremony. The CLI must work with zero configuration on any project. Every additional feature is opt-in complexity, never mandatory overhead.

### P3: The Spec Is the Source of Truth
The .spec file is not documentation that drifts from code. It is a compiled artifact with validation, graph semantics, and traceability. If something is true in the spec but false in the code, that is a bug in the code. This principle means we invest heavily in validation (36 codes and growing), drift detection, and CI integration — because the spec's authority depends on its accuracy.

### P4: AI Agents Are First-Class Consumers
Every output format, every graph query, every error message is designed to be consumed by AI agents as much as by humans. This means structured output (JSON, not just pretty-printed text), deterministic ordering, stable schemas, and token-efficient representations. We measure output quality not just by human readability but by agent task-completion rates.

### P5: Small Core, Rich Ecosystem
The compiler core stays small (8 entity types, 9 edge types) and stable. Everything else — additional entities, reference providers, output generators — lives in the plugin/provider/generator extension model. We would rather ship a clean extension API and let the community build 50 generators than ship 50 generators ourselves. The Terraform provider model is our north star for extension architecture.

### P6: Validate Early, Validate Everything
The compiler catches problems before code is written. Dangling references, missing test coverage, orphaned entities, ambiguous naming — these are compile-time errors, not runtime surprises. Every entity relationship is an edge in a validated graph. The cost of finding a specification error should be near zero (seconds in the editor), not thousands of dollars (in production).

### P7: Traceability Is a Product, Not a Report
Traceability (spec to code to test to result) is not a checkbox feature for compliance teams. It is the core feedback loop that makes AI agents self-correcting. When an agent can see that a behavior is specified, partially implemented, and has one failing test, it can fix the failing test. When it cannot see that, it rewrites everything from scratch. Traceability is what makes SpecForge compound in value over time.

---

## 3. User Journey Maps (AI-First Developer — Primary Persona)

### Journey 1: First Encounter (Day 0)

```
Trigger: Developer sees a tweet/blog post showing SpecForge reducing AI agent
         hallucinations by 75%. They are frustrated with their 2000-line CLAUDE.md
         that agents routinely ignore or misinterpret.

Step 1: Install
  $ brew install specforge
  (also: npx specforge, cargo install specforge-cli)
  Time: 10 seconds. No account. No config.

Step 2: Initialize
  $ specforge init
  Creates: spec/project.spec with spec block + 2 example entities
  Time: 5 seconds.

Step 3: First compile
  $ specforge check
  Output: "3 entities, 2 edges, 0 errors, 1 warning (W004: behavior
          'user_login' has no test coverage declared)"
  Moment of truth: The developer sees that SpecForge actually understands
  their specification structure. It is not a linter — it is a compiler.
  Time: 2 seconds.

Step 4: First AI integration
  Developer adds `specforge compile --format agent-context` output to their
  AI agent's context window. The agent builds a feature correctly on the
  first attempt that previously took 3 rounds of correction.
  Time: 5 minutes.
  Emotional state: "This is real."

Step 5: Commit
  Developer commits spec/ directory alongside their code.
  SpecForge is now part of the project.
```

### Journey 2: Daily Workflow (Week 2+)

```
Morning: Developer opens IDE. LSP is active. They write a new behavior block.
  - Autocomplete suggests entity references from the graph
  - Inline diagnostics show "W004: no test coverage" as they type
  - Go-to-definition jumps from a reference to the referenced entity

Coding session: Developer tells their AI agent to implement a feature.
  - Agent reads compiled spec context (200 tokens instead of 1400)
  - Agent generates code + test stubs matching verify/scenario declarations
  - Agent fills in the `tests` field linking to generated test files

End of day: Developer runs CI pipeline.
  - `specforge check` validates the spec graph (zero errors required)
  - `specforge trace` verifies test linkage (all testable entities covered)
  - `specforge report` shows 94% spec coverage
  - Failed: 1 behavior has a verify declaration but no linked test file (W018)
```

### Journey 3: Onboarding a Teammate (Month 2)

```
New developer joins the project. Instead of reading 40 pages of architecture
docs, they run:

  $ specforge compile --format overview
  Shows all 47 entities, their relationships, coverage status

  $ specforge query "what implements the payment flow?"
  Returns: 3 behaviors, 2 types, 1 port, linked to 5 test files

  $ specforge compile --format agent-context --scope payment
  Generates scoped context for AI agent: just the payment subgraph

New developer is productive in hours, not days. The spec graph is the
onboarding document that never goes stale.
```

### Journey 4: Scaling to Team (Month 6)

```
Team has 150+ entities across 30 .spec files. They need:
  - Namespace organization (spec/auth/, spec/payments/, spec/infra/)
  - Cross-file references validated at compile time
  - CI gate: `specforge check --strict` fails on any warning
  - Coverage dashboard in PR comments via `specforge report --format github`
  - Selective compilation: `specforge check spec/payments/` for fast iteration

The team has not purchased anything. SpecForge is still a free CLI tool.
The value scales with the project's complexity.
```

---

## 4. Feature Prioritization Framework

We use a weighted scoring model with four dimensions. Each scored 1-5.

| Dimension | Weight | Definition |
|-----------|--------|------------|
| **Agent Impact** | 35% | Does this feature measurably improve AI agent task-completion rates? |
| **Adoption Friction** | 25% | Does this reduce time-to-value or remove barriers to adoption? (Inverse: high friction = low score) |
| **Ecosystem Leverage** | 20% | Does this enable the community to build on top of SpecForge? |
| **Revenue Proximity** | 20% | Does this create or protect a future revenue opportunity? |

**Scoring rubric:**

- **5 — Transformative.** This feature is why someone adopts SpecForge over the alternative.
- **4 — Significant.** This feature meaningfully improves retention or expansion.
- **3 — Expected.** Users would be surprised if this were missing.
- **2 — Nice-to-have.** Improves experience but not a decision driver.
- **1 — Marginal.** Low impact; build only if trivial.

**Hard gates (any of these = automatic deprioritization):**
- Requires an account or cloud service for basic functionality
- Breaks backward compatibility of the .spec format without a migration path
- Adds mandatory configuration that did not exist before
- Increases cold-start compile time beyond 500ms for a 200-entity project

**Example scoring:**

| Feature | Agent Impact (35%) | Adoption (25%) | Ecosystem (20%) | Revenue (20%) | Weighted |
|---------|-------------------|-----------------|------------------|---------------|----------|
| Tree-sitter parser with error recovery | 4 | 5 | 4 | 3 | 4.05 |
| Agent-context output format | 5 | 4 | 3 | 3 | 4.00 |
| LSP with autocomplete | 3 | 5 | 3 | 2 | 3.30 |
| Cloud-hosted spec graph | 3 | 2 | 4 | 5 | 3.35 |
| Plugin marketplace | 2 | 2 | 5 | 4 | 3.00 |
| Figma provider | 2 | 2 | 3 | 3 | 2.40 |

---

## 5. MVP Definition

### What "MVP" Means for SpecForge

The MVP is the minimum product that makes an AI-first developer say: **"I am never going back to plain CLAUDE.md."** This is a high bar. It requires not just parsing a file but delivering a validated graph that demonstrably improves agent output quality.

### MVP: In Scope

| Component | Scope | Rationale |
|-----------|-------|-----------|
| **Parser** | Tree-sitter grammar for all 16 entity types. Multi-error recovery. | Core compiler. No parser = no product. Error recovery is non-negotiable because .spec files are edited live. |
| **Semantic model** | Entity graph with all 20 edge types. String interning via lasso. petgraph-backed. | The graph is the product. Without validated edges, we are just a linter. |
| **Validation** | All 36 validation codes (15 errors, 17 warnings, 4 info). Dangling reference detection. Cycle detection. | Validation is what separates SpecForge from a text file. Every error caught at compile time is a round-trip saved with the AI agent. |
| **CLI** | `specforge init`, `specforge check`, `specforge compile`. Three commands. | Init for onboarding. Check for CI. Compile for output. These three cover the entire MVP workflow. |
| **Output: agent-context** | JSON output format optimized for AI agent consumption. Token-efficient. Deterministic. | This is the killer feature. The reason someone installs SpecForge is to feed better context to their AI agent. |
| **Output: human-readable** | Pretty-printed terminal output with ariadne diagnostics. | Developers need to read error messages. ariadne gives us Rust-compiler-quality diagnostics for free. |
| **Core plugin system** | Load `@specforge/product` and `@specforge/governance` as built-in plugins. | 16 entities are the minimum viable expressiveness. 8 core entities alone cannot model real projects. |
| **Distribution** | brew, npx, cargo install, GitHub releases (Linux/macOS/Windows). | Four channels cover 95%+ of the developer audience. npx for zero-install trial. |
| **Spec format v1** | Stable .spec grammar with backward-compatibility commitment. | Developers will not adopt a format that breaks. v1 stability is a trust signal. |

### MVP: Out of Scope (and Why)

| Component | Deferred To | Rationale |
|-----------|-------------|-----------|
| **LSP** | Q2 | The LSP is high-value but not required for the "aha" moment. The CLI alone proves the concept. LSP is a retention feature, not an acquisition feature. |
| **Generators (test scaffolding, diagrams)** | Q3 | Code generation requires the spec format to be stable. Shipping generators on an unstable format creates upgrade debt. |
| **Providers (GitHub, Jira, Figma)** | Q3-Q4 | External integrations multiply the maintenance surface. The MVP must prove value with zero external dependencies. |
| **Watch mode** | Q2 | Nice-to-have for iteration speed. Not required for first value. |
| **Cloud/web dashboard** | Q5+ | Premature. No cloud until the CLI has proven product-market fit with thousands of users. |
| **`specforge trace` / test report consumption** | Q3 | Traceability requires the test ecosystem (proc macros, JUnit parsing) which is a large surface area. Ship the spec compiler first. |
| **`specforge query`** | Q3 | Graph querying is powerful but not required for MVP. `compile --format agent-context --scope X` covers the 80% use case. |
| **`scenario` blocks** | Q2 | verify blocks alone are sufficient for MVP test declarations. Scenarios add Gherkin-style complexity that can wait. |
| **User-defined types (`define` blocks)** | Q3 | The 16 built-in types cover most projects. Meta-schema extensibility is important but not urgent. |

### MVP Success Criteria

The MVP ships when all of the following are true:

1. A developer can go from `brew install specforge` to validated output in under 60 seconds.
2. `specforge check` catches at least 5 real error classes that would cause an AI agent to generate incorrect code.
3. `specforge compile --format agent-context` produces output that is 70%+ smaller than equivalent plain-text context while preserving all semantic information.
4. The compiler handles 200+ entities with sub-500ms compile time.
5. The .spec format is documented with a language reference and 10+ example files.
6. At least 3 real-world projects (including SpecForge itself) are self-hosting on the MVP.

---

## 6. Feature Roadmap by Quarter

### Q1 2026 (Now - June 2026): Foundation

**Theme: "Make it compile."**

| Milestone | Deliverables | Success Metric |
|-----------|-------------|----------------|
| **M1: Parser** | Tree-sitter grammar, CST-AST transform, multi-error recovery | Parses all 16 entity types; recovers from 3+ errors per file |
| **M2: Semantic Model** | Entity graph, edge resolution, string interning, petgraph integration | All 20 edge types resolve correctly; cycle detection works |
| **M3: Validation** | All 36 validation codes, ariadne diagnostics | 100% of validation codes have snapshot tests |
| **M4: CLI v0.1** | `init`, `check`, `compile` commands, agent-context output | End-to-end: init - write spec - check - compile in <60s |
| **M5: Self-hosting** | SpecForge's own specifications written in .spec format | 50+ entities, used as primary test corpus |
| **M6: Distribution** | brew tap, npm package, crates.io, GitHub releases | All 4 channels functional; CI builds for linux-x64, darwin-arm64, win-x64 |

**Key risk:** Tree-sitter grammar complexity. Mitigation: start with core 8 entities, add plugin entities incrementally.

### Q2 2026 (July - September 2026): Developer Experience

**Theme: "Make it delightful."**

| Milestone | Deliverables | Success Metric |
|-----------|-------------|----------------|
| **M7: LSP v0.1** | Diagnostics, go-to-definition, hover, document symbols | Works in VS Code + Neovim; <100ms response time |
| **M8: Watch mode** | `specforge watch` with incremental recompilation | Recompile on save in <200ms for 200-entity project |
| **M9: VS Code extension** | Syntax highlighting, LSP client, snippet templates | Published on VS Code Marketplace; 500+ installs in first month |
| **M10: Scenario blocks** | Full Gherkin-style given/when/then syntax | Parsed, validated, included in agent-context output |
| **M11: Scope filtering** | `--scope namespace` flag for selective compilation | Agent-context output scoped to subgraph |
| **M12: Error UX** | Fuzzy suggestion for misspelled references (strsim), fix-it hints | "Did you mean 'user_login'?" on typos |

**Key risk:** LSP performance on large files. Mitigation: incremental parsing via tree-sitter's edit API.

### Q3 2026 (October - December 2026): Ecosystem Launch

**Theme: "Make it extensible."**

| Milestone | Deliverables | Success Metric |
|-----------|-------------|----------------|
| **M13: Generator API** | Public generator trait, template engine, `specforge gen` command | 2+ first-party generators shipping |
| **M14: Rust generator** | `@specforge/gen-rust` — test scaffolding, proc macro crate | Self-hosting: SpecForge's own tests generated from specs |
| **M15: TypeScript generator** | `@specforge/gen-typescript` — test scaffolding for vitest/jest | 1 flagship TypeScript project using it |
| **M16: Test traceability v1** | `specforge trace`, `specforge report`, JUnit XML consumption | Coverage percentage calculated for self-hosting project |
| **M17: Provider API** | Provider trait, ref scheme registration, `specforge providers` | API documented, 1 first-party provider shipping |
| **M18: GitHub provider** | `@specforge/gh` — validate issue/PR references, resolve URLs | Links from spec entities to GitHub issues validated at compile time |
| **M19: `define` blocks** | User-defined entity types via meta-schema | Custom types compile and validate |

**Key risk:** Generator API stability. Mitigation: mark as `0.x` with explicit instability warning; gather feedback from 3+ generator authors before stabilizing.

### Q4 2027 (January - March 2027): Enterprise Readiness

**Theme: "Make it trustworthy."**

| Milestone | Deliverables | Success Metric |
|-----------|-------------|----------------|
| **M20: CI integration** | `specforge check --strict` exit codes, `specforge report --format github` PR comments | Used in 10+ CI pipelines |
| **M21: Multi-file projects** | Cross-file reference resolution, namespace support, import statements | 500+ entity projects compile correctly |
| **M22: Jira provider** | `@specforge/jira` — bidirectional sync of requirement status | 1 enterprise pilot using it |
| **M23: Spec format v1.1** | Backward-compatible additions based on 6 months of feedback | Zero breaking changes from v1.0 |
| **M24: Compliance output** | Requirements traceability matrix (RTM) generator for FDA/ISO | Generates RTM that a compliance officer can submit |
| **M25: Drift detection** | `specforge gen --check` for CI, checksum headers in generated files | Zero drift in self-hosting project over 30-day period |

### Q5-Q6 2027 (April - September 2027): AI-Native Features

**Theme: "Make the AI loop tight."**

| Milestone | Deliverables | Success Metric |
|-----------|-------------|----------------|
| **M26: Spec inference** | `specforge infer` — generate .spec from existing codebase | Infers 60%+ of entities from a well-structured Rust/TS project |
| **M27: Agent SDK** | Programmatic API for AI agents to read/query the spec graph | 2+ AI tools integrate (Claude Code, Cursor, Cody) |
| **M28: Diff-aware context** | `specforge compile --diff HEAD~1` — only entities affected by recent changes | 90%+ token reduction for incremental agent tasks |
| **M29: Spec chat** | `specforge ask "what depends on UserRepository?"` — natural language graph queries | Answers 80%+ of common graph questions correctly |
| **M30: Feedback loop** | Agent writes code - tests run - specforge-report.json - agent sees failures - agent fixes | End-to-end autonomous loop demonstrated |

### Q7-Q8 2027-2028 (October 2027 - March 2028): Platform

**Theme: "Make it the standard."**

| Milestone | Deliverables | Success Metric |
|-----------|-------------|----------------|
| **M31: SpecForge Cloud** | Hosted spec graph, web dashboard, team permissions | 100 teams on waitlist before launch |
| **M32: Cross-repo traceability** | Spec graphs that span multiple repositories | 3 organizations using cross-repo features |
| **M33: Plugin marketplace** | Community plugin registry, versioning, discovery | 20+ community plugins published |
| **M34: Figma provider** | `@specforge/figma` — link specs to design components | 5 design-to-dev teams using it |
| **M35: Org-wide coverage** | Aggregate spec coverage across all repos in an org | CTO-level dashboard showing org-wide spec health |

---

## 7. Platform Strategy (CLI - LSP - Cloud - IDE Extensions)

### Layer 1: CLI (Q1 — permanent foundation)

The CLI is the atomic unit of SpecForge. Everything else builds on it.

**Design principles:**
- Zero configuration required. `specforge check` works in any directory with .spec files.
- Machine-readable output on every command (`--format json`). CI systems and AI agents are first-class consumers.
- Sub-second execution for typical projects. The compiler is fast because it is written in Rust and the spec graph is small relative to code.
- The CLI is the single binary. No daemon, no background process, no server. Stateless by default.

**Why CLI first:** Developer tools that start with a GUI or a cloud service get adoption wrong. The CLI proves the core value proposition (validated spec graph - better AI output) with zero friction. It is also the integration point for CI, for AI agents, and for other tools. Everything else is a layer on top.

### Layer 2: LSP (Q2 — developer experience)

The LSP is how SpecForge becomes a daily-use tool rather than a CI gate.

**Capabilities by phase:**
- **LSP v0.1 (Q2):** Diagnostics, go-to-definition, hover info, document symbols
- **LSP v0.2 (Q3):** Autocomplete for entity references, rename symbol across files
- **LSP v1.0 (Q4):** Code actions (quick fixes), workspace-wide diagnostics, semantic tokens for rich highlighting

**Architecture:** The LSP binary (`specforge-lsp`) shares the parser library with the CLI binary (`specforge-cli`). The mutable graph (architectural decision from day 1) enables incremental updates on every keystroke without full recompilation.

### Layer 3: IDE Extensions (Q2-Q4 — reach)

**Priority order:**
1. **VS Code** (Q2) — 70%+ market share among target persona.
2. **Neovim** (Q2) — LSP works natively, syntax highlighting via tree-sitter grammar.
3. **JetBrains** (Q5) — Significant enterprise market share but lower priority.
4. **Zed** (Q6) — Growing AI-first editor. Natural fit but smaller user base.

**Extension scope:** IDE extensions are thin clients. They provide syntax highlighting, LSP integration, and convenience commands. All intelligence lives in the LSP.

### Layer 4: Cloud (Q7+ — monetization and team features)

The cloud layer is deliberately deferred to Q7. Premature cloud features would distract from core compiler quality.

**Cloud features (when ready):**
- Hosted spec graph with web dashboard
- Team permissions and role-based access
- Cross-repository traceability
- Spec coverage trends over time
- Webhook integrations (Slack notifications on spec coverage regressions)
- API access for custom integrations

**Pricing model (preliminary):**
- Free: CLI, LSP, all local features, community plugins. Forever.
- Team ($20/user/month): Cloud dashboard, cross-repo traceability, team permissions.
- Enterprise ($50/user/month): SSO, audit logs, compliance reports, SLA.

---

## 8. Ecosystem Strategy

### Extension Taxonomy

| Extension Type | Purpose | Examples | Distribution |
|---------------|---------|----------|-------------|
| **Plugins** | Add entity types and edges | `@specforge/product`, `@specforge/governance`, `@community/security` | Registry (future), git repos (near-term) |
| **Providers** | Validate external references | `@specforge/gh`, `@specforge/jira`, `@specforge/figma` | Same |
| **Generators** | Produce output from the graph | `@specforge/gen-rust`, `@specforge/gen-typescript`, `@community/gen-openapi` | Same |

### Phase 1: First-Party Ecosystem (Q1-Q4)

We build the first 8-10 extensions ourselves:
- 2 plugins: `@specforge/product`, `@specforge/governance` (shipped with core)
- 3 generators: `@specforge/gen-rust`, `@specforge/gen-typescript`, `@specforge/gen-markdown`
- 2 providers: `@specforge/gh`, `@specforge/jira`
- 1 test runner adapter: `@specforge/vitest` (or `@specforge/rust` as proc macro crate)

### Phase 2: Community Ecosystem (Q5-Q8)

**Community extension opportunities:**

| Category | High-Value Extensions |
|----------|----------------------|
| **Generators** | OpenAPI spec, GraphQL schema, Terraform modules, Kubernetes manifests, C4 diagrams, Mermaid diagrams |
| **Providers** | Linear, Notion, Confluence, Azure DevOps, GitLab |
| **Test adapters** | pytest, Go testing, JUnit, Playwright, k6, Cypress |
| **Language plugins** | `@community/security` (threat models), `@community/data` (schemas, migrations) |

### Phase 3: Marketplace (Q7+)

A hosted registry where community extensions are published, versioned, and discovered. Free to publish, free to install. Revenue comes from the cloud platform, not the marketplace.

---

## 9. AI-Native Features

### 9.1 Spec Inference (`specforge infer`) — Q5

Analyzes an existing codebase and generates a draft .spec file. Reduces the cold-start problem from hours to minutes.

### 9.2 Agent SDK — Q5-Q6

Programmatic API for AI coding agents to read, query, and update the spec graph. Integration targets: Claude Code, Cursor, Continue.dev.

### 9.3 Diff-Aware Context — Q5

`specforge compile --diff HEAD~1` outputs only the entities and edges that changed. Estimated 90%+ token reduction for incremental tasks.

### 9.4 Natural Language Queries — Q6

`specforge ask "what depends on the payment system?"` translates natural language to graph queries.

### 9.5 AI-Assisted Authoring — Q6

LSP-integrated AI suggestions while writing .spec files: suggest descriptions, missing edges, verify/scenario blocks.

### 9.6 Autonomous Feedback Loop — Q6

Closes the loop: Agent reads spec - generates code + tests - tests run - specforge-report.json - agent sees failures - agent fixes. The ultimate value proposition.

---

## 10. Success Metrics Per Feature

### Core Metrics (tracked continuously)

| Metric | Definition | Target (Y1) | Target (Y2) |
|--------|-----------|-------------|-------------|
| **Weekly Active CLI Users** | Unique users running any `specforge` command per week | 5,000 | 50,000 |
| **Spec Files in Public Repos** | GitHub code search for `.spec` files with SpecForge syntax | 1,000 | 25,000 |
| **Agent Task Completion Rate** | % of AI coding tasks completed correctly on first attempt with spec context vs. without | +40% improvement | +60% improvement |
| **Time to First Value** | Median time from install to first successful `specforge check` | <60 seconds | <30 seconds |
| **NPS (Developer)** | Net Promoter Score from in-CLI survey (quarterly, opt-in) | 50+ | 60+ |

### Funnel Metrics

```
Install (brew/npx/cargo)
  | (Target: 80% proceed)
First `specforge check`
  | (Target: 60% proceed)
10+ entities authored
  | (Target: 40% proceed)
Used in AI agent context (agent-context output)
  | (Target: 30% proceed)
Added to CI pipeline
  | (Target: 15% proceed)
Team adoption (3+ developers)
  | (Target: 5% proceed)
Cloud plan (paid)
```

**Key conversion to monitor:** The drop from "first check" to "10+ entities authored" is the critical funnel step. Mitigation: `specforge infer` (Q5), AI-assisted authoring (Q6), and excellent documentation (continuous).
