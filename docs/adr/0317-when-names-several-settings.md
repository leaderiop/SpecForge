# `when` names several settings, all of them on

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md), [ADR 0288](0288-an-extensions-settings-are-typed-and-set-in-specforge-json.md). Takes extension protocol 1.13.0.

## Context

Upstream hides a skill in any disabled cluster; `when` named one setting.

## Decision

- **D1.** Every `when` (agent, skill, slash command, hook, prompt) is a list of bool settings, all of which must be on. It reads a string or a list, and writes nothing when empty, a string when there is one, a list otherwise, so existing declarations are byte-identical. The SDK's `.when(s)` appends.
- **D2. `effort`.** Claude Code's slash skill does not write `effort`: upstream omits it on purpose (#3151). The declaration keeps it for other hosts.

## Rejected

An `any` form; writing `effort`.

## Limits

A 1.12 host cannot load a guest whose `surfaces` write a `when` list: its `when` is an `Option<String>`, and a list fails the category's decode (`DescribeFailed`), not one item (W021). `@gsd/core` declares lists (`gsd-help`, `gsd-update`, `gsd-review`, `gsd-ns-ideate`), so it needs SpecForge 1.13.0 or later. Pinned by `a_when_list_is_not_a_1_12_surface`.

## Consequences

Every `when` consumer takes a slice.
