# A context pack serves its core whole, and a route's budget is its enrich allowance

**Status:** accepted (2026-11-22). **Amends** [ADR 0356](0356-an-extension-declares-the-context-a-role-reads.md) D2 (a
budget of 0 is E123 only for a route with an enrich section), D4 (the floor is never refused by a read that states no
cap), D5 (`budget.budget_tokens` is the cap the fit used, and a pack shows no context record), D11 (a record holds the
cap its read stated, and none for a read that stated none), D12 (a record is evaluated again under the budget its read
had) and its Consequences line on a route's budget; amends
[ADR 0371](0371-links-come-from-gsds-fields-and-collected-data.md) D8 (the plan check reads 14 inputs, not
`all_decisions`). Protocol 2.3.0 is unreleased: the `budget` key and the record's `max_tokens` member keep their names
and their places, their meanings are narrowed, no bump.

## Context

- ADR 0356 D4 made a route's budget the size of the whole pack: its floor (the envelope, the subject and every core
  section) and its enrich sections together, and a floor above it E062 naming the route. Its Consequences said a read
  below the core "refuses loudly instead of dropping a task".
- A live run of `@gsd/core`'s plan-phase on a real repository met the refusal on the role that most needs its context.
  The plan checker's route had a fixed budget of 8,000 tokens; its core for the run's plan cost 21,039, so every read
  of `gsd/plan-checker` for that plan was E062 and the checker read the plan by hand. The repository's 100 project
  decisions alone cost about 9,300 tokens, and `all_decisions` served every decision of every phase on top of them.
- The executor's route had 6,000. A plan of that phase's size (3 tasks with long actions, 11 truths, 7 artifacts, 3
  key links and 5 prohibitions) has a core of about 6,040 tokens, so `gsd start-plan` refused to record its pack: the
  workflow could not start the plan at all.
- The parity fixtures every test ran on are 15 to 50 times smaller than a real phase, so no test met either refusal.
- A core section is, by ADR 0356's own definition, what the role cannot do without. A fixed number cannot know how
  large a project's decisions or a plan's body will be; refusing the read leaves the role with nothing, which is worse
  than any cut.

## Decisions

- **D1 The core is always served.** A read that states no cap serves the pack's floor whole, whatever it costs. No
  surface refuses it: `specforge context`, `specforge.context`, `specforge://context/{+entity_id}?role=` and a
  `record_context` without `max_tokens`.
- **D2 A route's budget is its enrich allowance.** The tokens of enrich sections a pack may add beyond its floor: the
  fit's cap is the floor plus the allowance, and the enrich sections fill it in declared order as ADR 0356 D4 says.
  The estimate counts each alphanumeric run as one token, so the cap a pack renders costs the same whatever its value,
  and the floor does not depend on it.
- **D3 E062 only under a stated cap.** `--max-tokens`, the MCP `max_tokens` and `record_context`'s `max_tokens` cap
  the whole pack, as before; a floor above a stated cap is E062 naming the route, the cap and the floor, and says that
  a read without a cap serves the core whole.
