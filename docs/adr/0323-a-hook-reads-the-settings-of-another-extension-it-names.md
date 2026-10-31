# A hook reads the settings of another extension it names

**Status:** accepted (2026-10-31, kernel round K20c, plan 15). Extends [ADR
0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md).
Amends [ADR 0288](0288-an-extensions-settings-are-typed-and-set-in-specforge-json.md) (a call carries only its
extension's settings: a hook may name others'). Takes extension protocol 1.14.0.

## Context

Upstream's injection scanner blocks a HIGH scan when `security.injection_blocking` is true. That key belongs to
`@gsd/security`, and a call carried "the extension's own settings … and nobody else's".

## Decision

- **D-K20c-7.** `HookDescriptor.reads_settings: Vec<String>`, each `<extension>:<setting>`. W021 when an entry has no
  `:`, an empty side, or names the hook's own extension (it reads those with `Settings::current`). The host sends
  `HookInput.other_settings`: the resolved value of each named setting (defaults included) keyed by the entry as
  written; absent when the extension is not enabled or the setting is unset. Values only, never a secret's presence.
  Hooks only. The member is `other_settings`, not `settings`: the host splices its own `settings` into every call.

## Limits

`@gsd/security` does not exist, so `injection_scanner` stays advisory in every real project; the mechanism is built and
tested.

One extension reads another's non-secret settings without the owner's consent: naming `<extension>:<setting>` in
`reads_settings` is enough. Secrets are never sent.

## Rejected

Sending every extension's settings; extending passes, commands and prompts (no consumer).

## Consequences

Protocol 1.14.0 (D-K20c-10).
