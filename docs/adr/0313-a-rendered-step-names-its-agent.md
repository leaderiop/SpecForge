# A rendered step names its agent

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR
0291](0291-an-extension-contributes-mcp-prompts.md). No protocol change (host-only).

## Context

A rendered step carried no agent, so `@gsd/core` named it in the title ("(gsd-planner)").

## Decision

- **D1.** `RenderedStep.agent` is the id of the agent the step declares, as the registry stores it (the id an installed
  agent file carries), only for an agent the registry kept. It is absent otherwise. Key order: `id, title, agent, tool,
  cli, prompt, gate`.

## Rejected

Leaving the agent in the title.

## Limits

A step naming an unknown agent has no `agent`.

## Consequences

The prompt payload gains a key; goldens move.
