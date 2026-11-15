# A risk is rated 1 to 10 and an invariant is judged to hold

**Status:** accepted (2026-11-14). Builds on [ADR 0285](0285-a-fields-type-may-be-a-list-of-words-a-block-a-union-or-a-range.md)
(a bounded int, a block, a reference union), [ADR 0287](0287-who-writes-a-field-is-declared.md) (a computed field),
[ADR 0288](0288-an-extensions-settings-are-typed-and-set-in-specforge-json.md) (a typed setting) and
[ADR 0354](0354-a-reference-field-that-names-its-reverse-is-its-own-relation.md) (a field that names its reverse).
D1 to D8 hold the risk's decisions; D9 to D15, the invariant's, were added when that kind landed (2026-11-15), so the
title names both. No SDK or protocol change of its own: D12 waits for an invariant in force with a judged target's
`judged_while` ([ADR 0362](0362-a-judged-target-may-wait-for-its-owner-state.md)), a protocol and SDK addition that
re-vendored the builtins. One host change, D15 (the registry, `crates/pipeline/registry`; no kernel crate, no
re-vendor), amends [ADR 0331](0331-the-gsd-requirement-thread.md) D7: the registry's check of a judged rule's or a
context route's `file()` selector. D7 amended by
[ADR 0359](0359-requirement-and-decision-fields-absorb-feature-constraint-and-decision.md) (`mitigated_by` reads back as
`mitigates` on task, truth and invariant, now that a union's named reverse is read on each of its kinds).

## Context

- `@gsd/core` modelled no risk: GSD keeps no risk file.
- The old `@specforge/governance` `failure_mode` rates severity, occurrence and detection as words (critical … low,
  certain … rare, certain … undetectable) and takes `rpn` as an authored integer that nothing checks. A failure mode
  rated low / rare / certain with `rpn 1000` and a `post_mitigation` of `rpn -5` checks with 0 errors, 0 warnings.
- SpecForge's own spec writes 35 failure modes, every one with its three words and an rpn, and the authored rpns
  contradict each other: the same three words carry two different rpns six times. Critical / unlikely / undetectable is
  80 on `silent_deadlock_miss` and 96 on `false_negative_condition_check`. No word → number scale reproduces them.
- The self-spec's invariant `rpn_arithmetic_integrity` promises that a mismatched rpn is an error. No rule emits one.
- A failure mode's other facts are prose (cause, effect, mitigation), its post-mitigation words, the invariant it
  threatens, the features and behaviors it affects, a description and references.
- `@gsd/core` modelled no invariant either: GSD has none. The old `@specforge/governance` `invariant` states a
  `guarantee` graded by a `risk` word, and nothing says whether it holds: W003 reports only one nothing references.
- A judged rule's `file()` input, or a context section's, whose chain reaches a kind only an unloaded optional peer
  declares dropped the rule (W112) or refused the route (E123), where the same chain as an entity or `test()` input is
  inert ([ADR 0331](0331-the-gsd-requirement-thread.md) D7).

## Decisions

- **D1. The kind `risk`, in `@gsd/core`** ([spec/governance.md](../../gsd/core/spec/governance.md)), beyond parity and
  unscoped: a risk outlives a phase, and moving it to another milestone does not rename it. Its writers are the planner
  and the executor, whose tasks and truths control it, so it is a core kind.
  - Authored: `cause` and `effect` (required); `severity`, `occurrence` and `detection` (required, each an int from 1
    to 10); `post`, a block of the same three, none required; `mitigated_by` (→* task | truth | invariant, D14);
    `mitigation` (prose); `affects` (→* requirement) and `milestone` (→ milestone), each read back as `risks`;
    `tags`, `description` and `refs`.
  - Computed: `rpn`, `post_rpn`, `status` and `blocking`.
  - All three ratings are required: an rpn needs all three.
- **D2. `rpn` and `post_rpn` are computed.** `rpn` is severity × occurrence × detection, 1 to 1000; `post_rpn` is the
  same product over the post ratings, a member left out keeping its pre rating, and equals `rpn` without a post block.
  A rating outside 1 to 10, in the post block too, is the kernel's E061, and a written rpn is E082: no rule compares an
  authored rpn, as there is none. A risk with a rating missing or out of range gets no computed value.
- **D3. Status.** `open` while `mitigated_by` names nothing, `mitigated` once every item it names is done, `mitigating`
  between.
  - A task is done when a recorded commit names it or its plan is complete (the rule that gives the task its `done`).
  - A truth is done when its phase's verification verifies it: an override, or a fresh passing `gsd/E501` verdict
    that no recorded test contradicts. A phase's verifications are read as one, so an override on any of them counts.
    A truth no verification assesses is not verified.
  - A risk is unscoped, so it names a mitigation by its full path, rooted or not. A name that does not resolve is
    never done, even when another item's path ends with it. The prose `mitigation` is no evidence.
  - A mitigation is read by its id: an id names one entity whatever its kind
    ([ADR 0284](0284-a-name-is-unique-within-its-scope.md) D3: a declaration of another kind at the same path is W060
    and is not placed).
- **D4. Blocking and the setting.** `blocking` is true while the risk's current rpn is at or above
  `specforge.risk_block_on`, an integer setting from 1 to 1000, 100 by default, that no profile presets. The current
  rpn is `rpn` until the risk is mitigated and `post_rpn` after: a mitigated risk whose residual is still at or above
  the threshold keeps blocking, and an optimistic post block lowers nothing before its mitigations are done. Without a
  post block `post_rpn` is the `rpn`, so a risk whose every mitigation is done keeps blocking until its `post` ratings
  are written: no number is guessed, no diagnostic is added, and `status` `mitigated` beside `blocking` `true` says
  it, as the field's description does.
- **D5. `gsd/W532`** warns on a risk whose `rpn` is at or above the threshold and that names nothing in `mitigated_by`,
  naming its rpn and the threshold. It is a warning under every profile: it is no GSD rule, and `gsd-strict` raises
  only what GSD warns about. The derive pass that rates the risk emits it, as it emits `gsd/W409`: one module holds
  the product, the threshold and the finding, and no second pass reads a computed value back.
- **D6. `risk_level` on phase and milestone.** A risk reaches a phase when a task or a truth its `mitigated_by` names
  belongs to a plan of the phase (a quick plan is in no phase). A phase's `risk_level` is the highest current rpn of
  the risks that reach it, so it drops as their mitigations land. A milestone's is the highest of its phases' levels
  and of the current rpn of each risk whose `milestone` names it. Each is absent when no risk reaches it.
- **D7. No reverse on `mitigated_by`.** The kernel reads a field's `inverse_of` through its one target kind, so a
  reverse named on a union would never be read. `affects` and `milestone` name `risks`. When the kernel lets a union
  field name its reverse, this decision is amended.
- **D8. The conversion's word scale and field map.** Each old word sits in the 1–10 band the FMEA scales give it;
  detection runs the other way (10 cannot be caught), and `low` is 2 because 1 means "no effect", which no old word
  says.

  | Old word | severity | occurrence | detection |
  |---|---|---|---|
  | critical | 10 | | |
  | high | 7 | | |
  | medium | 5 | | |
  | low | 2 | | |
  | certain | | 10 | 1 |
  | likely | | 7 | 3 |
  | occasional | | 5 | |
  | moderate | | | 5 |
  | unlikely | | 3 | 7 |
  | rare | | 1 | |
  | undetectable | | | 10 |

  - The old top-level `rpn` and `post_mitigation.rpn` are never written: the computed value replaces each, and the
    conversion's ledger records each one that differs.
  - `post_mitigation`'s words and the top-level `post_severity`, `post_occurrence` and `post_detection` become `post`
    members; where both are written and differ, the block's value wins and the other is a ledger row.
  - `cause`, `effect`, `mitigation`, `description` and `refs` keep their names.
  - `invariant` → `mitigated_by` (D14): the invariant a failure mode names is the control whose holding mitigates
    the risk, so the link has one home and no second field holds it.
  - `threatens_features` and `affected_behaviors` → `affects`, through the requirement each converts to.
  - The old `risk` string is computed now (`rpn`, `status`).

- **D9. The kind `invariant`, in `@gsd/core`** ([spec/governance.md](../../gsd/core/spec/governance.md#invariant)),
  beyond parity and unscoped: an invariant outlives the phase it starts at, and its name is a word unique in the
  project (`entity_id_uniqueness`), as the old ids are.
  - Authored: `statement` (required), `description`, `scope` (project, milestone), `holds_from` (→ phase), `proof`
    (→* test) and `items` (→* item), `@code/core`'s facts quoted as a truth's proof and a task's items are,
    `criticality` (critical, high, medium, low), `tags` and `refs`.
  - Computed: `status` and `last_verified` (D11).
  - Each field of `@specforge/governance`'s invariant has a home: `guarantee` → `statement`, `risk` → `criticality`
    (the same four words in the same order, so no scale is needed), `description` → `description`, `refs` → `refs`,
    and `@specforge/testing`'s `verify` → `proof`. Its presentation (icon, colours, inference guide) has none: no
    `@gsd/core` kind declares one. Loading both extensions is E026, as for `risk`.
- **D10. In force from a complete phase, for good.** An invariant with no `holds_from` is in force from the start;
  with one, once that phase has been complete in the phase's own sense, `executed`, `needs_review` or `complete`: the
  values of the phase's `complete_when`, one constant (`roadmap::PHASE_COMPLETE`) for both kinds. Executed is enough;
  verification is not awaited. A `holds_from` that resolves to no phase (the kernel reports it) keeps the invariant
  pending.
  - In force is reached once and stays. GSD's gap closure adds a plan to a phase whose verification found gaps, which
    moves the phase back to `in_progress`; the invariant stays in force, so its verdict still shows, the risks it
    mitigates still count it (D14) and its proof is still checked, when a violation matters most.
  - The status is computed and keeps no history, so the latch is read from what outlives the slide: the phase is
    complete now, or it has a recorded verification, which is written only once the phase ran and which gap closure
    keeps. A phase that gains a plan before any verification is still being executed, and its invariant waits.
  While an invariant is not in force its `status` is `pending`, and its own `complete_when` holds on every other
  status, so the kernel treats it as pending ([ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) D4, D5):
  its `proof` and `items` are not checked, and a test or an item its phase has yet to write is no finding. In
  force, a proof naming a test the served store lacks is `code/E101`.
- **D11. `status` and `last_verified`.** In force, `status` reads the invariant's verdict: `unverified` with none
  (missing, unrecorded or unreadable too), `stale` once an input moved since, else the fresh verdict's outcome,
  `holds`, `violated` or `uncertain`. No outcome field repeats it. `last_verified` is the `at` of the verdict while
  `status` is `holds`, absent otherwise: a time, as the verdict gives one; a phase view of it is another field's.
- **D12. `gsd/I536`, the verdict that an invariant holds.** A judged rule over every invariant in force
  ([ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md)), at Info under every profile: a judged rule
  reports on every target without a fresh passing verdict, and a converted project has one invariant per old one
  (149 in SpecForge's own spec), so a Warning would bury every other finding. A fresh `violated` stays Info; it shows
  in `status` and in the risks the invariant mitigates (D14).
  - A pending invariant owes no verdict. The rule's target waits for the invariant's owner state
    (`judged_while: complete`, [ADR 0362](0362-a-judged-target-may-wait-for-its-owner-state.md)): its `complete_when`
    holds on every status but `pending` (D10), read on the merged snapshot after the compute passes. So the rule, its
    prompt and the gates agree: a pending invariant is not reported, lists no judgement a gate reads, and the prompt
    does not list it; with only pending invariants it says so. A verdict recorded early (`--invariant <id>`) is kept
    and read once the invariant is in force.
  - The judge is the prompt `verify_invariant`, our own text (GSD has no invariant), rendered for each host, with no
    slash command: no upstream command row exists for one, as none exists for `execute_plan`. It reads the inputs,
    reads the code the items name, judges, and records with `specforge gsd record-verdict <invariant> I536 …`.
  - Two inputs: the invariant itself, and its tests, `test(self.proof, self<test.names)` (its proof's results and the
    tests whose name tags it, [ADR 0331](0331-the-gsd-requirement-thread.md) D7). A new result of one stales the
    verdict; on a fresh clone, with no test results, the input is unjudged and the verdict stays fresh. No input reads
    the items or a `file()`: an item reference selects no entity on a clone without a fact store, so a verdict
    recorded on one machine would be stale on the next. The judge cites each source file it read (`--cited`), which
    stales the verdict when one is edited, and a clone has those files.
  - Under `gsd-plus` a stale I536 on an invariant in force, as any stale verdict, refuses a scoped gate (gsd/E516).
- **D13. `scope` bounds where it reaches, not its status.** `scope` changes no `status`. It decides which phases a
  check of an invariant's code reads, when one is built: with no `holds_from`, every phase; else the `holds_from`
  phase, the later-numbered phases of its milestone and, where numbers do not order two phases (no roadmap order
  runs across milestones), any phase with a recorded commit no older than the first commit recorded for the
  `holds_from` phase. `scope milestone` keeps only the `holds_from` phase's milestone; with no `holds_from` it has
  no milestone and reads as `project`. That shape is accepted, not refused.
- **D14. An invariant that holds mitigates a risk.** `mitigated_by` (D1, D3) may name an invariant beside a task and a
  truth; it is done only while the invariant's `status` is `holds`. Violated, stale, uncertain, unverified and
  pending are not done, so the risk stays `mitigating` and blocks on its rpn until the invariant holds again. An
  invariant belongs to no plan, so it reaches no phase's `risk_level` (D6). This is the failure mode's `invariant`
  link of D8: the invariant a failure mode threatens is the control whose holding mitigates the risk.
- **D15. A `file()` through an unloaded optional peer's kind is inert** (amends ADR 0331 D7). A judged input or a
  context section whose `file()` chain reaches only a kind an unloaded optional peer declares selects nothing: the
  rule is registered, and the context section is skipped and listed, as an entity or `test()` chain already is. Any
  other unloaded kind stays W112 for a rule and E123 for a context route. Before, the `file()` form dropped the rule
  (W112, and E118 on each route that annotated it).

## Rejected

- An authored rpn checked by a rule: the authored values already contradict each other; a computed one cannot.
- Words as ratings: no arithmetic.
- AIAG & VDA's Action Priority in place of the rpn: the old model ranks by rpn. An `action_priority` computed from the
  same three ratings can be added later without changing authored data.
- A profile preset of the threshold.
- A separate check pass for `gsd/W532` (D5).
- A separate `invariants` field for the invariants a risk would break: the old link has one home, `mitigated_by`, and
  two fields would hold the same value twice.
- `gsd/I536` as a Warning: one per invariant without a fresh `holds`, on every converted invariant, buries every other
  finding (D12).
- An authored outcome or `status` on an invariant: the verdict holds it, and two homes would disagree.
- A verdict input over `items`, or a `file()` of them: an item reference selects nothing on a clone without a fact
  store, so the verdict would be stale on every other machine (D12). The judge's `--cited` files stale it instead.
- A slash command for `verify_invariant`: no upstream command row exists for one, and a host name of ours would be
  the only one GSD does not have.
- `scope` deciding `status`: a verdict judges the code as it is, wherever it reaches (D13).

## Would reopen it

- A project that needs Action Priority beside, or in place of, the rpn.
- A project that needs a threshold per milestone.
- A fact store on every clone: a verdict could then read an invariant's items (D12).
- A check of an invariant's code across the phases it reaches: D13's window is its scope.