- **D4 A route's budget is positive exactly when it has an enrich section.** A route of core sections only has
  nothing to fill beyond its core, so its budget is 0; a positive one would be an allowance nothing reads, and is E123
  ("a context route of core sections only allows nothing beyond them"), as a budget of 0 on a route with an enrich
  section is ("a context route with an enrich section allows it a positive number of tokens"). The agent section of a
  route of core sections only says every section is served whole and no `max_tokens` shrinks it; every agent section
  says that `budget.estimated_tokens` is what the pack costs and that a pack too large to read is read with
  `--format brief` (each entity's id, kind and title), the entities it needs then queried.
- **D5 What a pack renders.** `budget.budget_tokens` is the cap the fit used: the stated one, or the floor and the
  allowance. Read again with `--max-tokens <budget_tokens>`, the pack is the same byte for byte, so ADR 0356 D5's
  recompute line holds with the rendered value. A pack shows no field a context record gives: no digest reads one
  (ADR 0356 D9), so a pack shows none either, and what a pack costs does not move when it is recorded.
- **D6 Records and staleness.** A record keeps the budget its read had, in the one meaning `max_tokens` has on every
  surface: the cap a caller states. A `record_context` that states `max_tokens` writes it in the record; one that
  states none writes none. ADR 0356 D12 evaluates a record again under that budget: its cap when it holds one, else
  the core whole and the route's allowance as the route declares it now. A moved digest, no subject or several
  subjects is the route's stale code, as before, and so is a core grown above a cap the record holds; a record that
  holds no cap is never stale for its size. A pack shows what no digest reads (a subject's computed fields: a phase's
  critical path once it has plans, its completion date once it completes), so a core grows without anything the record
  read having moved; held to the cost its pack had when it was recorded, every record of a route of core sections only
  went stale the moment its phase gained a plan. The cap a pack was fitted to stays in the pack, as
  `budget.budget_tokens`: a reader who wants that pack again states it.

### `@gsd/core`'s side

- The routes with enrich sections keep their numbers as allowances: the executor 6,000, the others as declared. The
  four routes of core sections only (`plan-checker`, `integration-checker`, `phase-context`, `quick-context`) declare
  0.
- The plan check stops reading `all_decisions`. It reads `phase_decisions` (its phase's), `quick_decisions` (a quick
  plan's own, `self<decision.plan`), `project_decisions` (`decision[origin=project]`), `resolved_decisions` (the
  decisions that resolve its phase research's open questions, a decision of another phase included) and
  `settled_decisions` (the decisions a task of the plan names in `reversibility.settled_by`,
  `self<task.plan.reversibility.settled_by`, which settle its one-way doors): 14 inputs. These are the decisions the
  project's structure ties to the plan; a decision of another phase that none of them names moves no plan check of this
  phase. A decision the plan only cites by path in its text (`passes::citations`, which E312 reads to excuse a scope
  cut) is not one of them: a citation in prose is no reference a selector can follow, and a plan that depends on a
  decision of another phase names it in `settled_by` or answers its research's question with it. The
  `gsd/plan-checker` route follows the judged table.

## Rejected

- **Raising the constants.** Every number is a guess a larger project passes; the refusal comes back at the next size.
- **A per-project budget setting.** It moves the guess to the user, who learns it from an E062 after the role has
  already failed to start.
- **Cutting core sections to fit.** Makes "core" mean "enrich first": the role silently loses what it was declared
  never to lose. ADR 0356 D4's refusal was right to forbid it; the fix is not refusing when nobody asked for a cap.
- **A record holding the cap its pack was fitted to.** It reads as reproducible, but it holds the record to the cost
  its pack had, which grows with what no digest reads (D6): a route of core sections only reported every record stale
  once the phase gained a plan. Choosing the budget by the route's shape (no cap for a route of core sections only)
  fixes that case alone: on a route with an enrich section, a core grown under the old cap leaves less room for the
  enrich sections, a member drops, and the digest moves although nothing the record read did. It also gave
  `max_tokens` a second meaning beside the cap a caller states.
- **Keeping the budget as a total and ignoring it when the floor is above it.** Two meanings in one number, and an
  enrich section never served on exactly the large projects where it would help.

## Consequences

- A role always starts with its core. The host does not bound a core: a pack can be large on a large project, and
  keeping a route's core selectors as narrow as its role needs is the route author's work (as the plan check's
  decisions were narrowed here). A pack reports what it cost (`budget.estimated_tokens`, which for a route of core
  sections only is its core), an E062 under a stated cap names the core's cost, and an agent reads a pack too large
  for it with `--format brief`. No separate core-size figure or doctor finding is added: the estimate already is one,
  and a threshold would be the fixed guess this ADR rejects.
- A cap is a reader's choice, stated per read, and the only thing E062 is about.
- A route's declared number states what its role may read beyond what it needs, not what the whole pack costs.
- A record that states no cap goes stale only when what it read moved, never because the project grew around it.

## Tests that hold it

- The emitter's `pack` tests: an allowance serves the floor whole whatever it costs and enrich up to the allowance
  beyond it, and serves what a cap of the floor and the allowance serves; a cap below the floor is E062 naming the
  route.
- The registry's `contexts` tests: a route of core sections only is registered with no allowance; one with an enrich
  section and none is E123.
- The registry's `contexts` tests also refuse a positive budget on a route of core sections only (E123).
- The ops `context_routes` and `context_records` tests: a read that states no cap serves the core whole however far
  past the allowance, and reads the same pack again under the cap it renders; a recorded default pack holds no cap and
  costs what the unrecorded one did; a stated cap is recorded. The project `context_records` tests: a record that holds
  no cap is read again under the route's allowance and is stale only when its pack moved; a core above a held cap is
  stale.
- `@gsd/core`'s `cmd_digest` test: a research and a spec recorded with no cap stay fresh (no `gsd/W406`) after their
  phase gains two plans in a strict wave, which makes its pack cost more and moves no digest.
- `@gsd/core`'s real-size fixture (`gsd/core/tests/projects/real-size`, the 100 project decisions of a real repository
  and one phase planned at the live run's size): every route serves its core whole within its allowance on it and on
  each parity fixture, its cap the floor that an E062 under a cap of one token names plus its allowance; the plan
  checker serves a core above 8,000 tokens, its digests the plan check's inputs, and a cap of 8,000 is E062; the plan
  check reads the 100 project decisions and the two its research resolves, and an edit of an unrelated phase's
  decision moves none of its input digests until a task names it in `settled_by`, when only `settled_decisions` moves;
  `gsd start-plan` records a real-size plan whose executor core is above 6,000 tokens, and only a stated cap below it
  is refused.

## What would reopen it

A tokenizer the host can count with, so a cap could match a model's window; a role whose core is too large to serve
on real projects, which would be a route to split, not a budget to raise.
