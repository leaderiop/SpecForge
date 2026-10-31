# Install refuses an unusable specforge.json, and uninstall removes only what install created

**Status:** accepted (2026-10-31, kernel round K20a, plan 13). Extends
[ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md) (D3, the host install). No protocol
change.

## Context

A broken `specforge.json` made `specforge install` strip every gsd hook, agent and skill with exit 0 (REVIEW-r44
M12): the compile records config problems and `run()` never read them. Uninstall removed what install did not create
(an emptied container, a directory the user had) and left the empty host root it did create (m20, m21). A host
config of the wrong shape got a bogus byte position, uninstall's hint named `install --force`, and a Codex config with
`mcp_servers = 1` was silently half-installed (m23).

## Decision

- **D1.** Install, uninstall (which reads no extension, so a broken `specforge.json` is fixed before uninstalling), `--dry-run` and `--check` refuse with the first config problem (E069) before anything is
  staged. Any problem, not only one that loads nothing: a mistyped `spec_root` rewrites the guard's paths.
- **D2.** A wrong-shape host config is refused by name: `'{place}' is an array, and install needs an object there`
  (or the reverse); TOML: `'{keys}' holds a value where install needs a table`.
- **D3.** Uninstall's edited-file text names `specforge uninstall {host} --force`.
- **D4. The install record.** Install writes `.specforge/install/<host>.json` = `{"format":1,"dirs":[...],"made":
  {"<file>":[[], ["mcpServers"], ...]}}` (`[]` = the file itself was absent). A strip of an emptied container not in
  `made` keeps its braces; the file goes only when `made` holds `[]`. Uninstall removes the install-namespace dirs plus
  the recorded dirs, the host root included, deepest first. The record is untrusted: a `dirs` entry is kept only if it
  is a layout directory of the host, a `made` key only if the renderer merges it; an unreadable record is no record.

## Rejected

A key in the user's host file; never removing an emptied container (breaks the byte-for-byte round trip); recording
the whole pre-install text.

## Limits

An install made before K20a has no record, and its uninstall keeps more than it did before K20a: the layout directories
(`.claude/agents`, `.claude/skills`, and the like), the emptied containers (`.mcp.json` stays as `{"mcpServers": {}}`)
and a config file install created. A re-install does not write a record: it creates nothing, so there is nothing
honest to record. The way out is to uninstall, delete the leftovers, then install (`an_install_without_a_record_...`
pins the leftovers). `.specforge/` is gitignored, so on
a fresh clone uninstall keeps containers. A container written `{ }` comes back `{}`.

## Consequences

The record is not in `HostPlan.files`, so outcome documents and goldens do not move.
