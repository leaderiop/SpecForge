# A hook fires when a named project file changes

**Status:** accepted (2026-10-31, kernel round K20c, plan 15). Extends [ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md). Takes extension protocol 1.14.0.

## Context

Upstream's `gsd-config-reload.js` runs on Claude Code's `FileChanged` and no event of ours stood for it. `FileChanged`
gets `file_path` (absolute) and `event` (`change`, `add`, `unlink`); its matcher's `|`-separated literal file names in
the working directory build the watch list and filter by base name.

## Decision

- **D-K20c-4.** A new event `file_changed`, appended to `HookEvent::ALL` (11). It takes `paths` (required, literal file
  names relative to the project root: no `*?[{`) and no `tools`. Claude Code renders it as `FileChanged` with the
  paths' base names joined by `|`; the host's filter then matches the changed file's project-relative path against
  `paths`. The input carries the file as `paths[0]` and `change`. Codex and Cursor document no such event:
  `not_rendered`. The answer's `context` is written as `hookSpecificOutput.additionalContext` (upstream's output) and
  `notice` as `systemMessage`.

## Limits

Claude Code's docs read only `systemMessage` from a `FileChanged` answer; `additionalContext` is written as upstream
writes it.

## Rejected

A glob matcher (Claude registers literal names only).

## Consequences

`@gsd/core` ports `gsd-config-reload.js`. Protocol 1.14.0 (D-K20c-10). A 1.13 host refuses a `file_changed` hook (W021).
