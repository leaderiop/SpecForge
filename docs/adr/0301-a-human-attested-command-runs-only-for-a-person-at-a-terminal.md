# A human-attested command runs only for a person at a terminal

**Status:** accepted (2026-10-31, kernel round K20a, plan 13). Extends
[ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md) (D7, who writes a field),
[ADR 0291](0291-an-extension-contributes-mcp-prompts.md) (a gated command) and
[ADR 0298](0298-an-agent-does-not-change-or-delete-what-a-person-wrote.md). Takes extension protocol 1.12.0.

## Context

A person's answer (a checkpoint, a UAT result, a validation approval, a window waiver) is recorded by a command that
takes `--by`. Any caller may type any name, so every gate that reads such an answer (gsd's W404, the sign-off gates)
is an honest-agent gate. The host already knows who calls: a plan from an extension call, MCP, or the command line
with no terminal is an agent's (`flags::agent_facing`); the LSP and a person at a terminal are not. A command's plan
is always `Origin::Command`, so the surface was never passed in, and the descriptor had no way to say "a person's".

## Decision

- **D1. The descriptor says it.** `CommandDescriptor.human: bool` (default false, skipped when false): "Records what
  a person says: the host runs it only for a person at a terminal". The SDK's `CommandBuilder::human()` sets it and
  calls `writes()`.
- **D2. One rule in one place.** `ops::authoring::apply::execute_with` (and `execute`) take a `Surface` after
  `caller`. Before the export is called, a `human` command whose surface is agent-facing is refused with
  `RunError::Gate` carrying `PermissionDenied` and the code `human_only`; nothing runs and nothing is written, and
  `--dry-run` is refused too. The CLI passes `Surface::Cli { interactive }` from its terminal facts; MCP passes
  `Surface::Mcp`.
- **D3. Registry.** A `human` command that does not write is W021 and dropped, so MCP's non-writing path never runs
  one.
- **D4. The host records nothing new.** The `by` a person types at their own terminal is stored as before; only a
  person reaches the command.
- **Protocol.** 1.12.0 (one optional field). A 1.11 host ignores `human`.

## Rejected

A host-stamped identity (git `user.name`, `$USER`: an agent sets them); an environment variable or flag as a test
seam (an agent can use it); an `Asking` question (needs `--format human` and gains nothing over the terminal check);
MCP serving no tool (the agent gets "unknown tool" and no remedy).

## Limits

An agent that fakes a terminal (`script(1)`) is outside the guarantee (ADR 0289 D7's bound: who typed is not proven).
MCP elicitation as a person path is ADR 0110's reopen and is not built.

## Consequences

An agent that recorded an answer now asks the person to run the command. A test reaches the person path through a real
pseudo-terminal on stdin.
