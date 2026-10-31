# A prompt reads the slash commands the project serves

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR
0291](0291-an-extension-contributes-mcp-prompts.md), [ADR
0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md). Takes extension protocol 1.13.0.

## Context

`/gsd-next` and the namespace routers listed commands from a roster in the extension's source, whatever the project
served.

## Decision

- **D1.** `PromptDescriptor.reads_slash_commands`. The host sends `PromptInput.slash_commands: Vec<ServedSlash>`
  (`name`, `extension`, `prompt`, `tool`, `cli`), the list an install writes (`when` applied, sorted by name).

## Rejected

Sending every extension's full declaration.

## Limits

A prompt sees what is served, not what is uninstalled.

## Consequences

`served_slashes(view)` is one function an install and a prompt share.
