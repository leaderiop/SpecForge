# `@gsd/core` holds exactly gsd-core's kinds and fields

**Status:** accepted (2026-11-16). D8's conversion design decided by
[ADR 0372](0372-the-converter-gives-every-old-kind-one-fate-and-writes-gsds-ids.md). **Supersedes**
[ADR 0357](0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md),
[ADR 0358](0358-a-risk-is-rated-and-an-invariant-is-judged.md) and
[ADR 0359](0359-requirement-and-decision-fields-absorb-feature-constraint-and-decision.md): each of their decisions is
reversed, kept or moot as the table below says, and a kept decision stays in force as it is written there, with the
amendments it made to other ADRs. **Amends**
[ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md) D2, D4, D5, D6, D7, D14 and D16 (the
conversion drops what has no home, and the self-spec sample runs no test until the swap: D8),
[ADR 0364](0364-the-software-kinds-are-dropped-and-a-census-gives-every-old-kind-its-fate.md) (the census rows and the
milestone's fields it left to a later ADR: D3 and D8) and
[ADR 0368](0368-a-bound-is-part-of-a-requirements-text-and-the-prove-pass-and-the-protocol-do-not-change.md) D2 and
D3 (the requirement's `metric` and the fate of an old invariant, which they left to a later ADR: D3 and D8). Closes
the review finding IN-03, which [ADR 0365](0365-a-module-is-dropped-and-no-kind-is-added-to-hold-it.md) left to a later
ADR: ADR 0357 is superseded whole, so its D13 and its three module rows no longer stand. Leaves
[ADR 0367](0367-the-self-spec-sample-retires-at-the-swap-and-absorb-stays-while-the-old-builtins-ship.md) D1 as it is
(the sample retires at the swap). No protocol, SDK or kernel change comes with this decision, and no builtin is
re-vendored: the kernel features of D9 stay. Its rows for ADR 0359 D8 and D9 are re-sourced by
[ADR 0371](0371-links-come-from-gsds-fields-and-collected-data.md).

## Context

**The rule.** On 2026-11-16 the owner set the rule `@gsd/core` is held to: its entities, their fields and their
formats are exactly those gsd-core defines (gsd-core is the upstream definition `@gsd/core` tracks; the build follows
its release 1.19.0). SpecForge adds only computed values, collected facts, graph logic and guards. Anything in the old
spec with no home in GSD is dropped with a recorded reason, and no entity is added to port the old spec. ADR 0364
applied the rule to the four software kinds and ADR 0365 to the module; this ADR applies it to `@gsd/core` itself.

**The rule it replaces.** The R4 round of the Beyond Parity build plan was built on 2026-11-14 under another rule:
the complete solution, in which converting SpecForge's own spec loses no value. Under it, R4 gave `@gsd/core` what the
old spec held and GSD does not:

- five kinds: `persona`, `journey` and `term` (ADR 0357), `risk` and `invariant` (ADR 0358);
- fields on the kinds gsd-core defines: a requirement's quality `category`, `priority`, `metric`, `source`,
  `personas`, `constrains`, `protects`, `tags` and `effort`, with its heading renamed `area` (ADR 0359); a decision's
  `context`, `alternatives`, `consequences`, `decided`, `deciders`, `affects`, `tags` and `supersedes`, with the
  computed `in_force` (ADR 0359); a milestone's `exit_criteria`, `starts`, `targets`, `depends_on`, `owner`,
  `contributors`, `artifact_types` and `tags` (ADR 0357 D8); a UAT test's `journey` (ADR 0357 D22);
- seven codes, `gsd/W530`, `gsd/E531`, `gsd/W532`, `gsd/W534`, `gsd/I536`, `gsd/W538` and `gsd/E537`, and the
  milestone half of `gsd/E102` (ADR 0357 D8);
- the setting `specforge.risk_block_on` (ADR 0358 D4) and the prompt `verify_invariant` (ADR 0358 D12);
- the passes that compute their values and judge them: a risk's rpn, status and level, a journey's coverage, a
  term's uses, the vocabulary and the glossary, an invariant's status and the commits it is owed a judgement for.

Beside R4's additions, other authored values of the kinds that stay are written by no gsd-core text either: a truth's
`criterion` and `proposed_verification`, a UAT test's `criterion` and `from_task`, and the task type `manual`.

**The counts.** At the commit this ADR lands on, `@gsd/core` declares 66 kinds (the `KINDS` list of
`gsd/core/src/kinds/mod.rs`), 77 codes (`gsd/core/src/codes.rs`) and 45 prompts (`gsd/core/README.md`). This ADR
takes them to 61 kinds, 70 codes and 44 prompts.

Upstream facts are cited by their path under `gsd/parity/upstream/v1.19.0/` and their line.

## D1. The rule, and the test that holds it

A kind of `@gsd/core` is one gsd-core defines, and an authored field of it is one gsd-core writes. SpecForge adds
computed values, collected facts, graph logic and guards, and nothing else.

The rule has a test. Every authored field of every kind names, in a document of the model,
`gsd/core/spec/provenance.md`, the upstream file and line it mirrors, or the SpecForge capability that reads it (a
pass, a rule, a route, a command, a guard or a computed value). The test fails on a field that names neither.

## D2. The five kinds leave `@gsd/core`

`persona`, `journey`, `term`, `risk` and `invariant` leave, with their fields, their computed values (`risk_level` on
a phase and a milestone, `journeys_covered` on a milestone, `invariants_due` on a phase), the reverse names they gave
the kinds that stay, their passes, the setting `specforge.risk_block_on`, their codes and the prompt
`verify_invariant`. No kind that stays holds a reference into them.

Where their content lives in GSD:

- **A risk** is a threat of `@gsd/security`'s threat register (`gsd-core/templates/SECURITY.md:25-27`), or a row of
  its Accepted Risks Log (`gsd-core/templates/SECURITY.md:37-39`).
- **An invariant** is a requirement, with a must-not prohibition in the plan that relies on it (a resolved
  prohibition is lifted into `must_haves.prohibitions`, `gsd-core/templates/spec.md:91-94`). "Due again after a
  commit" is a truth's freshness: its verification verdict goes stale when what it judged changes.
- **Personas, journeys and terms** are prose in `PROJECT.md` (`gsd-core/templates/project.md:10`).

## D3. The authored fields gsd-core does not write leave the kinds that stay

- **requirement:** `category` (the quality class), `priority`, `metric`, `source`, `personas`, `constrains`,
  `protects`, `tags` and `effort`.
- **decision:** `context`, `alternatives`, `consequences`, `decided`, `deciders`, `affects` and `tags`.
- **milestone:** `exit_criteria`, `starts`, `targets`, `depends_on`, `owner`, `contributors`, `artifact_types` and
  `tags`. A milestone keeps gsd-core's fields alone: its name, what it delivered, its goal and target features, and
  the computed state, dates and stats. `gsd/E537` (a milestone that targets a day before it starts) goes, and
  `gsd/E102` judges the cycles of phases and plans only.
- **truth:** `criterion`. **UAT test:** `criterion`, `from_task` and `journey`.
- **task:** the type `manual`. gsd-core's task types are `auto`, `tdd` and `tracer` and the checkpoints
  (`gsd-core/references/planner-guidance.md:37-44`, `agents/gsd-planner.md:231-249`,
  `gsd-core/references/checkpoints.md:113`).
- **phase:** `tag`. **methodology:** `status` and `superseded_by`. **lesson:** `milestone`. **disposition:**
  `reviewer`. These five came to light when D1's table was written: no upstream line writes them and no capability
  reads them. Upstream's lessons carry a phase only, and its review ledger has no reviewer column and asks for no
  reviewer enum (`gsd-core/references/planner-reviews.md:78`): who raised a concern stays reachable through the
  disposition's `concern`, whose `reviewers` the review round records.

`pause.reason` and `blocker.plan` came to light with them and stay, because upstream writes both: a pause's reason is
STATE.md's `Stopped at:` (`gsd-core/templates/state.md:98`), and a blocker's plan is the plan of the
`last_gate_trip: {plan_id}/{task_id}` the executor records when a TDD gate trips (`agents/gsd-executor.md:426`).

`requirement.shapes` and `requirement.text_en` stay: upstream's edge probe reads both (`src/edge-probe.cts:59-64`,
`gsd-core/references/edge-probe.md:44-57`, `gsd-core/references/specless-probe-fallback.md:87-88`).

Every asset line that learned a flag for one of these fields carries upstream's text again: the `specforge new
requirement` and `specforge new decision` lines, `/gsd-verify-work` recording a UAT test's journey; the `specforge
new lesson` line of `/gsd-extract-learnings` and the `specforge new disposition` line of the review ledger lose their
`--milestone` and `--reviewer`, with nothing in their place. The port lock,
`gsd/core/tests/port.lock`, records the text.

## D4. A decision supersedes nothing

`decision.supersedes` and the computed `decision.in_force` leave. `gsd/E310`, `gsd/W317` and `gsd/W506` count every
decision of a context, as they did before ADR 0359 D16: no decision is left out of them for having been reversed. A
decision that no longer holds is answered by a new decision of the phase that changes it, as GSD writes one, and
upstream's own way to keep a decision out of tracking stays the author's. The asset lines and the agents' wording
that spoke of a decision "in force" carry upstream's text again.

## D5. The requirement's heading is `category` again

The requirement's heading field takes upstream's name again: `category`, the required string that upstream's
`### [Category]` heading names (`gsd-core/templates/requirements.md:39`), in ids written `[CATEGORY]-[NUMBER]`
(`gsd-core/templates/requirements.md:80`). ADR 0359 D1 renamed it `area` to free the name for a quality class; D3
removes the quality class, so the reason for the rename is gone, and the name has one meaning again.

The kind, its model document, every test project and parity fixture, the conversion's fill and `specforge new
requirement --category` follow. The assets carry upstream's text again where R4 changed it: `[CATEGORY]-[NUMBER]`,
`### [Category 1]` and the roadmapper's "Group by Category".

## D6. The planner writes a truth's verification tier

`truth.proposed_verification` leaves: upstream writes it nowhere. `truth.verification` loses its person-only mark, and
the planner writes it as upstream's planner does: a lifted backstop edge is a truth whose `verification` is
`backstop` (`{ statement, verification: backstop }` in `must_haves.truths`,
`gsd-core/references/specless-probe-fallback.md:124-128`). A person may still change the tier. The verifier abstains
on a `backstop` truth that has no explicit evidence, as upstream's honest-verifier rule says
(`gsd-core/references/honest-verifier.md:42-43`).

The refusal of `/gsd-phase --remove` on a phase whose truth holds a tier goes with the mark: it existed only because
the tier was a person's. This drops a guard, and it is the owner's choice: an agent may now set the tier a person used
to confirm, as upstream's agent does.

## D7. SpecForge's names for items GSD leaves unnamed stay

GSD writes truths, artifacts, key links, prohibitions, gaps and tasks as list items with no id. SpecForge gives each a
name (`TR1`, `AR1`, `KL1`, `PR1`, `VG1`, `T1` and the other prefix names of the model documents): every entity needs
one, and a verdict, an override and a gap-closure plan must point at one. The names are not fields added to port the
old spec. They stay, as the one accepted difference of format between `@gsd/core` and gsd-core.

## D8. The conversion drops what has no home, and the sample runs no test until the swap

This amends ADR 0360.

**What has no home.** A value whose home leaves `@gsd/core` by this ADR, or whose old kind converts to nothing, is
ledgered `dropped`, with its reason, and nothing is written for it. absorb gains no home under this ADR: a value with
a home in GSD's kinds that absorb does not write yet waits for the conversion change. The absorb table gains the
dropped home and its counts move with each removal (ADR 0360 D2); its `Default` home goes with the channel, its one
row. No home writes a reference after this ADR, so reference resolution and its out-of-selection ledgering have
nothing to do until the conversion change adds a home that writes one again (ADR 0360 D4). A risk's rpn and a
decision's `in_force` are no longer compared after the write (ADR 0360 D5), and no `metric` record is written: the
field leaves (ADR 0360 D7, as ADR 0368 D2 already amended it).

**The proofs and the sample.** No kept kind holds a proof, so absorb cites no test: the proof join, the `unproven`
ledger entry and the refusal of a source with no recorded item go (ADR 0360 D6). The self-spec sample then runs no
test until it retires at the swap (ADR 0367 D1, unchanged): its bound no longer asks that absorb cite a test (ADR
0360 D14), and its goldens are re-blessed as the conversion drops values (ADR 0360 D16).

This weakens the check, and the owner chose it so. The half that runs the cited tests can no longer fail: from this
change to the swap it runs none. What the check still proves: the sample of the old spec converts and checks with no
error and no warning, and its infos, ledger and selection equal their goldens. The tests absorb cited (41 when ADR
0360 measured the sample) are workspace tests, and they still run in the `tests` check whenever a change reaches them;
only the sample's own run of them stops. Nothing is added that a later change must remove.

**The fates ADR 0364 and ADR 0368 left to a later ADR.**

- An old invariant becomes a requirement: its guarantee goes into the requirement's text, and its `verify` lines and
  its formal claim are dropped (ADR 0364's census, ADR 0368 D3).
- An old persona, journey, term, failure mode and channel are dropped (ADR 0364's census).
- An old decision converts while it is still in force; one that no longer holds is dropped, and `docs/adr/` stays its
  record (ADR 0364's census).
- A milestone keeps gsd-core's fields alone (D3), so a release's values that no such field takes are dropped with
  their reason (ADR 0364 D3).
- `requirement.metric` leaves (D3), and a bound stays in the requirement's text (ADR 0368 D2).

How each is written is the conversion change's design. Until that change, absorb ledgers an old invariant's values
`dropped` with the reason that the conversion change writes them as a requirement. What becomes of the test
attributes that name a software kind (ADR 0364's Consequences) is not decided here.

**Where the removed code is kept.** Reference resolution, its out-of-selection ledgering, the `gsd/E537` pre-check
and the proof module (with the proof join and the refusal of a source with no recorded item) were removed from absorb
by the change that made it drop these values. The last commit that holds them is
`2183c2a9dbfe12174fd31ab6f11491e6df153367`, where the conversion change can take them back
(`xtask/src/absorb/plan.rs`, `xtask/src/absorb/plan/to.rs`, `xtask/src/absorb/proof.rs`,
`xtask/src/absorb/read.rs`).

## D9. The kernel features R4 added stay, with no `@gsd/core` user

These stay in the kernel and the host:

- the decimal field type ([ADR 0361](0361-a-kind-field-may-be-a-number-an-exact-decimal-written-as-one-word.md)),
  whose one `@gsd/core` user was a requirement metric's `threshold`;
- a judged target that waits for its owner state, `judged_while`
  ([ADR 0362](0362-a-judged-target-may-wait-for-its-owner-state.md)), whose one user was `gsd/I536`;
- a code whose findings wait for it, `once_complete`
  ([ADR 0363](0363-a-codes-findings-may-wait-for-its-owner-state-and-a-write-that-reveals-one-adds-no-error.md)),
  whose one user was `gsd/E531`;
- the named reverse of a self-relation or of a union, which ADR 0359 D12 added to the rule of
  [ADR 0354](0354-a-reference-field-that-names-its-reverse-is-its-own-relation.md), whose users were
  `decision.supersedes`, `decision.affects`, `requirement.constrains` and `risk.mitigated_by`.

After this ADR no `@gsd/core` kind, rule or code uses any of them. They are not removed here: each is a host rule
another extension may use, and a later change removes one only if nothing else uses it.

## D10. The prose marks leave with their only reader

A field's prose mark said its text was the author's own wording. The vocabulary passes, `gsd/W534` and `term.used_in`,
were the only code that read a mark, and they leave with `term` (D2). A mark that nothing reads holds nothing, so
every mark leaves and `gsd_kit`'s `Field` has no prose flag. No mark is left to review and no golden pins one.

## What each decision of ADRs 0357, 0358 and 0359 becomes

"Reversed" means the decision no longer holds. "Kept" means it holds as written. "Moot" means nothing is left for it
to apply to.

| ADR | Decision | Outcome | Why |
|---|---|---|---|
| 0357 | D1 | reversed | `persona`, `journey` and `term` leave `@gsd/core` (D2) |
| 0357 | D2 | reversed | a channel was a word of `journey.channels` and its text a term's; both kinds leave (D2), and an old channel is dropped (D8) |
| 0357 | D3 | reversed | a journey's steps leave with the journey (D2) |
| 0357 | D4 | reversed | `journey.milestones` and the milestone's reverse `journeys` leave with the journey (D2) |
| 0357 | D5 | reversed | the journey leaves, and the requirement's `priority` leaves (D3) |
| 0357 | D6 | reversed | the term leaves, and `gsd/W538` with it (D2) |
| 0357 | D7 | reversed | the persona and its status leave (D2) |
| 0357 | D8 | reversed | the milestone's eight planning fields leave, with `gsd/E537` and the milestone half of `gsd/E102` (D3) |
| 0357 | D9 | moot | no product kind is left for a command to read |
| 0357 | D10 | moot | no product kind is left for the rules it declined to carry |
| 0357 | D11 | kept | a project loading an old builtin beside `@gsd/core` is still E026, now on `decision`, `deliverable` and `milestone` alone, the keywords both still declare ([ADR 0369](0369-the-builtin-vocabulary-maps-a-kind-keyword-to-every-builtin-that-declares-it.md)) |
| 0357 | D12 | moot | ADR 0364's census gives the old deliverable to gsd-core's own deliverable kind, written by the conversion change; `artifact_types` and the journeys it named are gone (D2, D3) |
| 0357 | D13 | reversed | zero loss is no longer the rule: a value with no home is dropped with its reason (D1, D8) |
| 0357 | D14 | reversed | `gsd/E537` leaves with `starts` and `targets` (D3) |
| 0357 | D15 | kept in part | `has_word` at the JavaScript boundary, which `gsd/E312` reads, and `folded`, which the git collector reads, stay; the Unicode boundary goes with its only readers, `gsd/W534` and `term.used_in` |
| 0357 | D16 | reversed | the marks leave with the vocabulary, their only reader (D10) |
| 0357 | D17 | reversed | `term.used_in` leaves with the term (D2) |
| 0357 | D18 | reversed | `gsd/W534` leaves with the term (D2) |
| 0357 | D19 | reversed | a journey names no requirement: the journey leaves (D2) |
| 0357 | D20 | reversed | `journey.covered` leaves with the journey (D2) |
| 0357 | D21 | reversed | `journey.tests` leaves with the journey (D2) |
| 0357 | D22 | reversed | `uat_test.journey` leaves (D3), with `journey.last_uat` and the asset lines that pass `--journey` |
| 0357 | D23 | reversed | `milestone.journeys_covered` leaves with the journey (D2) |
| 0357 | D24 | reversed | `gsd/W530` leaves with the journey (D2) |
| 0358 | D1 | reversed | `risk` leaves `@gsd/core`; a risk is a threat of `@gsd/security` (D2) |
| 0358 | D2 | reversed | `rpn` and `post_rpn` leave with the risk (D2) |
| 0358 | D3 | reversed | a risk's status leaves with the risk (D2) |
| 0358 | D4 | reversed | `blocking` and the setting `specforge.risk_block_on` leave (D2) |
| 0358 | D5 | reversed | `gsd/W532` leaves with the risk (D2) |
| 0358 | D6 | reversed | `risk_level` leaves the phase and the milestone (D2) |
| 0358 | D7 | reversed | `mitigated_by` leaves with the risk, so its reverse is no question; ADR 0359 D18 had already amended it |
| 0358 | D8 | reversed | an old failure mode converts to nothing: its values are dropped, and no word scale is read (D8) |
| 0358 | D9 | reversed | `invariant` leaves `@gsd/core`; an old invariant becomes a requirement (D2, D8) |
| 0358 | D10 | reversed | no invariant is in force: the kind leaves (D2) |
| 0358 | D11 | reversed | an invariant's `status` and `last_verified` leave with the kind (D2) |
| 0358 | D12 | reversed | `gsd/I536` and the prompt `verify_invariant` leave (D2); the kernel's `judged_while` stays (D9) |
| 0358 | D13 | reversed | an invariant's `scope` leaves with the kind (D2) |
| 0358 | D14 | reversed | no invariant mitigates a risk: both kinds leave (D2) |
| 0358 | D15 | kept | a host rule of the registry (amending ADR 0331 D7), for every judged rule and context route, not only for `gsd/I536` |
| 0358 | D16 | reversed | `gsd/E531` and the `invariants` pass leave (D2) |
| 0358 | D17 | reversed | nothing is guarded by an invariant: the kind leaves (D2) |
| 0358 | D18 | reversed | `gsd/E531`'s window leaves with the code (D2) |
| 0358 | D19 | reversed | no verdict covers a commit for `gsd/E531`: the code leaves (D2) |
| 0358 | D20 | reversed | `gsd/E531` leaves (D2); the kernel's `once_complete` stays (D9) |
| 0358 | D21 | reversed | `gsd/E531`'s file fallback leaves with the code (D2) |
| 0358 | D22 | reversed | the join and `phase.invariants_due` leave (D2) |
| 0358 | D23 | reversed | `gsd/E531` leaves (D2) |
| 0358 | D24 | kept | `set_verdict` records a verdict's time to the second for every judged rule, and the verification and the phase ladder read it through `time::verdict_time` (amending ADR 0294 D3 and ADR 0289 D6) |
| 0359 | D1 | reversed | the heading is `category` again (D5) |
| 0359 | D2 | reversed | the quality class leaves (D3) |
| 0359 | D3 | reversed | `requirement.priority` leaves (D3) |
| 0359 | D4 | reversed | `requirement.metric` leaves; a bound is the requirement's text (D3, D8) |
| 0359 | D5 | reversed | `requirement.source` leaves (D3) |
| 0359 | D6 | reversed | `personas`, `constrains` and `protects` leave with their reverses (D3) |
| 0359 | D7 | reversed | `requirement.tags` leaves (D3) |
| 0359 | D8 | kept | `satisfied_by` is computed, which the rule allows; the links change re-sources the `task.items` it reads, and no `manual` task is left (D3) |
| 0359 | D9 | kept | `proof` is computed and is the one predicate `gsd/E515` reads (amending ADR 0331 D5 and ADR 0353 D6); the links change re-sources what it reads, and no `manual` task is left (D3) |
| 0359 | D10 | reversed | `requirement.effort` leaves (D3) |
| 0359 | D11 | reversed | `decision.supersedes` leaves (D4) |
| 0359 | D12 | kept | a kernel rule of the field registry (amending ADR 0150 D2), now with no `@gsd/core` user (D9) |
| 0359 | D13 | reversed | `decision.alternatives` leaves (D3); a checkpoint's options keep their three members |
| 0359 | D14 | reversed | `decision.affects` leaves (D3) |
| 0359 | D15 | reversed | `context`, `consequences`, `decided`, `deciders` and `tags` leave the decision (D3) |
| 0359 | D16 | reversed | `in_force` leaves, and `gsd/E310`, `W317` and `W506` count every context decision (D4) |
| 0359 | D17 | reversed | the asset lines carry upstream's text again, with no `--supersedes`, `--affects` or `--alternatives` (D3, D4) |
| 0359 | D18 | reversed | `mitigates` leaves with `risk.mitigated_by` (D2) |

ADR 0357: 18 reversed, 1 reversed in part, 1 kept, 1 kept in part, 3 moot. ADR 0358: 22 reversed, 2 kept. ADR 0359:
15 reversed, 3 kept. ADR 0359's table for converting an old governance decision follows D8: a value whose new field
leaves is dropped with its reason.

## Rejected

- **Keeping the five kinds as an extension of SpecForge's own.** `@gsd/core` mirrors gsd-core; a kind gsd-core does not
  define would belong to another extension, and none is planned.
- **Keeping the fields "for the conversion".** The conversion drops what has no home in GSD (D8); a field kept for it
  is a field gsd-core does not write.
- **Emptying the kinds but keeping their codes.** A code with no rule has no firing test.
- **Keeping `area` as the heading's name.** Its reason was the quality class, which leaves (D3); keeping it would make
  the field's name and the assets' text differ from upstream's heading.
- **A tier only a person writes, with the planner writing none.** It keeps the guard, but the planner's backstop tier
  has no home, and the verifier would judge a lifted edge as inferable until a person acts: a divergence from
  upstream.
- **Keeping `proposed_verification` as the planner's half of the tier.** It is a field upstream does not write.
- **Citing the tests that state a converted constraint's obligation.** The sample would keep running about 8 tests,
  but absorb would keep the proof join and add a cite-only home for a constraint's `verify`, the one home this change
  would add against D8's line, which the conversion change and the swap would then have to remove.
- **Giving the sample a truth whose proof names a test.** absorb would create a plan and a truth that no old entity
  is, and the links change removes `truth.proof` itself, so the route would last one change.
- **Removing the kernel features of D9 here.** Each is a host rule another extension may use; removing one is its own
  change.

## Consequences

- `@gsd/core` declares 61 kinds, 70 codes and 44 prompts. Every pin of the code count, the parity suite's counts in
  `gsd/parity/ACCEPTANCE.md` and the kinds and codebook goldens follow.
- The rules that read a removed field change or leave with it: `gsd/E310`, `W317` and `W506` no longer leave out a
  superseded decision (D4); `gsd/E102` judges phases and plans only (D3); computed risk levels, journey coverage and
  invariant status are gone (D2).
- absorb's ledger gains a `dropped` class. The sample's goldens are re-blessed, and the sample runs no test until the
  swap (D8).
- The warnings ADR 0360's Consequences expected on a converted project (`gsd/W530`, `gsd/W532`, `gsd/W534`,
  `gsd/W538`) no longer exist.
- The keywords `@gsd/core` shares with the old builtins are `decision`, `deliverable` and `milestone`; ADR 0369's rule
  reads them from the staged declaration and does not change.
- The kernel features of D9 have no `@gsd/core` user.
- Breaking: a project that writes one of the five kinds or a removed field, or sets `specforge.risk_block_on`, no
  longer checks clean: `@gsd/core` does not know them.

## Would reopen it

- gsd-core defining one of these kinds or fields in a release SpecForge tracks.
