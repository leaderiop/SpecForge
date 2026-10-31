# A collector's plan is judged for host errors only

**Status:** accepted (2026-10-31, kernel round K20a, plan 13). Amends
[ADR 0297](0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md) (a collector's plan is judged against the
report it wrote). No protocol change.

## Context

`apply_collected` applies each plan with `Origin::Collector` through the same E080 as a command. A collected fact (a
commit, a test result) is history: refusing it for an extension's error leaves it unrecorded, fixes nothing and starves
every later fact (REVIEW-r38 C1, M1). ADR 0297 rejected "host errors only" pending its one consumer, `gsd/E515`; E515
is built as a guard at the ship gates (it refuses no write), so the consumer is gone.

## Decision

For `Origin::Collector`, E080 ignores every diagnostic whose origin is an extension's, whatever its code. Host errors
(E003, E016, ...) still refuse. The extension's errors are reported after the write. `host_only` is threaded through
`validate::check`, `plan_steps` and `refuse_new_errors`; the typed edit passes `false`.

## Rejected

Exempting only the collector's own extension (a peer's rule would still starve it).

## Limits

A collector that adds a host error is still refused.

## Consequences

An extension's rule can no longer block history from being recorded; it reports it.
