# A risk is rated 1 to 10 and an invariant is judged to hold

**Status:** accepted (2026-11-14). Builds on [ADR 0285](0285-a-fields-type-may-be-a-list-of-words-a-block-a-union-or-a-range.md)
(a bounded int, a block, a reference union), [ADR 0287](0287-who-writes-a-field-is-declared.md) (a computed field),
[ADR 0288](0288-an-extensions-settings-are-typed-and-set-in-specforge-json.md) (a typed setting) and
[ADR 0354](0354-a-reference-field-that-names-its-reverse-is-its-own-relation.md) (a field that names its reverse). No
kernel, SDK or protocol change. This record holds the risk's decisions; the invariant's are appended when that kind
lands, so the title names both.

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

## Decisions

- **D1. The kind `risk`, in `@gsd/core`** ([spec/governance.md](../../gsd/core/spec/governance.md)), beyond parity and
  unscoped: a risk outlives a phase, and moving it to another milestone does not rename it. Its writers are the planner
  and the executor, whose tasks and truths control it, so it is a core kind.
  - Authored: `cause` and `effect` (required); `severity`, `occurrence` and `detection` (required, each an int from 1
    to 10); `post`, a block of the same three, none required; `mitigated_by` (→* task | truth); `mitigation` (prose);
    `affects` (→* requirement) and `milestone` (→ milestone), each read back as `risks`; `tags`, `description` and
    `refs`.
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
    that no recorded test contradicts. A truth no verification assesses is not verified.
  - A risk is unscoped, so it names a mitigation by its full path, rooted or not. A name that does not resolve is
    never done, even when another item's path ends with it. The prose `mitigation` is no evidence.
  - A mitigation is read by its id: an id names one entity whatever its kind
    ([ADR 0284](0284-a-name-is-unique-within-its-scope.md) D3: a declaration of another kind at the same path is W060
    and is not placed).
- **D4. Blocking and the setting.** `blocking` is true while the risk's current rpn is at or above
  `specforge.risk_block_on`, an integer setting from 1 to 1000, 100 by default, that no profile presets. The current
  rpn is `rpn` until the risk is mitigated and `post_rpn` after: a mitigated risk whose residual is still at or above
  the threshold keeps blocking, and an optimistic post block lowers nothing before its mitigations are done.
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
  - `invariant` → `mitigated_by`, once an invariant is one of its kinds: the invariant a failure mode names is the
    control whose holding mitigates the risk, so the link has one home and no second field holds it.
  - `threatens_features` and `affected_behaviors` → `affects`, through the requirement each converts to.
  - The old `risk` string is computed now (`rpn`, `status`).

## Rejected

- An authored rpn checked by a rule: the authored values already contradict each other; a computed one cannot.
- Words as ratings: no arithmetic.
- AIAG & VDA's Action Priority in place of the rpn: the old model ranks by rpn. An `action_priority` computed from the
  same three ratings can be added later without changing authored data.
- A profile preset of the threshold.
- A separate check pass for `gsd/W532` (D5).
- A separate `invariants` field for the invariants a risk would break: the old link has one home, `mitigated_by`, and
  two fields would hold the same value twice.

## Would reopen it

- A project that needs Action Priority beside, or in place of, the rpn.
- A project that needs a threshold per milestone.
