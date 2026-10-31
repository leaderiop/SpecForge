# A command and a prompt read their extension's state, and the SDK exports TriggerKind

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR
0291](0291-an-extension-contributes-mcp-prompts.md), [ADR
0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md). Takes extension protocol 1.13.0.

## Context

A command and a prompt could declare `reads_state` and the host sent the `StateView`, but neither `CommandCall` nor
`PromptCall` read it, so `/gsd-workstreams status` could not name the active workstream. The SDK exported neither
`Trigger` nor `TriggerKind`, so a pass compared the kind by its debug text.

## Decision

- **D1. `state()`.** `CommandCall::state() -> Option<&StateView>` and `PromptCall::state() -> Option<&StateView>` return
  the view the host sent. `None` when the declaration does not `reads_state`. `StateView` joins the prelude.
- **D2. `TriggerKind`.** The SDK re-exports `Trigger` and `TriggerKind` and the prelude names them.

## Rejected

A typed per-key getter (the guest owns its keys); `PassInput::is_gate()` (more surface, same fact).

## Limits

A guest that did not declare `reads_state` sees `None`, never an empty view.

## Consequences

The accessor reads what the wire already sends; the only new surface is in the SDK.
