# Use cases

What you can do with SpecForge, field by field. Each scenario says what works **today**, with the builtin
extensions or one you write yourself, and what is **roadmap**. Runnable projects for several of these are in
[`examples/`](../../examples/README.md).

Two facts hold in every field:

- **An extension is how SpecForge learns your field.** With the [extension SDK](../extension-sdk.md) you declare
  entity kinds, their fields and links, the rules that judge them (declarative, such as "this link is required",
  or custom code), and commands. `specforge extension init`, `build` and `validate` scaffold, compile and check
  it; [Extending SpecForge](../guides/extending-specforge.md) walks through it.
- **The AI reaches SpecForge from outside**, mostly over MCP. Claude, or any MCP client, reads the graph, asks for
  the slice it needs, and changes it through typed edits (`new`, `set`, `link`, …) that are checked like any other
  change. People who don't use an editor or an agent have no screen of their own yet: AG-UI and HTTP surfaces are
  planned.

---

## 1. Software teams and software methods

**The situation.** An agent writes clean code and passing tests while missing a rule written in a Markdown spec.
Or the team follows a method (GSD, BMAD, OpenSpec, SpecKit) whose state lives in Markdown files nobody checks.

**Today, with the builtin extensions.**

- `@specforge/software` declares behaviors, invariants, events, types and ports. `@specforge/governance` adds
  decisions, constraints and failure modes; `@specforge/formal` adds properties an SMT solver proves.
- `specforge check` refuses a broken reference, an invariant no behavior enforces, an event nothing produces, or a
  missing required field, with the line and a fix hint.
- Each `verify` line is an obligation. Rust and vitest tests name the obligation they prove;
  `specforge collect` runs the test command (after you approve it) and `specforge review` shows what is proven
  and what is still open.
- `specforge infer-guide` and `infer-status` guide an agent that writes a spec from an existing codebase.
- The agent works over MCP: it asks for one entity and its neighbours instead of re-reading the repository.

**Today, with your own extension.** A method is a vocabulary plus rules. You can declare `phase`, `plan` and
`task` kinds, require every task to belong to a plan, and refuse a phase marked done while a task is open. The
method's state then lives in `.spec` files, checked on every change.

**Roadmap.** A GSD extension family (`@gsd/*`) is being designed, and some of the kernel changes it needs are
not built yet. BMAD, OpenSpec and SpecKit extensions are not started.

## 2. Product and marketing

**The situation.** The product plan is spread across a roadmap deck, a ticket tracker and a brand document. An
agent asked "which personas does the spring release serve, on which channels?" has to guess.

**Today, with the builtin extensions.**

- `@specforge/product` declares features, personas, channels, journeys, deliverables, milestones, releases,
  modules and terms (your team's vocabulary).
- The check refuses a journey that names a persona or channel that doesn't exist, and warns about a journey no
  deliverable ships or a deliverable that names no journey.
- Over MCP an agent can ask product questions directly: which features each persona touches, which milestones
  are complete, what is on the critical path, which features are unscheduled.
- [Product getting started](../guides/product-getting-started.md) walks through a first product spec.

**Today, with your own extension.** Marketing has its own things: campaigns, audiences, messages, claims, assets.
You can declare them and their rules: every campaign names an audience and a channel; every claim in a message
links to its approved source; nothing is scheduled after its release's end.

**Roadmap.** Pulling campaign data from an ad platform or a CRM: extensions can't reach external systems today.
A browser view for marketers who don't use an agent comes with the planned AG-UI and HTTP surfaces.

## 3. Pharma and other regulated industries (GxP)

**The situation.** Procedures, deviations, change controls and their approvals must trace to each other, and an
auditor asks for that trace. Today it lives in a quality system, documents and spreadsheets kept in sync by hand.
The rule of the field is "AI may draft, a qualified person signs".

**Today, with your own extension.** No builtin extension covers GxP; you write one. You can declare kinds such as
`procedure`, `deviation`, `capa` and `batch`, their links, and rules like "every deviation links to an
investigation", "a batch with an open deviation can't be marked released" or "every procedure names its owner".
The `.spec` files sit in version control, so every change has an author, a date and a diff, and the check runs in
CI. An agent can draft entities over MCP; the check refuses what breaks a rule; a person reviews and commits.

**Roadmap.** Several things a regulated team needs are not there yet, and SpecForge does not claim them:

- **Evidence beyond tests.** Proof in SpecForge today means test results. Recording a signed review or an
  executed procedure as evidence is future work.
- **Electronic signatures and validated-system status** (21 CFR Part 11, EU Annex 11). SpecForge is not a
  validated system and doesn't sign records.
- **Reading from your quality system or LIMS.** Not possible today.
- A builtin or published GxP extension. None exists yet.

## 4. Any business with things and rules worth checking

**The situation.** A small distributor, an agency or a clinic has its own entities (suppliers, products,
contracts, appointments) and its own rules ("every contract has a renewal date", "no product without a
supplier"). An AI assistant helps run it, but nothing stops the assistant from breaking those rules.

**Today, with your own extension.** Declare the kinds and the rules once. The graph becomes the business's
memory, validation its rulebook, and the AI its worker: the assistant reads and edits the graph over MCP, and
every edit goes through the check. This is SpecForge used as a **harness**: the business runs on it, with
extensions as its domain and AI models as its brain.

**Roadmap.** A screen for staff who don't use an AI agent or an editor (AG-UI, HTTP), and live connections to
the business's other systems. Until then, the graph holds what people and agents write into it.

---

## Shipped and planned, in one place

| | Today | Planned |
|---|---|---|
| Your own domain | Extension SDK, WASM runtime, `specforge extension init/build/validate`, registry and `publish` | Live connectors to external systems |
| Builtin extensions | software, product, governance, formal, testing, cargo-test, vitest, rust, typescript | — |
| Software methods | Your own method as an extension | `@gsd/*` (being designed); BMAD, OpenSpec, SpecKit (not started) |
| Doors | CLI, MCP, LSP and the VS Code extension | AG-UI, HTTP |
| Proof | Test results, through `specforge collect` | Other kinds of evidence (reviews, executed procedures) |
| Who uses it | Developers, and anyone working through an AI agent or an editor | Business users in a browser |
