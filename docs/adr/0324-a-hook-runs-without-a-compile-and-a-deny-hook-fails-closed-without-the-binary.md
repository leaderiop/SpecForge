# A hook runs without a compile and a deny hook fails closed without the binary

**Status:** accepted (2026-10-31, kernel round K20c, plan 15). Extends [ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md). Takes extension protocol 1.14.0.

## Context

`specforge hook` compiled the whole project for every event, though `hook::run` reads only the registries, settings,
state and, when declared, the graph and revisions. And a line `specforge hook …` exits 127 when `specforge` is not on
the host's PATH; all three hosts treat any non-zero exit but 2 as a non-blocking failure, so the spec guard was
silently skipped.

## Decision

- **D-K20c-8.** `specforge hook` loads the environment, looks the route up in its registries, and builds the view over
  `CompiledProject::unread` when the hook declares neither `reads_graph` nor `writes` and is not `core.spec_guard`;
  otherwise it compiles in that same load. An unreadable event uses `unread` too.
- **D-K20c-9.** A rendered hook whose policy is `deny` (`on_failure: deny`, and `core.spec_guard`) runs
  `command -v specforge >/dev/null 2>&1 || { echo '…' >&2; exit 2; }; specforge hook … # specforge-install`. Exit 2 with
  the reason on stderr blocks on all three hosts. A hook whose policy is `allow` keeps the plain line and fails open,
  as upstream's hooks do. Install recognises both prefixes as its own, so an old install is replaced.

## Rejected

Refusing install without `specforge` on PATH (install cannot know the host's PATH); the binary's absolute path
(goldens become machine-specific); Cursor's `failClosed: true`.

## Consequences

Protocol 1.14.0 is unaffected (no wire change). The guarded line is the only golden change for deny hooks.
