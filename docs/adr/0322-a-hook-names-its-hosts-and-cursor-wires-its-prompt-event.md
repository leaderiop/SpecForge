# A hook names its hosts and Cursor wires its prompt event

**Status:** accepted (2026-10-31, kernel round K20c, plan 15). Extends [ADR
0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md). Takes extension protocol 1.14.0.

## Context

Every hook the build holds was rendered for every host, so Cursor got the isolation guard line and `read_guard`
(which returns early on Claude Code) was installed there. Cursor's `user_prompt` was not rendered although the reader
maps `beforeSubmitPrompt`, and Cursor reads `permission`, not the `block` the adapter wrote.

## Decision

- **D-K20c-5.** `HookDescriptor.hosts`, each one of `HOOK_HOSTS = ["claude-code", "codex", "cursor"]` (W021
  otherwise); empty means every host. Install skips a hook whose `hosts` does not name the host (no `not_rendered` row:
  the extension chose it, as with `when`). `specforge hook` for an unnamed host is a no-op that calls nothing. The
  host is the one the install wrote into the line, never one the caller reports (ADR 0318).
- **D-K20c-6.** `user_prompt` renders as Cursor's `beforeSubmitPrompt`. A denied or asked `pre_tool`/`pre_commit` answer
  is `{"permission": "deny", "user_message": <reason>, "agent_message": <reason>}`. `beforeSubmitPrompt` gets no
  `additional_context` (its documented output is `continue` and `user_message`).

## Limits

Cursor has not been run; the shapes are its documentation's of 2026-10-31. Its documented `preToolUse` output has no
`additional_context`, which upstream and this adapter write for advisory answers: unverified either way.

## Rejected

A `not_rendered` row for a hook the extension excluded; Cursor's `failClosed: true` (it also blocks on the empty
answer our allow answers are).

## Consequences

Protocol 1.14.0 (D-K20c-10). A 1.13 host ignores `hosts` and installs the hook everywhere.
