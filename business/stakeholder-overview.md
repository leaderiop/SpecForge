# SpecForge — Stakeholder Overview

## What It Is

**SpecForge is the structured context standard for AI agents.** It's a specification language (`.spec` files) and compiler that turns human-written knowledge into a machine-readable graph that any AI agent can consume — across any domain.

Think of it as **the bridge between what your team *means* and what AI agents *understand*.**

## The Problem It Solves

AI agents today operate without structured understanding of your work. Whether you're building software, managing compliance, designing APIs, or running data pipelines — agents guess from scattered docs, comments, and chat history. This leads to:

- AI that contradicts your own rules because it never knew them
- No traceability from intent → decisions → outcomes
- Teams repeating the same context to every AI tool, every time
- No way to verify that an agent's plan respects your constraints

## How It Works

1. **You write `.spec` files** — a concise DSL describing the entities that matter to your work
2. **The compiler builds a typed entity graph** — resolving references, validating constraints, catching contradictions
3. **AI agents consume the graph** — via a standardized JSON schema (the "Graph Protocol") that any agent can read

```
.spec files  →  SpecForge Compiler  →  Entity Graph (JSON)  →  Any AI Agent
```

## Zero Built-In Concepts — You Choose Your Domain

**SpecForge's core has zero domain knowledge.** The compiler knows how to parse blocks, resolve references, build graphs, and validate constraints — but it has no opinion about *what you're describing*. All domain vocabulary comes from installable **extensions** that you select based on your work.

This is exactly how **Terraform** works: Terraform's core has zero infrastructure knowledge — all cloud concepts come from providers (AWS, GCP, Azure). SpecForge's core has zero domain knowledge — all concepts come from extensions.

### Example Extension Selection

| If your work involves... | You install... | You get entities like... |
|---|---|---|
| Software engineering | `@specforge/software` | behavior, invariant, feature, event, type, port |
| Product management | `@specforge/product` | journey, deliverable, milestone, module, term |
| Architecture governance | `@specforge/governance` | decision, constraint, failure_mode |
| Regulatory compliance | `@specforge/compliance` | regulation, control, evidence, audit |
| API design | `@specforge/api-design` | endpoint, schema, operation |
| UI/Design systems | `@specforge/atomic-design` | atom, molecule, organism, template, page |
| Data engineering | `@specforge/data-pipeline` | source, transform, sink, schedule |

You combine extensions freely. A compliance team might install `@specforge/compliance` + `@specforge/governance`. A full-stack team might use `@specforge/software` + `@specforge/product` + `@specforge/api-design`. Each combination shapes the graph that agents consume.

## Competitive Positioning

| | Traditional Approach | SpecForge |
|---|---|---|
| Format | Prose docs (Confluence, Google Docs) | Structured, compiler-checked DSL |
| AI consumption | Copy-paste into prompts | Direct graph ingestion |
| Validation | Manual review | Automated (orphan detection, missing references, contradictions) |
| Traceability | Spreadsheet matrices | Built into the graph |
| Extensibility | None | Open extension ecosystem |
| Domain scope | One format per domain | Any domain, one compiler |

**No direct competitor** occupies this space. Gherkin handles test scenarios. OpenAPI handles API contracts. FMEA templates handle failure modes. Each is siloed. SpecForge provides **one graph protocol across all domains** — and every AI agent reads the same schema.

## Business Model

- **Open-source core** — compiler, LSP, CLI, foundational extensions
- **Commercial extensions** — regulated industries (`@specforge/gxp`), enterprise governance
- **SaaS layer** (future) — hosted graph, team collaboration, agent marketplace
- **Network effect moat** — the Graph Protocol schema that all agents agree to read. The more agents that consume it, the more valuable every `.spec` file becomes.

## Key Metrics That Matter

- **Graph Protocol adoption** — how many AI agents/tools consume the schema
- **Extension ecosystem growth** — community and commercial extensions
- **Spec coverage** — % of a team's structured intent captured in `.spec` files vs. scattered docs

## What It Is NOT

- **Not a code generator** — SpecForge does not produce output artifacts. It provides structured context so that *agents* can do their job better.
- **Not a test runner** — it can consume test reports to verify coverage, but never executes anything itself.
- **Not documentation** — `.spec` files are compiler-checked contracts, not prose. They break the build when they're wrong.
- **Not limited to software** — any domain that benefits from structured knowledge and AI agents can use SpecForge.

---

**One sentence**: SpecForge lets any team — engineering, product, compliance, data — turn their domain knowledge into a structured graph that AI agents can read, making agents reliable collaborators instead of context-starved guessers.
