# Settings are set and unset through `specforge settings`, admitted by the setting's form, and never a secret

**Status:** accepted (2026-10-28, kernel round 35, plan K16). Extends [ADR 0288](0288-an-extensions-settings-are-typed-and-set-in-specforge-json.md)
(typed settings, E086, E087) and [ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md) (the write
lock and the journal). Amends ADR 0288. Takes extension protocol 1.11.0.

## Context

Settings are read from `specforge.json`, and `specforge settings` lists them. A workflow changes configuration (a
profile, a model tier, which skills are surfaced), and an agent that does it writes the file raw: unchecked until the next
load, its layout rewritten, and a credential could be put there. The edit plan writes `.spec` entities only.

## Decision

- **D1. The form.** `specforge settings` stays the listing (`--all`, `--extension`, `--path`, `--format`).
  `specforge settings set <extension> <name> [VALUE]... [--value-json JSON] [--dry-run]` and
  `specforge settings unset <extension> <name> [--dry-run]` write. `extension` is the name the listing shows and `name`
  the flat dotted name or family key as written in `specforge.json`. The value is read by the setting's declared type:
  `string` and `enum` take one word; `integer` one word read as an integer; `bool` `true` or `false`; `string_list` and
  `enum_list` every remaining word as an item, none being `[]`; `block` and `block_list` take `--value-json` only; `--value-json`
  reads its argument as JSON for any type, in place of the VALUE words (`--json` is not a flag of any command: a convention
  test keeps `--format json` the one way to ask for JSON output). A word that does not fit its type is passed through as text so the form's
  own E086 says "must be an integer, found string". `unset` removes the member, so the setting reverts to its preset or
  default (or is unset); it never writes `null`, which a setting with a default refuses. Unsetting what is not written
  succeeds and changes nothing.
- **D2. Admission before any write.** One function takes the extension, the name, the value and the registry, and
  answers the value or a diagnostic: the load's own `secret` check first, then its own `check`, so a load and a `set` can
  never disagree. A refusal is E086 (unknown name, wrong type or bound, an object that is not one) or E087 (a declared
  secret, or a name that looks like a credential and is no declared setting), both `invalid_input`; an extension that is
  not in `extensions` or declares no settings is `extension_not_enabled` with the suggestion `specforge add`. The value
  is never scanned for a secret. Under the lock the file is read again and the splice is admitted again over the new text.
- **D3. The edit preserves the file.** `specforge.json` is spliced as text, with the byte ranges of its members: replace
  a member's value; insert after the last sibling, copying its indentation and the file's newline; create the missing
  `settings` and `settings.<extension>` objects at the end of the nearest existing one; remove a member with its comma. A
  list or block is rendered pretty at the member's depth, a scalar inline. Before anything is staged the new text must
  parse and equal the old document with exactly that one path replaced or removed; otherwise nothing is written. An
  emptied `settings.<extension>` object is left as `{}`. A `specforge.json` the config check refuses (E069) is not edited.
- **D4. The write is one change under the lock and the journal** (ADR 0289), all or nothing, `--dry-run` previewing the
  member's before and after. `specforge.json` joins the files a journal may restore; without it an interrupted write
  would leave a journal that a later write refuses to roll back and every write of the project would be blocked.
- **D5. MCP.** `specforge.settings_set` is a mutation tool over the same write: `extension`, `name`, one of `value` (any
  JSON) or `unset`, and `dry_run`. Its reply names the value, the previous value, whether anything changed and the
  diagnostics. The core surface is 39 tools.

## Rejected

Re-rendering the file from a parsed document (it rewrites the author's layout, and the file is reviewed). A `number`
setting type (nothing asked for one; whole numbers are `integer`). Scanning values for secrets (a value cannot be told from
prose; the name is what the load already judges).

## Consequences

A command with two subcommands, a tool, a config splicer and a journal allow-list entry. ADR 0288's Consequences gain the
pointer: an agent no longer writes the file raw. A hand-formatted `specforge.json` is touched at one member only.
