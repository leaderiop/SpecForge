# A command is told the host that calls it

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR
0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md). Takes extension protocol 1.13.0.

## Context

`smart_entry` took `--runtime` to name commands for a host.

## Decision

- **D1.** `CommandInput.host: Option<String>`, the host flag spelling. MCP passes the session's host; the CLI takes
  `--host <host>` on every extension command that does not declare an arg of that name, validated by `Host::from_flag`.
  The trampoline's command fallback passes `--host`. SDK `CommandCall::host()`. A rendered command step's `cli` names it
  too (`specforge gsd smart-entry --host codex`).

## Rejected

The host rewriting a command's answer.

## Limits

`HOST_OPTIONS` is unchanged. The host is the caller's claim, never a trust signal: the CLI takes any of the three hosts
and MCP maps `clientInfo.name`, and nothing decides on it. A per-host hook filter must key on the installed hook line's
own host, never on one a command or a session reports.

## Consequences

A command that declares `host` keeps it as its own argument and is not told the calling host by it.
