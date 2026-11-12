# A pass reads the facts it declares, and perf is pinned by counted work

**Status:** accepted (2026-11-10, R2 of the Beyond Parity build plan). **Amends**
[ADR 0337](0337-the-ports-protocol-is-2-0-0-inputs-are-one-list-and-effects-are-journaled.md) D2 (the `facts` input is
served to a pass too), [ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) (a pass reads
facts of the kinds it declares, not every fact),
[ADR 0346](0346-a-fact-says-where-it-is-written-and-what-it-names.md) D4 (`dangling` rides on the facts a pass is
sent), [ADR 0347](0347-item-and-package-facts-and-ids-a-scip-symbol-maps-into.md) D9 and D11 (`@code/core`'s `refs`
judges what a spec writes, `proof` what the facts say) and [ADR 0019](0019-one-entity-snapshot-per-compile.md) (the
pass input's entities). Builds on
[ADR 0345](0345-the-syntax-import-and-collectors-that-read-answer-and-are-served-facts.md) (the `facts { kinds }`
input, the syntax cache and its `SyntaxWork`). Protocol 2.0.0, unreleased: the change goes into its 2.0.0 row, no
bump. Codes W200 and I214.

## Context

SpecForge's own code is facts now: about 37k `item` facts beside 2,272 authored entities. Measured on a release build
(fat LTO, line tables; `/usr/bin/sample` at 1 ms over five runs), `specforge check` with item-shaped facts took 3.1 s of
CPU against 0.5 s for the same project with none:

| Share of samples | 200k facts | 37k item facts |
|---|---|---|
| Check passes inside guests, decoding their input | 51 % | 47 % |
| Graph build (`settle`, `link`, `declare`) | 19 % | 11 % |
| `EntitySnapshot::of` | 14 % | 15 % |
| Registry checks (custom verdicts, unknown fields) | about 3 % | 10 % |
| Serving the stores | 2 % | 1 % |

- **Every pass was sent every fact.** `EntitySnapshot::pass_entities()` held every entity, facts included, and was the
  one pass payload: each check pass decoded all of it in its guest, whether it read a fact or not.
- **A validation rule that names no kind ran on every fact**, so a custom rule called its guest once per fact.
- **Nothing counted a collect's work.** The syntax cache's hits and misses were tallied per call and never shown, so
  a cache that could not be written made every collect parse cold, silently.

A test may not time anything: a wall-clock assertion fails on a loaded machine and passes on a fast one.

## Decisions

- **D1. A pass may declare the `facts` input.** `served(Pass, Facts)` holds for a pass of every phase (check,
  compute, guard, analyze), as it does for a collector. A kind no loaded extension declares collector-written is W021,
  as for a collector; a kind an unloaded optional peer declares is inert.
- **D2. A pass is sent the authored entities and the facts of the kinds its `facts` input names, nothing more.** A
  pass that declares none is sent no fact. Facts are an input like `files` and `tests`: the pass that reads them says
  which. `PassInput.fact_stores` is unchanged (every served store), so a pass still tells "no store holds this kind"
  from "the store holds no such fact" without the facts.
- **D3. Edges and counts.** A pass is sent the edges whose two ends it is sent. `PassEntity`'s edge counts stay the
  graph's: they describe the entity (ADR 0270), not the payload.
  The per-entity coverage view (`EntitySnapshot::coverage_entities`, read by `specforge coverage` and the surfaces'
  coverage answers) is the payload of a pass that declares no fact, so it holds the authored entities only, as the
  `@specforge/testing` coverage pass sees them: a fact owes no verify obligation and is no row of it.
- **D4. One encoding per distinct `(facts, baseline)` set.** Passes that declare the same kinds and the same baseline
  kinds share one encoded payload; a pass with `files` or `tests` inputs has its own, as before. A pass's
  `entities_analyzed` is what it was sent.
- **D5. A validation rule that names no kind judges no fact.** `Rule::applies_to(kind, collected)`: a rule with no
  target applies to every kind no collector writes, a rule with a target to that kind, collected or not. The
  obligation rule (`verify_rule_for`) reads it the same way. The kind decides, never the record's span.
- **D6. `@code/core`'s passes declare what they read.** `refs` judges the references a spec writes to facts and names
  the closest held test or run, so it declares `["test", "test_run"]`; `proof` judges what the facts say: a linked test
  that failed (W401), a `names` entry that is no entity (I402), and a fact's own reference that answers nothing (E101,
  ADR 0346 D4, moved here from `refs`), so it declares all four kinds. Item facts cross into one guest, not two.
  `@gsd/core`'s passes that read the requirement thread (`verify_rules`, `scan`, `gate_e515`, `derive`) declare
  `["test"]`.
- **D7. A collect says its syntax work.** Each collector's result carries `syntax: { parsed, cached, bytes,
  unwritten, unwritable? }`, the reader's own `SyntaxWork` (`RunnerResult.syntax`, absent for a collector that parsed
  nothing; `unwritable` is the first cache write error, when one failed), read from its call's port session after the
  call (`PortCall::take_syntax_work`). **I214** says it in words, once per collector that parsed. **W200** says, once
  per collect, that the syntax cache could not be written: every later collect would parse those files again. The
  collect still stores its facts.
- **D8. Perf is pinned by counted work and measured, never asserted in time.** The pins count: entities and facts a
  pass is sent, the custom-rule calls, files parsed and read from the cache by a second collect and after one edit,
  the items a source collect stores, and the guest fuel it spends under a fixed limit. Time is measured by
  `cargo run -p xtask --bin perf-report` on a release build of SpecForge itself against the round's budget (collect
  cold ≤ 10 s, warm ≤ 2 s; `check` with the facts ≤ 2 s), and recorded by a nightly workflow that is not a check.
- **D9. The blob budget is re-based** after the round's last blob change: the measured total of the builtins,
  rounded up to the next 100,000 B (the rule is unchanged).

## Rejected

- **Every entity to every pass** (today's): half of `check` decoding what most passes never read.
- **The authored entities plus the pass's own extension's collected kinds**: implicit, and `proof` would decode items
  to read tests in a project with no test.
- **A `kinds` input that narrows authored kinds too**: with facts gone, the guests' share over authored entities is
  5 % of `check`.
- **A gate check on time, or a timed test**: runner noise would fail merges; a test counts work instead.
- **Filtering a kind-less rule by the record's span** (a fact store's file): a span is where, not what.

## Consequences

- **Breaking.** A pass is sent no fact unless its `facts` input names the fact's kind, and a validation rule that
  names no kind no longer runs on facts. A third-party pass that read facts without declaring them reads none.
- A pass's `entities_analyzed` counts what it was sent.
- `specforge collect --format json` and MCP `specforge.collect` carry `runners[].syntax`.
