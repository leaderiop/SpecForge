<!-- Ported from gsd-core v1.19.0 gsd-core/references/research-documentation-lookup.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
When you need library or framework documentation, check in this order:

1. If Context7 MCP tools (`mcp__context7__*`) are available in your environment, use them:
   - Resolve library ID: `mcp__context7__resolve-library-id` with `libraryName`
   - Fetch docs: `mcp__context7__query-docs` with `libraryId` (the ID from step 1) and `query`

2. If Context7 MCP is not available (custom subagents cannot see project-scoped
   `.mcp.json` servers — they only inherit user-scoped `~/.claude/mcp.json`, so a
   context7 server configured at the project scope is invisible to spawned
   agents), use the CLI fallback via Bash:

   Step 1 — Resolve library ID:
   ```bash
   # ctx7 is the user's own program, not part of SpecForge or @gsd/core: exit 127 means it is not installed
   specforge run -- ctx7 library <name> "<query>"



   ```
   Step 2 — Fetch documentation:
   ```bash
   # ctx7 is the user's own program, not part of SpecForge or @gsd/core: exit 127 means it is not installed
   specforge run -- ctx7 docs <libraryId> "<query>"



   ```

Exit 127 means ctx7 is not installed: say so once, then read the library's official docs with the
host's web fetch. Never install ctx7 or download it with npx: that silently runs an unverified
package from the registry; the user may install it after checking the package.
