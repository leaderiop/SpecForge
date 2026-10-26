# Why SpecForge exists

AI can now do real work: write code, draft a campaign brief, prepare a batch record, answer a customer. What it
works from is the weak part. The knowledge that should guide it (what your organisation's things are, how they
relate, and which rules they must obey) lives in documents, wikis, tickets, spreadsheets and people's heads.
The AI reads some of it, guesses the rest, and nothing checks the result against the rules.

SpecForge fixes where that knowledge lives. **It turns what an organisation knows into one typed, validated graph
that people and AI agents read and change through standard protocols.** A change that breaks a rule is refused,
whoever made it.

This page is the why. The [principles](principles.md) are the beliefs a design decision is tested against, the
[north star](north-star.md) is where this is going, and the [use cases](use-cases.md) say what you can do with it
today, domain by domain.

---

## The problem: unchecked knowledge

Every field has two kinds of knowledge.

- **Things and links**: the products, people, procedures, features, campaigns, batches, and how each one
  depends on the others.
- **Rules**: what must always be true. "Every refund has a reason." "Every campaign names its audience."
  "Every deviation has an investigation before the batch is released."

Today both sit in prose. Prose has no referee. A rule in a document is advice: an AI may follow it, skip it, or
follow last month's version. A link in a wiki can point at something that no longer exists, and nothing breaks.
The faster AI works, the more of its output nobody has time to check.

This is not a model problem. A better model still reads prose and guesses. It is a missing layer: a place where
knowledge is structured, and where the rules are checked by a program, the same way every time.

Other fields got that layer once machines started consuming them. Databases got SQL schemas. APIs got OpenAPI.
Infrastructure got Terraform. AI agents, which now consume an organisation's knowledge, have no such layer yet.

## The idea: hard skills, not soft skills

There are two ways to teach an AI how your organisation works.

A **soft skill** is advice in the AI's context: a prompt, a Markdown guide, a policy PDF, a skill file. It helps,
but the AI can ignore it, and you can't tell when it did.

A **hard skill** is a rule a program enforces. The AI can't skip it, because the check fails and says why.

SpecForge is an engine for hard skills. You (or your agent) write down your things, their links and their rules
in small `.spec` files. SpecForge compiles them into a graph and checks every rule on every change. The AI then
works from the checked graph, not from prose, and every edit it proposes goes through the same check.

## How it is built: five parts

| Part | What it is | In plain words |
|---|---|---|
| **Extensions** | WASM packages that declare entity kinds, their fields and links, the rules that judge them, and commands | The "apps". They teach SpecForge a domain |
| **The graph** | Every entity and link, compiled from `.spec` files | The "memory": one source of truth |
| **Validation** | The extensions' rules plus structural checks, run on every compile and every edit | The "referee": a change that breaks a rule is refused, whoever made it |
| **Surfaces** | CLI, MCP, LSP with a VS Code extension, and a package registry today; AG-UI and HTTP planned | The "doors" people and agents come in through |
| **AI models** | Claude or any other agent, driving SpecForge from outside, mostly over MCP | The "brain" or "worker". Bring your own |

```
            people                         AI agents (Claude, any MCP client)
               │                                         │
     ┌─────────┴─────────── surfaces ────────────────────┴─────────┐
     │   CLI      MCP      LSP + VS Code      registry   (AG-UI, HTTP: planned)
     └─────────────────────────────┬───────────────────────────────┘
                                   │ read, query, typed edits
                     ┌─────────────┴──────────────┐
                     │   the graph  +  validation │   no AI inside: same input,
                     └─────────────┬──────────────┘   same verdict, every time
                                   │ kinds, fields, links, rules, commands
     ┌─────────────────────────────┴───────────────────────────────┐
     │ extensions: @specforge/software, product, governance, …     │
     │             your own: @acme/quality, @gsd/core, …           │
     └─────────────────────────────────────────────────────────────┘
```

## No domain in the core

This is the most important design decision.

The core grammar knows five structural words: `spec`, `ref`, `use`, `define` and `defaults`. It does not know
what a "behavior", a "persona", a "campaign" or a "batch" is. Every kind of entity, every field, every link and
every rule comes from an extension. The core parses, links, checks and serves the graph. That is all.

The extensions that ship inside the binary (`@specforge/software`, `product`, `governance`, `formal`, `testing`,
and the runner and language extensions) are a starter set. They are good examples of what an extension can
declare, and they cover software and product work well. They are not the base: a project can load none of them
and use only its own.

The test of this decision is simple: if a new domain needs a change to the core rather than a new extension, the
design has failed. A maritime logistics team should be able to write `@acme/shipping`, and a clinical research
group `@lab/trials`, without anyone at SpecForge having planned for them.

## Two ways to use it

- **As a product.** A team installs SpecForge, loads the extensions it needs, and uses it as the checked source
  of truth for its specs, its product plan or its procedures, next to the tools it already has.
- **As a harness.** A business runs on SpecForge: its own extensions are its domain, the graph is its memory,
  validation is its rulebook, and AI models do the work through the surfaces. People review what the graph says
  is done and what is still open.

The first works today. The second works today for teams whose people reach SpecForge through an AI agent or an
editor. A screen for business users who use neither comes with the planned AG-UI and HTTP surfaces.

## What SpecForge is not

- **Not an AI.** It makes no model calls. It is a deterministic compiler, written in Rust. The AI is yours.
- **Not a generator.** It doesn't write your code, your documents or your campaigns. Agents do, from knowledge
  SpecForge has checked. Reports that help you read the graph (coverage, traceability) are part of SpecForge;
  anything shipped to your customers is not.
- **Not a test framework.** Tests stay in `cargo test`, vitest and the like. A runner extension declares the one
  command that runs them, and `specforge collect` runs it only after you approve it
  ([ADR 0002](../adr/0002-test-runner-extensions.md)). Extensions themselves stay pure: no files, no network.
- **Not only for software.** Software specification is one of its biggest uses, not its definition. Any field
  with things, links and rules worth checking can use it.
- **Not a connector hub, yet.** Extensions can't reach your CRM or your quality system today. The graph holds
  what is written into it.

## Who this is for

Anyone whose work has rules worth checking and who wants AI to help without guessing:

- **Software teams** who want agents to build against a checked spec instead of a Markdown file they may skip,
  and teams who want their own method (GSD, BMAD, OpenSpec, SpecKit, or their own) to be checked, not just
  documented.
- **Product and marketing teams** who want one checked map of features, personas, journeys, channels and
  releases that an agent can report from.
- **Regulated industries** (pharma, medical devices, safety engineering) where "the AI drafts, the rules check,
  a qualified person signs" is how the work has to be done.
- **Any business** with entities and rules it wants an AI to respect.

The [use cases](use-cases.md) walk through each one, with what is possible today and what is still roadmap.

---

*SpecForge turns what you know into a checked graph. The AI is the worker. The graph is the memory. The rules
are the referee.*
