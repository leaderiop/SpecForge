# A command call carries a host nonce

**Status:** accepted (2026-11-06, S1 round). Extends [ADR 0294](0294-a-pass-sees-a-list-field-as-its-items-and-a-writing-call-is-told-the-time.md)
(the clock a writing call is told) and [ADR 0337](0337-the-ports-protocol-is-2-0-0-inputs-are-one-list-and-effects-are-journaled.md)
(protocol 2.0.0). Amends [ADR 0325](0325-pick-prints-one-value-of-a-json-answer.md) D7 (`--fields` on an extension
command).

## Context

- `@gsd/core` mints quick ids: `<yymmdd>-<three base-36 digits>`. Its workflows made them in the shell, from `date`
  and `/dev/urandom`, and could not check an id free in the graph.
- A guest already has randomness: its WASI context keeps the clocks and the random source wasip2's std needs
  (`no_capabilities`). So the guest does not lack entropy. What a draw from there lacks is a way to pin it: the
  command's answer would not be a function of its input, and a test could only assert a shape.
- The clock is the model to follow. A command is told `now` by the host (ADR 0294), a test pins it, and every value
  made from it can be asserted exactly.

## D1. The field

`CommandInput.nonce`: 16 bytes from the OS's entropy, as 32 lowercase hex digits, drawn per call beside the instant.
It is skipped while empty. The nonce is randomness, not a secret: it may be printed, journalled or written into an id,
and it grants nothing, so ADR 0288's rule that a secret's value never reaches a guest does not touch it. Nothing may
treat it as a credential: 16 bytes a guest can read are no proof of anything.

## D2. Which calls carry it

A command's: `run_on` is the one place the host builds a command's input, so the CLI and MCP both send it. A pass and a
prompt do not (they are reproducible, and a value that changes per call would break that). A hook and a collector do
not (nothing asks for one; each would add it under its own decision). The host says nothing about what a nonce is for,
and knows nothing of quick ids.

## D3. A guest draws from it deterministically

A guest that needs random values derives them from the nonce with its own deterministic generator, so its answer is a
function of `(nonce, input)`. A test passes a nonce as it passes an instant, and asserts the exact values. The SDK reads
it as `call.nonce()`, and its harness sends the feed's nonce, or 32 zeros, so a test run is reproducible. A guest must
treat an empty or malformed nonce as it treats an empty clock: a refusal, never a value made up.

## D4. An item of 2.0.0

2.0.0 is unreleased, so the nonce is one of its items (`version.rs`'s rule): the string stays `2.0.0`. An older 2.0.0
guest ignores the key. Protocol-types is an input of every blob, so the change re-vendors them.

## D5. `--fields` on an extension command

ADR 0325 D7 kept `--fields` from an extension command because "no prompt reads two of their values at once". A
`quick-init` answer is read that way: an id, its slug and its branch name come from one call, and a second call would
mint another id. An extension command takes `--fields` beside `--pick` unless it declares an arg of the name, as it
takes `--pick` and the shapes.

## Rejected

- **WASI randomness in the guest.** A test could not pin it, and a value drawn from the world would not be the host's
  to hand out (ADR 0335 keeps what a call takes from the world on the host's side). Closing WASI randomness to guests
  is a separate decision: wasip2's std needs it.
- **Time-based ids.** Upstream's grain gives one id per two seconds; a batch needs several at once.
- **A `specforge random` verb.** A process per id, and each id must be checked against the graph the command already
  holds.
- **A `u64`.** JSON readers lose bits above 2^53; a string prints as `today` and `now` do, and 16 bytes outlast any
  guest's draws.

## Consequences

- One key on a command's input, one accessor in the SDK, one field on the harness's feed.
- `specforge gsd quick-init` answers the ids, checked free in the graph, and the workflows' shell draws go.
- The input of a command call is not reproducible between two calls. Anything that keys on it (a journal, a replay)
  must not key on the nonce; none does today: the effects journal keys on the effect's request, not the call's input.
