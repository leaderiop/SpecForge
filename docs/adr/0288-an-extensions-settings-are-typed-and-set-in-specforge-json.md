# An extension's settings are typed and set in specforge.json

**Status:** accepted (2026-10-27, architecture round 31, plan K6). Extends [ADR 0012](0012-one-extension-declaration.md),
[ADR 0037](0037-the-sandbox-is-limits-the-host-enforces.md), [ADR 0239](0239-the-call-target-takes-its-own-arguments.md)
and [ADR 0286](0286-a-diagnostic-code-is-namespaced-by-its-extension.md).

A feature flag was a boolean an extension declared and nothing read; `specforge.json` had no key to set one. GSD's
configuration is about 270 typed keys, preset by a profile that also raises rule severities, with secrets kept out of
the file.

## D1. An extension declares settings, presets and secrets

The `settings` describe category holds three item kinds. A **setting** has a dotted name or a family (`*` segments,
each optionally closed by a key list), a K3 field type (`bool`, `integer` with bounds, `string`, `enum`,
`string_list`, `enum_list`, `block`, `block_list`), a JSON default or none, a description and the codes it gates. A
**preset** maps each value of an enum or bool setting, the extension's own or a peer's, to settings of the extension
it presets and codes of the extension it raises. A **secret** names the environment variable it is read from. A
feature flag is read as a bool setting. A bad item is W021 and costs that item.

## D2. specforge.json sets them, keyed by extension, by flat dotted name

An unknown name, a wrong type, a value outside its bounds or vocabulary, or `null` on a defaulted setting is E086 at
the key; a secret written there is E087, and no diagnostic echoes a written string; settings for an extension not
enabled are W166.

## D3. One resolution, with its origin

Each setting is explicit, else preset, else default, else unset, resolved once per environment load by a pure
function of the config text and the declarations. `specforge settings` and `specforge.settings` show each value and
its origin.

## D4. Settings gate and raise codes in the project's report

While a bool setting is false, its gated codes are reported by no source. A selected preset raises its codes, upward
only. Both apply where the project's report is assembled, and in analyze's findings, before a surface's strict.

## D5. Every call but handshake and describe carries the extension's own settings

The host splices `settings` into the call input where bytes cross; an MCP tool gets it as a reserved argument the SDK
takes out. A call stays a function of its input. No secret value enters a guest: commands and tools see whether a
secret is set (prompts too, since ADR 0291).

## Consequences

Protocol minor 1.8: the category, the `settings` key, the reserved argument. The shipped schema types `settings`;
`specforge schema --config` writes a project's; the language server serves `specforge.json`.

## Would reopen it

- A setting whose value depends on another extension's setting other than through a preset.
- A guest that must read a secret's value (it would need a capability the sandbox does not give).
- A per-code severity override written by the user, or a lowering one.
