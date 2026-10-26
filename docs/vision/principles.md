# Principles

These are the beliefs that govern how SpecForge is built. When a decision is unclear, they settle it. If a
feature or a technical choice contradicts one, the principle wins and the decision is revisited.

Each principle has a **test**: a question that tells us whether we live up to it. A principle without a test is
a wish.

---

## 1. Structure is a spectrum, not a binary.

You don't need to model your whole domain before SpecForge helps. One entity in a `.spec` file is better than
zero. Ten entities are better than one. A hundred entities with full traceability is transformative.

A single behavior with two verify declarations already gives an AI agent more to work with than a page of prose.
The same holds outside software: one procedure with its required approvals, or one campaign with its audience and
channel, is already a rule an agent can't skip. Start where the pain is, and grow as the value compounds.

SpecForge must never require complete coverage to be useful.

**The test:** a team with five spec files and fifty unstructured documents should still see its AI make fewer
mistakes than with zero spec files.

## 2. The core knows nothing about your domain.

The core grammar has five structural words (`spec`, `ref`, `use`, `define`, `defaults`) and no entity kinds.
Every kind, field, link and rule comes from an extension. The core parses, links, checks and serves the graph.

This is what lets SpecForge serve fields its authors never planned for. The builtin extensions are a starter set
that proves the model, not a base everyone inherits.

**The test:** if a line of core code names a domain concept ("behavior", "campaign", "batch"), it is in the
wrong place. Core may only name structural concepts: entity, field, edge, reference, rule.

## 3. Agents are first-class users.

Every output, error message and query is designed for machines as much as for people: structured JSON in a stable
order, schemas that don't break between versions, queries that return exactly the slice an agent needs, and
answers small enough for a context window. If a person can read an output but an agent can't parse it reliably,
it is a bug.

**The test:** every new output and every schema change answers one question: does it help an agent do the task
right the first time?

## 4. Validation is the value.

Without the check, a `.spec` file is a text file with syntax. The value is in what the check catches: a link to
something that doesn't exist, a rule nothing enforces, a required field left empty, a cycle, a duplicate, a
promise with no proof.

The check is impartial. The same rules run on a person's edit and an agent's edit, in the editor, on the command
line and over MCP. No AI is involved: the same input gives the same verdict every time.

**The test:** when someone asks "why not just use Markdown with conventions?", the answer is always the same:
Markdown doesn't refuse a change that breaks a rule.

## 5. Traceability is a feedback loop, not a report.

Promises link to their proof, proof produces results, and results feed back into the graph. That loop is what
lets an agent see what is done and what is still open, and work on the gap instead of starting over.

```
     ┌──────────┐       ┌──────────┐       ┌──────────┐
     │ Promises │ ────> │  Proof   │ ────> │ Results  │
     │ (.spec)  │       │ (tests…) │       │ (report) │
     └──────────┘       └──────────┘       └────┬─────┘
          ▲                                      │
          │         ┌──────────┐                 │
          └──────── │  Agent   │ <───────────────┘
                    │ (reads   │
                    │  graph)  │
                    └──────────┘
```

Today the proof is tests: `verify` obligations that a test runner extension maps to test results through
`specforge collect`. Other kinds of evidence (a signed review, an executed procedure) are future work.

**The test:** a project that has fed results back for six months should give agents better answers than the same
project on day one, because the record of what is proven has grown.

## 6. Open doors, standard protocols.

People and agents reach the graph through standard protocols (MCP for agents, LSP for editors, a plain CLI for
scripts and CI), not through a format only SpecForge can read. The graph's JSON export is versioned and
documented, so other tools can produce and consume it.

**The test:** for every feature, ask: does this make the graph easier to reach from tools we didn't write, or does
it lock users into ours? If it locks in, redesign it.

## 7. Extensions over built-ins, always.

When in doubt, make it an extension. When it feels like it belongs in the core, make it an extension anyway. A
clean way for anyone to write fifty domain vocabularies beats fifty kinds baked into the core: each baked-in kind
limits who can use SpecForge, and each extension invites a new field in.

**The test:** before adding anything to the core, ask "could this be an extension?" If yes, even at some cost in
convenience, it becomes one.

## 8. Seconds to value, not days.

Install to a first checked result in under a minute. No account, no configuration ceremony, no wizard.
`specforge init` writes a starter file, `specforge check` checks it. Everything else (extensions, the registry,
the editor, an agent) is opt-in.

**The test:** time it from running the install script to seeing a verdict from `specforge check`. Over a minute
is a top-priority bug.
