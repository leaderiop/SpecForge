# A judged finding is the judged evaluation's own, and an extension's finding is paired by its subject

**Status:** accepted (2026-10-31, kernel round K20a, plan 13). Amends
[ADR 0297](0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md) (D1, D1b),
[ADR 0151](0151-a-spec-file-is-changed-by-a-typed-edit.md) and
[ADR 0167](0167-a-typed-edit-is-compared-with-the-checked-report.md) (the E080 key). No protocol change.

## Context

E080 pairs the errors of the edited project with the project's own by `(code, severity, message with digits
collapsed, file)`. An extension's pass reports `explicit truth '.../TR1' is verified, but ...` and the same for `TR2`:
the digits collapse alike, so two `set_verdict` ops can swap an error and E080 sees nothing added. ADR 0297 D1b
exempts a judged finding by code and owning origin only, so a check pass or a guard that reuses a judged rule's code
inherits the exemption (both holes were named "still open").

## Decision

- **D1. The key gains the subject.** For a diagnostic with an origin (an extension's), the key's fourth text part is
  its `Subject` entity. Host diagnostics keep today's key. Before-side subjects are relocated through the plan's moves
  (renames, renumbers), so a `/gsd-phase` renumber is not refused wherever an extension error stands.
- **D2. Only the evaluation's own findings are exempt.** `Judgements` records every finding the judged evaluation
  pushes; `is_finding` compares code, message, origin, subject and file (not severity: a preset may raise it). The
  exemption asks the post project. A check pass that reuses a judged code, and a guard's fail-closed error, are
  counted.
- **D3. Registry.** A guard pass whose `reports` names a judged rule's code of its extension is W021.

## Rejected

Digits kept in the message key (a moved line breaks it); a subject key for host diagnostics (E002 and friends need
none); a state marker on the diagnostic (as ADR 0297 rejected).

## Limits

A check pass declares no codes, so its reuse of a judged code is caught at run time only.

## Consequences

`added` takes the plan's moves; `PlannedChange` keeps them for `rejudge`; migrations and collectors' re-reads pass
none.
