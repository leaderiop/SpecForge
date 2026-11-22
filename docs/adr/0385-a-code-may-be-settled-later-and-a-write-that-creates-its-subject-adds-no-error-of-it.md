# A code may be settled later, and a write that creates its subject adds no error of it

**Status:** accepted (2026-11-22). **Amends** [ADR 0297](0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md)
(E080 counts a third finding as no error a write adds, beside a judged rule's),
[ADR 0363](0363-a-codes-findings-may-wait-for-its-owner-state-and-a-write-that-reveals-one-adds-no-error.md) D3 (an
unmatched error is counted unless it is judged, revealed or settled later) and
[ADR 0286](0286-a-diagnostic-code-is-namespaced-by-its-extension.md) D3 (a `codes` entry may declare
`settled_later`). Answers the Limits of
[ADR 0384](0384-a-phases-plans-are-written-in-one-edit-and-its-spec-is-committed-like-code.md) in the kernel and
amends its D4 ("E080 and what it counts" do not change). Protocol 2.3.0 is unreleased: the key goes into its row, no
bump. The wire and the SDK change, so every builtin is re-vendored.

## Context

- E080 refuses a typed edit when the project it leaves reports an error the project does not (ADR 0167, keyed by code,
  severity, message shape, file and, for an extension's finding, its subject, ADR 0302). Two findings are exempt: a
  judged rule's (ADR 0297) and one a write revealed by moving its entity's owner state (ADR 0363).
- Some findings name a state an authoring workflow passes through on purpose: an entity is created first, and what
  settles it comes in a later step. `@gsd/core` has three: an open question no decision resolves yet (`gsd/W204`, on
  the question), a phase whose requirement is in no plan yet (`gsd/E301`, on the phase, from its first plan) and,
  under gsd-strict, a plan with no truth and no artifact yet (`gsd/W309`, on the plan).
- E080 refuses the write that creates such an entity, because the finding is new. The only accepted route is one edit
  that also settles it. There is none when the settling is a person's later answer (the question's decision), a later
  run (quick's plan, minted before its planner runs) or the rest of a phase planned plan by plan. ADR 0384 recorded
  these three refusals as its Limits and named the fix a kernel one.
- The finding is right about the project the write leaves; it is not the write's mistake. The rule that says so is a
  property of the code, which only the extension that owns the code knows.

## D1. `settled_later`, a property of the code

A `codes` entry may declare `settled_later` (`CodeDescriptor.settled_later`, a bool, absent when false). The SDK
writes it with `CodeBuilder::settled_later()`. The code book keeps it on the owner's own entry
(`ExtensionCode.settled_later`) and answers `Codebook::settled_later(extension, code)` for the extension that owns the
code's namespace only, as `Codebook::machine` does. A first-party extension declares no `codes` (ADR 0286), so no host
code is settled later. A bool has no malformed value: E030 does not change.

The name says what the declaration means: a later step of the authoring settles the finding. `open` is taken
(`while_open()` writes `reported_while: pending`, ADR 0363), and a word about severity (deferred, lenient) would be
wrong: the finding's severity does not change.

## D2. E080: a settled-later finding on what the write creates adds no error

A typed edit's judge and the re-judge under the write lock count an unmatched finding as no added error when all hold:

- its origin is an extension, and that extension's own entry for the code declares `settled_later` (another
  extension's, a host diagnostic and a code that does not declare it are counted);
- it names a subject entity;
- the write's plan creates that subject, or an entity under it: an id that is the subject's followed by `/`, the
  scope path of ADR 0284.

The plan creates what its `create` edits name, a typed one or an extension plan's, each at the id it has once the plan's
later moves are made. A move, a rename and a delete create nothing; a delete and a create of one id in one write
create it. A migration, the rule's other caller, has no plan and creates nothing: it counts every such finding.

The finding of an edit that only changes or removes is counted as before: unsetting the decision that resolves an
existing question, or deleting the one plan that holds a requirement, is refused. So is a write that creates an
entity elsewhere: the creation must be the subject or under it.

## D3. Nothing else changes

The finding keeps its severity in `check`, `--strict` promotes it, every gate and a prompt's entry gate read it, a
profile's raise applies to it, and the write lists it among the findings it reports (`EditReport.added`). It is never
hidden: it stays in the project, failing `check` at its severity, until the step that settles it is written. A paired
finding (one the project already reports) is unchanged. ADR 0363 D3 holds: an entity a write creates reveals nothing.

## Rejected

- **Never counting a settled-later code** (the declaration alone): any write could leave such a finding anywhere,
  edits and deletes included, and E080 would stop guarding the codes that declare it.
- **A per-request scope of open findings** (the caller names the codes a write may leave): the policy moves to every
  caller, E080 becomes advisory, and the MCP tool, the CLI and the edit plan all change.
- **A transaction over several edits**: state across processes that every read must overlay, a collision with the
  harness ledger and the one project change of ADR 0128, and a question the user leaves unanswered is still refused.
- **Rewriting the rules in `@gsd/core`** (hiding the findings from `check` while authoring, lowering a severity at
  write time, a judged rule with a fake verdict): each hides from `check` what is true, or invents a verdict.
- **Widening ADR 0363's revealed finding to a creation**: a creation moves no owner state, and ADR 0363 D3 rules a
  created entity out on purpose.
- **Keying the exemption by the entity kind** (only a plan under a phase, say): it needs a kind in the declaration
  for a precision no case needs yet (Would reopen it).

## Limits

"Under it" is wide: a write that creates a plan under a phase also excuses a new settled-later finding of another
rule on that phase in the same write. `check` and every gate still report it.

## Consequences

- A workflow may create an entity whose settled-later finding a later step settles: the write is accepted, and the
  finding is reported by the write, by `check` and by every gate until then.
- A state left between two steps (a chunked run stopped midway, a question left open) is in the repository and fails
  `check` at its severity, under gsd-strict and in CI too. That is intended; a workflow says so.
- `@gsd/core`'s side is recorded in the next change, the one that declares its codes settled later: which codes, the
  workflows' texts that then record an open question open and write a phase's plans plan by plan, ADR 0384's D1
  reason, D3 and Limits, and the gates that then read those findings.

## What each decision becomes

| ADR | Decision | Outcome | Why |
|---|---|---|---|
| 0297 | Decision (a judged rule's finding is no added error) | kept, amended | a settled-later finding on what the write creates is not one either (D2) |
| 0363 | D3 (a revealed finding is no added error; every other is counted) | amended | the third exemption (D2) |
| 0363 | D3 (an entity a write creates reveals nothing) | kept | D3 |
| 0286 | D3 (an extension catalogs its codes) | amended | an entry may declare `settled_later` (D1) |
| 0384 | D4 (E080 and what it counts do not change) | amended | D2 |
| 0384 | Limits (a kernel fix, an owner decision) | answered in the kernel | D1, D2; `@gsd/core`'s side in the next change |

ADR 0297: 1 amended. ADR 0363: 1 amended, 1 kept. ADR 0286: 1 amended. ADR 0384: 1 amended, 1 answered.

## Tests that hold it

Each test fails when the decision it holds stops being true.

| Decision | Tests |
|---|---|
| D1 | `specforge-protocol-types` `declaration::codes_round_trip_on_the_wire`, `specforge-extension-sdk` `declaration::a_code_entry_may_be_settled_later`, `specforge-diagnostics` `codebook::tests::settled_later_reads_the_owners_own_entry`, `specforge-registry` `build::namespaces::a_settled_later_code_reaches_the_code_book_and_no_e030` |
| D2 | `specforge-ops` `authoring::planned::tests::a_change_opens_what_it_creates_and_every_entity_it_is_under`, `authoring::edit::declared::tests::a_created_entity_is_followed_through_a_later_move`; in `operations::settled_later`: `a_finding_on_an_entity_the_write_creates_adds_no_error`, `a_finding_on_an_entity_the_write_creates_something_under_adds_no_error`, `a_write_that_creates_nothing_there_still_adds_the_finding`, `only_the_owners_settled_later_code_is_exempt`, `the_re_judge_under_the_lock_exempts_it_too`, `a_migration_counts_it` |
| D3 | `operations::settled_later::a_finding_on_an_entity_the_write_creates_adds_no_error` (the write lists the finding, and `check` reports it once written) |

## Would reopen it

- A settled-later finding that must be excused only for some kinds of what a write creates.
- A finding a later step settles on an entity the write neither creates nor holds anything new under.
