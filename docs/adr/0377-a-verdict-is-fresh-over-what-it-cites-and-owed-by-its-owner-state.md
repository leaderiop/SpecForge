# A verdict is fresh over what it cites and owed by its owner state

**Status:** accepted (2026-11-19). **Amends**
[ADR 0304](0304-a-judged-input-reads-a-path-field-goes-on-past-a-reference-and-a-pass-sees-the-tests-a-verdict-cites.md)
(its Consequences: E501 no longer reads a truth's plan files). **Extends**
[ADR 0362](0362-a-judged-target-may-wait-for-its-owner-state.md) with new users of `judged_while`: E501's targets and
W525 wait for a complete owner, the plan checks for an open one, W302 for a closed threat. Builds on
[ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) D2-D3 (an unrecorded verdict counts as missing;
`cited` is digested for every verdict), [ADR 0295](0295-a-judged-rules-judge-may-be-a-prompt-of-its-extension.md) (a
prompt of its extension may judge),
[ADR 0302](0302-a-judged-finding-is-the-judged-evaluations-own-and-an-extensions-finding-is-paired-by-its-subject.md)
(a judged finding is the evaluation's own, so E516 stays at the gate),
[ADR 0286](0286-a-diagnostic-code-is-namespaced-by-its-extension.md) D1 (each extension owns its number space) and
[ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md) D1 (no new authored field on `truth`). No protocol
change and no kernel change: the host's judged machinery, owner states, gate triggers and verdict origin check already
do what these decisions need.

## Context

- **One edit staled every truth of a plan.** E501's truth target read the plan-wide input
  `file(self.plan.files_modified)`, so a change to any file the plan touched staled the verdict on every truth of that
  plan, including the truths whose text is about another file. The verifier already cites the files it read, and the
  host already digests those citations as the verdict's `cited` input; the plan-wide input was a proxy for them.
- **A hand-written exception decided when a plan check counts.** The gate guards skipped the plan-check verdicts of a
  plan that had a summary carrying `completed`, a second definition of "executed" beside the plan's own `status`, which
  its kind's `complete_when` already reads.
- **Only `@gsd/core` could write a verdict.** `record_verdict` hard-coded the `gsd` namespace and core's table. Two
  `@gsd` extensions share the command short `gsd`, and a second command of the same short and name is shadowed, never
  run, so `@gsd/security` could neither reuse core's command nor declare one of the same name.
- **E516 saw only stale verdicts.** A missing, unrecorded or unreadable verdict passed every gate, though none of them
  is a usable verdict.

## D1. A truth's verdict is fresh over what it cites

E501's truth target reads `truth`, `plan`, `phase` and `tests`, plus the verdict's `cited` and `cited_tests`, which the
host digests for every verdict. It no longer reads the plan's `files_modified`. Artifacts and key links keep their own
file inputs, which already name only their own files. The verdict command refuses a truth verdict that cites no file in
the project and no test, whatever its outcome: with nothing cited, the verdict could never go stale on a code edit. No
field is added to `truth` (ADR 0370 D1): the citations are the verdict's own record.

A recorded input that the target no longer names is not compared, so the verdicts already committed stay fresh after
the input is dropped; they are re-recorded with their citations.

The reason: a verdict says what it read, and only a change to what it read can make it wrong.

## D2. A verdict is owed by its owner state

E501's targets (truth, artifact, key link) and W525 are judged `once_complete`: a must-have owes the verifier a verdict
once its plan is complete. The plan checks (W311, W313, W315, W316) are judged `while_open`: a plan check is owed while
its plan is open. Executed means the plan's `status` is `complete`, the one source of truth its kind's `complete_when`
reads, not a summary carrying `completed`; the hand-written exception in the gate guards goes. E202, E518 and W205 bind
verdict values or outcome fields, which cannot wait for an owner state (ADR 0362 D3), so they are owed from creation,
which is where their judge runs.

The reason: "missing" means something only where a verdict is owed, and the owner state is the one place that says when.

## D3. Staleness is read from the input digests, not from the graph delta

A verdict is stale when the digest of one of its inputs now differs from the digest it recorded. The graph delta of
[ADR 0376](0376-a-snapshot-is-a-graph-at-a-revision-and-a-pass-reads-what-changed.md) is taken against a base the host
picks (none, a revision's merge-base or the previous check), never against a verdict's time, and files are not graph
nodes, so it cannot say what changed since a verdict. No gsd pass declares the delta input yet.

The reason: the per-input digests are exact, already memoised per entity and per file stamp, and recorded at the
verdict itself.

## D4. `re_verify` names what no longer holds

`re_verify <phase|plan> [--stale-only]` is a read-only command of `@gsd/core` over the judgements the command graph
already carries, for the rules the verifier judges. It answers each judgement with its state, its outcome, and its
changed inputs mapped to their selectors from the rule's target; `cited` and `cited_tests` map to the verdict's
citations. `--stale-only` keeps the judgements that are not fresh: stale, missing, unrecorded and unreadable. A fresh
verdict, passing or failing, is never listed, since judging unchanged inputs again would answer the same. The execute
workflow runs it before the verifier, who judges exactly what it lists and leaves every other verdict as recorded.

The reason: the verifier should spend its judgement on what changed, and the table that declares a rule already owns
the selectors of its inputs.

## D5. A capability extension declares and writes its own judged rules

The judged table types, their declaration and the verdict command builder live in `gsd_kit::judged`, which every
`@gsd` extension links. Each extension declares its own table and its own verdict command, with its own name
(`record_verdict` for `@gsd/core`, `record_security_verdict` for `@gsd/security`) and its own namespace. The judge of a
verdict is the rule's own, never an argument. The host's origin check is unchanged: a verdict must come from a command
of the rule's extension, so no peer writes another extension's verdict. A threat declares
`complete_when("status", ["closed"])`, so a rule on it can wait for it to close.

The reason: a capability extension cannot link core, and a shared builder keeps every verdict command's arguments,
refusals and output word for word the same.

## D6. Three more judged rules

- `gsd/W525`: a judgment-tier prohibition owes the verifier a verdict. A warning, `unless_set test` (a prohibition
  that names its enforcing test is checked by that test), judged once its plan is complete. A prohibition judges its
  plan's whole change ("no code of the plan does X"), so its target reads `files`
  `file(self.plan.files_modified)` beside the prohibition, its plan and its phase, the plan-wide input D1 removes from
  a truth, whose text is about some files of the plan only. The verdict also reads what it cites, and the verdict
  command refuses one that cites no file of the project and no test, as it refuses a truth's: what a prohibition
  forbids may appear in code outside the plan's list, and the verifier cites where it looked. An edit to the plan's
  files or to a cited file stales it.
- `gsd.security/W302`: a closed threat's disposition holds over its `files`, judged by the prompt `secure_phase`, once
  the threat is closed. An edit to a file the threat names stales exactly the threats naming it.
- `gsd.security/W303`: a phase's threat flags are judged against its threat register, by the prompt `secure_phase`.

With them the code books hold twelve judged rules.

The reason: a mitigation the auditor checked and a flag the executor raised are judgements, and a judgement nobody
records is not on file.

## D7. A verdict that is not fresh blocks every step but the one that judges it

E516 is reported at a gate only, for a judgement whose state is stale, missing, unrecorded or unreadable. Stale,
unrecorded and unreadable count anywhere in the project; missing counts only within the subject's scope. Within that
scope, a judgement whose rule's judge the workflow itself runs is excused: an agent step of the workflow, the workflow
itself as `prompt:<id>`, transitively through the prompts it runs; another extension's rule is never excused by a core
workflow. One E516 is reported per entity and rule, naming the subject. The code is retitled "Verdict not fresh blocks
the workflow", and its setting is renamed `specforge.verdict_gate`, with no alias.

The reason: a verdict on record that no longer holds refuses every step, and the workflow that records a verdict is its
remedy, not its victim.

## Rejected

- **A context-route filter on judgement state.** A selector filters by an authored scalar field; a filter on judgement
  state would be a grammar change that a judged input must then refuse, since a judgement cannot depend on judgements.
- **A kernel bulk judgement read.** No second consumer needs selectors per changed input, which core already owns, and
  `specforge check` already lists every judgement that does not hold.
- **A value filter on a target.** ADR 0362 refuses it: the owner state already says when a verdict is owed.
- **A check-time E516.** The judged evaluation already reports every judgement that does not hold as the rule's own
  finding at every check.
- **A new field on `truth`.** The verdict's citations are its own record.
- **A core command that writes a security verdict.** The host's origin check refuses it, and rightly: an extension
  writes only its own verdicts.

## Limits

- A changed `cited` input names the input, not which cited file changed: a verdict records one digest for all its
  citations.

## Consequences

- An edit to one file re-judges exactly the truths whose verdict cites it.
- The verifier sees, before it judges, which must-haves no longer hold and why.
- A plan check is owed while its plan is open, a must-have once its plan is complete, a threat's disposition once it is
  closed.
- `@gsd/security` declares and writes its own judged rules.
- A gate refuses a step while a verdict it depends on is not fresh, except the step that judges it.

## Would reopen it

- A verdict that must name which of its cited files changed.
- A second consumer of judgements in bulk.
- A gsd pass that needs the graph delta.
