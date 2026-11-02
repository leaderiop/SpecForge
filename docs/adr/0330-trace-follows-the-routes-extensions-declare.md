# Trace follows the routes extensions declare

**Status:** accepted (2026-11-02, R1 of the Beyond Parity build plan, plan 06). Extends
[ADR 0012](0012-one-extension-declaration.md) (a describe category), [ADR 0015](0015-read-views-are-operations-over-the-project-view.md)
(a read view), [ADR 0050](0050-the-guest-wire-carries-only-what-guests-compile.md) (thirteen categories from 1.15),
[ADR 0174](0174-the-graph-owns-directed-reach-and-degree.md) (the reach stays the graph's),
[ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) (the selector grammar, judgement state) and
[ADR 0299](0299-the-graph-a-call-receives-carries-what-every-surface-shows.md) (judgements are read, not sent). Amends
[ADR 0029](0029-operations-decide-their-failure-and-verdict.md) D5b. Ships in extension protocol 1.15.0, with
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md).

## Context

The walking skeleton's exit is one line:

```
TASK-01 → plan 01-01 → T1 → <sha> (Gsd-Task) → js:taskboard/test/tasks.test.js#a task is added with the next id (passed) → TR1 verified, fresh
```

`specforge trace TASK-01` gives none of it. It is a generic reach both ways over every edge (`read/trace.rs`): about
51 upstream rows on the taskboard twin, and no commit, no test status and no verdict state (explore card G5). What a
requirement's thread is depends on the domain: a gsd requirement goes through plans, tasks and commits, and is proven
by tests a truth names. The kernel knows no such kind.

The owner's answer U1 puts the thread in the kernel: an extension declares *routes* (a start kind, ordered steps over
its edges, annotations from fields, judgements and facts) and the kernel's `trace` follows them on every surface. The
coordinator settled the output (N1–N5): a routed entity prints its human thread by default; steps are joined by ` → `;
a step prints a label and its entity's name; annotations follow in parentheses, a judgement bare; only a complete
thread prints by default, a partial one with `--partial`.

## Decision

- **D1. The declaration.** A describe category `routes` (protocol 1.15), the thirteenth. A `RouteDescriptor` has a
  kebab `name` (unique in its extension, qualified by the extension's short name: `gsd/requirement-thread`), an
  optional `description`, a `start` kind, a `default` flag, the start's `annotations` and ordered `steps`. A step has a
  word `name` (never `start`), a `from` (`start` or an earlier step; absent, the previous step), a `via` written in ADR
  0290's selector grammar without its head (`<plan.requirements`, `.proof`, `<truth.requirement.proof`), an optional
  `label` and `annotations`. An annotation has exactly one of `field` (with an optional selector `of` that picks the
  entities whose field is read) and `judgement` (a judged rule's code), and an optional `absent` text. A 1.14 guest is
  never asked for the category. The SDK builds it with `ContributionsBuilder::route`.
- **D2. The registry checks each route once.** Every check runs where every loaded kind is known:
  - the route's name, its steps' names, `from` naming `start` or an earlier step;
  - `via` and `of` parse as selector steps and type-check from the kinds their `from` reaches, through the judged
    rules' own `walk` (a computed or verdict-written reference makes no edge, so a route can't follow it either);
  - a `field` is declared on every kind reached; a `judgement` names a judged rule targeting every kind reached.

  A route that fails is **E118** and is not registered. A start, step or `of` that reaches a kind of an optional peer
  the project does not load is not an error: the route is dropped silently when it is its start kind, otherwise the
  step, the steps that go on from it, or the annotation are recorded as *skipped* (ADR 0096's enhancement precedent).
  Several routes from one kind with not exactly one `default` is **W195**, and the first in load order is followed.
- **D3. What `trace` follows.** `Follow::Declared` (the default) follows the start entity's kind's route, else the
  generic chain. `Follow::Route(name)` follows that route, refused when no loaded extension declares it
  (`unknown_route`) or it starts at another kind (`route_of_another_kind`). `Follow::Reach` is the generic chain.
  With no entity, `Declared` traces each entity as its own trace would, `Route` every entity of its start kind, and
  `Reach` every generic chain. `--plan` takes no route (`plan_with_route`).
- **D4. The evaluation.** The start is one match. For each step in order, for each match of its `from` in order, the
  entities its `via` selects from that match, in id order, are matches under it. Steps go on only from earlier steps,
  so it ends. The selection is the judged rules' evaluator, moved to `specforge_project::select`: one parser, one
  type-check, one evaluator. Facts are entities (ADR 0327), so a step reaches them as any entity; an unheld reference
  is no match.
- **D5. Annotations.** A field prints the values it holds on the entity (or on each entity `of` picks): a reference
  as its target's name, lists flattened, scalars unquoted, each once, as ` (v1, v2)`. A judgement is read from the
  view's judgements (no protocol field, no recompute; ADR 0299 D5) and prints ` <outcome>, <state>` when an outcome
  is recorded (`verified, fresh`), else ` <state>`. With nothing to print, `absent` prints in its place, or nothing.
- **D6. The thread (N2–N5).** A match's segment is its step's label and a space, the entity's name (`Node::name`),
  then its annotations. When every step has at most one match, the thread is one line: the start's segment and each
  step's, joined by ` → `; a step with no match under a matched `from` prints `no <label or name>`. Otherwise it is a
  tree, two spaces per `from` hop, each line `→ <segment>`. A match is *kept* when every unskipped step that goes on
  from its step has a kept match under it; the thread is *complete* when the start is kept. By default only a
  complete thread prints, over its kept matches (dead-end branches left out); an incomplete one prints
  `<start>: incomplete thread (no <step>, …); pass --partial to print it`. `--partial` prints every match with its
  `no <step>` markers. The summary (`thread: …`) counts a step's matches. A skipped step is an **I210** notice on the
  trace, beside its document, never in it (ADR 0130).
- **D7. Surfaces (N1).** The CLI's `trace` takes `--route`, `--reach` and `--partial`. Absent `--format`, a route
  trace prints its human thread and every other trace json; a refusal is decided before any outcome and keeps json.
  This amends ADR 0029 D5b for route traces only. MCP's `specforge.trace` takes `route` and `reach` and answers the
  route document, whose `text` is the `--partial` rendering and whose `complete` and `missing` say what the default
  prints. The LSP shows the summary as a code lens on each routed entity and the thread in its hover, both from one
  ops call.
- **D8. What stays.** The generic reach (ADR 0174), `trace::affected` and the trace prompt read reach, not a thread. A
  project whose extensions declare no route traces byte for byte as before. `specforge schema` does not list routes:
  the outline does.

## Consequences

- An extension says what tracing its kind means; plan 04's `@gsd/core` declares the requirement thread.
- A typo in a route is caught at load (E118), on every project that loads the extension, not at the first trace.
- A script that runs `specforge trace <requirement>` without `--format json` gets text once `@gsd/core` declares its
  route. VS Code passes `--format json`.
- A lens request evaluates a route per routed entity in the file, over one shared referrer index per request.

## Rejected

- **A bespoke `{field, direction, kind}` edge list.** A second path language with a second checker.
- **A strict chain of steps.** No edge joins a commit to a test; the thread needs side branches from the start.
- **A gsd-only `specforge gsd trace` over a shared module** (U1's alternative). Every domain would write its own.
- **A route beside the reach.** It doubles the document and makes the thread one member among many; `--reach` keeps
  the reach one flag away.
- **A kernel-computed "thread state" field.** The view's judgements already carry it.
