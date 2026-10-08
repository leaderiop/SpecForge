# A feature's status is declared; its evidence is derived

**Status:** accepted (2026-10-07)

`specforge product milestone-completion` reported 0% for every one of the fifteen completed
milestones: of 133 features, none was `done` (128 had no `status` at all), while each milestone's
`status completed` was set by hand. Nothing checked one claim against the other, and nothing
compared either with what the project's tests prove. The same gap hid unbuilt work inside completed
milestones: `wasm_runtime` listed the planned host-function API and claimed "All 8 host functions
work correctly" though the component runtime imports none, and `extension_ecosystem` listed features
of three extensions (compliance, embeddings, markdown-renderer) that have no source. The product
extension also specified three checks across entities (I063, I064, I065) that nothing implemented,
because its declarative rules check one entity at a time.

## Decisions

- **D1. Status and evidence are two facts.** A feature's `status` is its author's delivery claim and
  stays the input of every status query (`done_count`, `completion_ratio`, burndown, health). Its
  **evidence** is derived from the recorded test report and is reported beside it, never instead
  of it: a feature can be delivered and under-tested, and the report says so rather than
  rewriting the claim.
- **D2. A feature is proven when its implementers are.** The behaviors that implement a feature are
  the `behavior` entities whose `features` field names it. A feature is proven when at least one
  behavior implements it and the coverage rule counts every one proven (ADR 0004 D2-a: at least
  one obligation, every one named by a passing test, no failing test). No implementer, no proof.
- **D2a. Evidence is the one product reader of a foreign edge.** Product's queries traverse only its
  20 edge types (`pe_cross_extension_query_boundary`). Delivery evidence reads
  `BehaviorImplementsFeature` (software's `features` field on a behavior), because the implementers are
  what the tests prove; its fields are additive and absent without recorded evidence, and every
  status result stays identical with and without @specforge/software. The boundary's contract says so.
- **D3. The host scores, the extension aggregates.** The host knows no product kind (zero-entity
  core), so it passes a command each entity's score: `CommandInput.evidence` is `none` without a
  report, `unreadable` with the reason when the report cannot be used, else
  `recorded { entities }`: obligations, proven obligations and failing tests of every entity that
  counts toward coverage, from `ProjectView::coverage` (computed by `specforge_ops::command::run`, the
  numbers `specforge stats` reports). The field is optional on the wire, so the protocol stays 1.1.0:
  an older guest ignores it, a newer guest reads its absence as `none`. An analyze pass, which
  receives the recorded test results, scores with the same crate (`specforge-coverage`).
- **D4. Claims across entities are checked by passes.** `@specforge/product` declares two compiler
  passes: `lifecycle` in the check phase (every compile, watch, the LSP, MCP) reports what the
  specs declare against each other: W154 (a completed milestone delivers a feature neither `done`
  nor `deprecated`), I063, I064 and I065; `delivery_evidence` under `specforge analyze` reports I071
  (a `done` feature the recorded tests do not prove) and a summary comparing declared and proven
  counts. W154 is a warning because both sides are declarations and one is false; I071 is
  information because evidence lags delivery.
- **D5. `milestone-completion` reports both.** Its payload keeps `done_count` and
  `completion_ratio` and adds `evidence` (the state), `proven_count`, `proven_ratio`,
  `proven_features` and each feature's evidence (`feature_evidence`); its human layout prints an
  `Evidence:` line and each feature's proven behaviors and obligations.
- **D6. The spec was made true, not the check quieted.** The 74 delivered features of the
  completed milestones were marked `done`. The six that were never built (`product_graph_diff`,
  `fa_progressive_warnings`, and the four of the compliance, embeddings and markdown-renderer
  extensions) moved to new `planned` milestones (`ms_followups`, `extension_catalog`), as the
  host-function API moved to `wasm_host_functions`; two exit criteria of `extension_ecosystem` that
  claimed what does not exist (npm, OCI and GitHub registry sources; token refresh) were rewritten
  to what does.

## Consequences

- `specforge check .` reports a completed milestone that lists unfinished work, so a phase cannot
  be closed by editing its status alone.
- With a fresh `specforge collect`, this repo's completed milestones are 100% done and between 0%
  and 100% proven (formatting 2/2; validation and errors 0/4): the gap is the test-linking work
  left, by feature.
- The milestone payload grows optional keys; consumers that pinned its exact key set see
  `evidence` (present whenever the command ran).

## What would reopen this

A feature implemented by something other than behaviors (a deliverable-level test, a formal proof
of a feature), or a project that wants `status` itself derived from evidence: then D1's separation
would become a policy option, not a rule.
