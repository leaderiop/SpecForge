# A judged rule's judge may be a prompt of its extension

**Status:** accepted (2026-10-28, kernel round 35, plan K15). Extends [ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md)
(judged rules; its D1 names the `judge`), [ADR 0291](0291-an-extension-contributes-mcp-prompts.md) (prompts) and
[ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md) (agents are the other judges). Amends ADR
0290. Takes extension protocol 1.11.0.

## Context

A judged rule names who judges: an agent of its extension or of a declared peer. Some judgements have no agent. A
workflow scores the ambiguity of a phase inline, in the session that follows it, and records the outcome; adding an agent
only to give the rule a name would add a contribution that nothing spawns and would change the roster an extension keeps
in step with the workflow it models.

## Decision

- **D1. A judge is a word or `prompt:<id>`.** `<id>` is the id of a prompt of **the rule's own extension**. The registry
  checks it against the extension's registered prompts (always complete: no peer is read; a prompt the surfaces check
  drops, W021, is not registered, so it is no judge) and a missing one is W112, as an unknown agent is. A peer's prompt is not a judge: a prompt is the peer's workflow, and agents keep their peers.
- **D2. The registry keeps the string as written.** The rule's judge is `prompt:spec_phase`; `set_verdict` compares
  strings and already demands a writing command of the rule's extension as the origin, so it needs no change. The
  verdict's member reads `judge "prompt:spec_phase@1.0.0"`.
- **D3. A rendered prompt lists the rules it judges.** The prompt payload gains `judged`: each rule it judges, with its
  outcomes and passing outcomes, the analogue of the section an agent's body lists. Absent while the prompt judges none,
  so no prompt a project has changes.
- **D4. Freshness is unchanged.** The basis, the digests and staleness do not depend on who judged.

## Rejected

An agent added for the purpose (it breaks the roster's parity with the workflow). A peer's prompt as a judge (the peer
would be judging a rule it did not declare). A new `judged.by` field (the judge string already says what judges).

## Consequences

The grammar of one string and one registry check; one optional key of a prompt payload. ADR 0290's D1 reads "an agent or
a prompt of the extension". A rule whose prompt a later release drops is W112 again, and its verdicts stay as written.
