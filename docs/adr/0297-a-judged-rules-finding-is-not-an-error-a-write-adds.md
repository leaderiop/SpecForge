# A judged rule's finding is not an error a write adds, and a collector's plan is judged against the report it wrote

**Status:** accepted (2026-10-29, kernel round K17, plan 07b). Extends [ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md)
and [ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md). Amends
[ADR 0151](0151-a-spec-file-is-changed-by-a-typed-edit.md) (D3), [ADR 0167](0167-a-typed-edit-is-compared-with-the-checked-report.md)
and [ADR 0211](0211-a-migration-is-judged-by-the-errors-it-adds-whatever-its-graph.md). No protocol change.

## Context

E080 refuses a write when the edited project reports an error the project does not (an *added error*, keyed by code,
severity, message with its digits collapsed, and file). A judged rule (ADR 0290) reports at its own severity on every
node of its kinds unless a fresh passing verdict says otherwise. Four consequences, each proved by a test:

1. **A failing verdict is refused.** Before: `…: no verdict`. After `set_verdict … failed`: `…: the verdict of <date> is
   'failed' (0 findings)`. The message differs, the code, severity and file do not: the after-finding matches nothing in
   the before and, on an Error rule, is an added error.
2. **A new judged target is refused.** Creating a truth, a phase spec, an artifact or a key link adds a node the rule
   targets, with no verdict: one more E501 or E202 than the project had.
3. **An edit that stales a passing verdict is refused.** A fresh passing verdict reports nothing; any edit to an input
   makes it `stale`, which reports at the rule's severity: new. After the first verified truth, every write to its plan or
   phase is refused.
4. **A writing collector's plan is judged before its own report.** `collect` writes the report, then the plans are judged
   over the view compiled before it. An error the recorded results alone cause (a pass that reads the tests) is unmatched
   in the before, and the collector's commits are refused whole.

## Decision

- **D1. A judged rule's finding is never an added error.** A diagnostic whose code is a judged rule's
  (`RegistryBuild::judged_rule`) is still paired against the project's report first, so an unchanged finding is not
  reported twice; an unmatched one is reported with the write's other warnings and never refuses it. The rule is the same
  one behind E080: the under-lock re-judge and a migration's `added_errors` use it too. It covers every state of a
  judgement (no verdict, stale, failing): no typed edit writes a verdict member, so a forged or unreadable one cannot come
  from a write, and the compile's own checks (E083, W168) are unchanged. Nothing becomes passing: `check` and every gate
  read the finding, and a failing verdict stays failing. Every other error, an extension's included, is counted as before.
- **D2. A collector's plan is judged with its report.** When a collect answered plans, `apply_collected` re-reads the
  project over the disk as the collect left it and applies each plan over that view. The plans keep the revisions they were
  sent (a collect writes no `.spec` file); a plan whose file changed under it is `stale_revision`, as before. The cost is one
  more compile per collect that answered a plan. A plan's `set_verdict` therefore checks its `basis` against the project
  with the collect's report: one whose entity's inputs the report changed (a `test(self)` input, say) is refused with a
  stale basis, where before it was accepted and went stale on the next compile (a behaviour change; it fails closed).
  The re-read view keeps the call's containment.
- **D1b. The exemption is the owning extension's.** The diagnostic must also name the extension that owns the judged
  rule as its origin (the judged evaluation sets it), so a host diagnostic or another extension's with the same text is
  counted. The registry refuses (W112) a declarative rule of the extension that shares a judged rule's code. Still open
  (now K19): a pass that `reports` a judged rule's code, and a guard pass's fail-closed "could not be evaluated" error for
  such a code, carry the extension's origin and are exempt too (a guard that lists a judged code fails open on writes);
  closing it means having `CompiledProject` keep the findings `judged::evaluate` produced and match against that list.
- **D3. No protocol change.** No type, export or descriptor key changes; `PROTOCOL_VERSION` stays 1.11.0 and no blob is
  rebuilt. The E080 catalog text gains one clause.

## Rejected

- Keying a judged finding without the verdict part of its message, or by rule and subject: they fix claim 1 only; a new
  subject (claim 2) is still a new key and a stale message (claim 3) still differs.
- A state marker on the diagnostic (exempting only missing, stale and failing): a new `DiagnosticData` variant that
  changes every judged finding's JSON, the published schema and the gsd goldens, to tell apart states a write cannot produce.
- Judging a collector's plan for host errors only (REVIEW-r38 M1): it would let a plan add any extension error. It needs its
  own decision with its one consumer, gsd/E515, which is not in the round that needs this.

## Consequences

A write may leave judged Errors: `check` exits non-zero on them, every gate reads them, and a write's output lists them. A
verifier records `failed` instead of being refused, and an author creates a judged entity before its verdict is owed. A pass
that reports a judged rule's code of its own extension would have that finding exempted too (D1b); no builtin or gsd
pass does.
