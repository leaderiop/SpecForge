# North star

This is not a roadmap and has no dates. It describes the world SpecForge is built toward, and marks plainly what
exists today and what doesn't yet.

---

## The world we are building

An organisation's knowledge lives in one checked graph. Its things, their links and their rules are written once,
in `.spec` files under version control, and checked on every change.

AI agents read that graph before they act and write to it through typed edits. A coding agent reads the spec
before it builds. A product agent reads the plan before it writes the status report. A quality agent reads the
procedures before it drafts a batch record. When an agent proposes a change that breaks a rule, the change is
refused with the reason, and the agent fixes it. People spend their time on what the graph says is still open,
not on re-reading everything the AI produced.

Extensions exist for fields nobody at SpecForge planned: shipping, clinical trials, game design, a family
business's own way of working. They are versioned and installed like libraries, from a registry anyone can publish
to.

When someone asks "how does this AI know what it's allowed to do?", the answer is: it reads the graph, and the
graph is checked.

---

## Three horizons

Maturity stages, not calendar milestones. Each builds on the last.

```
  H1: The engine           H2: The ecosystem         H3: The harness
 ┌────────────────────┐  ┌────────────────────┐  ┌────────────────────┐
 │ compiler, SDK,     │  │ domain extensions  │  │ a business runs on │
 │ CLI, MCP, LSP,     │─>│ from many authors  │─>│ it: AI as worker,  │
 │ builtin extensions │  │ (methods, pharma,  │  │ people review, new │
 │ one team, one repo │  │ product, …)        │  │ doors for everyone │
 └────────────────────┘  └────────────────────┘  └────────────────────┘
  Competes with:          Competes with:          Competes with:
  prose and habit         every tool inventing    closed platforms that
                          its own format          own your knowledge
```

### H1: The engine (today)

This exists. The compiler and validator, the extension SDK and WASM runtime (`specforge extension init|build|validate`),
the builtin extensions, the CLI, the MCP server, the LSP with its VS Code extension, the package registry, typed
edits (`specforge new|set|unset|link|unlink|delete`), `collect` for test results, and the `infer` guides that help
an agent write a spec from existing code.

One person in one project gets value on day one, without asking their team. That is how lasting tools start.

### H2: The ecosystem (starting)

Many extensions, written by many authors, for many fields:

- **Software methods as extensions.** GSD, BMAD, OpenSpec and SpecKit each have a vocabulary (milestones,
  phases, stories, proposals, tasks) and rules. As extensions, their state becomes checked entities instead of
  Markdown files. A GSD family (`@gsd/*`) is being designed; the others are not started.
- **Regulated domains.** Pharma quality, medical devices, safety cases: procedures, deviations, approvals and
  their evidence, with rules a regulator would recognise. Not built yet; anyone can start one with the SDK.
- **Product and business vocabularies** beyond the builtin `@specforge/product`.

The registry and `specforge publish` exist today. The extensions above are what is missing.

### H3: The harness (planned)

A business runs on SpecForge. Its extensions are its domain, the graph is its memory, validation is its rulebook,
and AI models are the workers. This needs pieces that don't exist yet:

- **New doors.** AG-UI and HTTP surfaces, so a business user can work with the graph in a browser or another app
  without an editor or an agent in between. Planned.
- **Connections to the systems a business already has.** Today extensions are pure (they can't open files or the
  network), and external reference schemes register but nothing reads them yet. Reading from a CRM, an issue
  tracker or a quality system is future work.
- **Evidence beyond tests.** Proof today means test results. A signed review, an executed procedure or an audit
  record as evidence is future work.

---

## The measure of success

One number, for any domain:

**What share of AI agent tasks come out right the first time when the agent has the checked graph?**

We believe the graph moves this number a long way. That is a hypothesis, not a result: `evaluation/` holds a
harness to measure it, and no run has been recorded yet. If SpecForge reaches a hundred thousand projects and that
number hasn't moved, we have failed. If it reaches a thousand and the number has moved a lot, we are on the right
path.

---

## What we refuse to become

Each of these will come with good-sounding arguments. We will resist them.

- **A generator.** SpecForge provides checked knowledge; agents produce output. Reports that help you read the
  graph are part of SpecForge; code, documents and anything shipped to your customers are not.
- **An AI.** The check has no model inside, so its verdict can be trusted the same way twice. The brain is yours
  to choose.
- **A test framework.** SpecForge consumes test results; runners stay runners. A runner extension declares one
  command, and `specforge collect` runs it only with your consent
  ([ADR 0002](../adr/0002-test-runner-extensions.md)).
- **Software-only.** The core has no domain so that every field gets equal treatment through extensions.
- **Enterprise-only.** A solo user with no manager must be able to choose it and get value.
- **A walled garden.** Extensions, the registry and the export format are open. If the ecosystem can't grow
  without our permission, the design is wrong.
