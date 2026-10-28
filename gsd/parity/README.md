# Parity suite

**Status: being designed; nothing is built yet.** What makes "1:1 with GSD, version by version" a check rather than a hope.

| Check | Proves |
|---|---|
| roster | every command, agent, hook, capability and config key of the tracked gsd-core is contributed by one `@gsd/*` extension or listed as not applicable with a reason |
| scenario | a scripted scenario run by real gsd-core and by SpecForge `@gsd/*` (profile `gsd-compat`) yields the same graph: GSD's `.planning/` output is imported, then compared entity by entity, derived values included |
| rules | every rule in [rules.md](../core/spec/rules.md) fires on its fixture and stays silent on its counterpart |
| upgrade | for a new gsd-core release, the diff of its inventory manifest, `capability.json` files, templates, prompts and gate modules against the tracked one, written as a parity backlog |

`tracks` (the crate's `[package.metadata.gsd] tracks = "gsd-core@<version>"` and the handshake keyword `tracks:gsd-core@<version>`) moves only when roster, scenario and rules pass on that version. The fixture projects and
the version matrix live here.
