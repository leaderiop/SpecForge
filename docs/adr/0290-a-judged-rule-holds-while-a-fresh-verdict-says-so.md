# A judged rule holds while a fresh verdict says so

**Status:** accepted (2026-10-28, architecture round 32, plan K7). Extends [ADR 0020](0020-rules-are-a-typed-set.md),
[ADR 0287](0287-who-writes-a-field-is-declared.md) and [ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md).
Takes extension protocol 1.9.0. Extended by [ADR 0297](0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md) (a judged rule's finding is not an error a write adds) and by [ADR 0299](0299-the-graph-a-call-receives-carries-what-every-surface-shows.md) (a call's graph lists the judgements).

## Context

Some checks need judgement: does a plan cover its requirements, does code satisfy a truth, does a decision hold. A rule
cannot compute them, so GSD had agents record the outcome in a field, and nothing noticed when what the agent read
changed after: a plan checked once stayed checked. A status that reads "checked" must mean "checked over what is here
now".

## Decision

- **D1. A rule may be judged**: `check: "judged"` with a `judged` descriptor (`judge`, `outcomes`, `passing`, targets
  with named inputs and `unless_set`, `values`, `outcome_fields`). It holds while a fresh verdict of an outcome it calls
  passing says so; no verdict fires it as missing, a stale one as stale, a failing outcome as failed.
  Amended by [ADR 0295](0295-a-judged-rules-judge-may-be-a-prompt-of-its-extension.md): the `judge` may be `prompt:<id>`.
- **D2. A verdict is a member of an entity's block**: `verdict <rule> <outcome> { judge model at cited cited_tests
  inputs findings values }`, beside fields and `verify` statements, never a field. Only `set_verdict` writes it, and
  records a ledger line under `verdict:<rule>`; a verdict with no line is unrecorded (E083) and counts as missing.
- **D3. Inputs are selectors over the project**: `self`, a kind, `.field` hops, `<kind.field` back-references,
  `file(..)` and `test(..)`. A verdict records the digest of each input and of what it cites (always last, empty
  included). Entities digest by content and full path, files by bytes (a project `.spec` source by the entities it
  declares), tests by name and status only. Where the report is missing, test inputs are skipped and I205 says so once.
- **D4. Freshness is evaluated once per compile**, after the graph build and before the compute passes (so statuses
  derived from a verdict read it), over a memo of digests that a graph delta forgets entity by entity and a file stamp
  forgets file by file; a verifying session compares it with a cold evaluation. A gated rule is not evaluated, listed
  or asked for.
- **D5. A field may be set by a verdict** (`writer: "verdict"`): it holds the outcome or a `values` member of a fresh,
  recorded verdict and nothing otherwise; written by hand it is E082. Passes see each entity's `judgements`.
- **D6. The inputs are read before they are judged**: `specforge query <id> --inputs <rule>` and
  `specforge.verdict_inputs` answer each input, the verdict's state and the `basis` that `set_verdict` is checked
  against, by the one function the check calls. A basis that differs is `stale_revision`.
- **D7. History**: the baseline's judgements are evaluated at the baseline commit over that commit's files
  (`History::blobs`), without a report, so `re_verification` compares statuses computed alike at both ends.
- **D8. Outputs**: `inspect`, `query`, the graph export and a command's graph list each entity's `verdicts` and
  `judgements`; the editor's hover lists them and a code lens says fresh, failed or stale.

## Rejected

A computed field a pass derives from findings (a pass cannot read files or reports). A verdict as a field (a field has
no inputs, and `verdict` stays a legal field name). A fresh/stale flag stored beside the verdict (it would be the
agent's word). Treating no report as stale (a machine without the report would stale every verdict). Translating
digests through move records (a rename stales what read the renamed entity, which is the safe answer).

## Consequences

A new rule check, statement, writer, two codes (W168, I205), a protocol minor, a ledger key namespace, a memo the graph
delta feeds, and a history question. The registry's `Verdict` stays a custom rule's pass or fail, and the new words are
`Judgement` and `Verdict` statement.
