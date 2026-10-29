<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/new-project/steps/codebase-map-offer.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**If `needs_codebase_map` is true** (from init — existing code detected but no codebase map):

**Text mode (`workflow.text_mode: true` in config or `--text` flag):** Set `TEXT_MODE=true` if `--text` is present in `$ARGUMENTS` OR `text_mode` from init JSON is `true`. When TEXT_MODE is active, replace every `AskUserQuestion` call with a plain-text numbered list and ask the user to type their choice number. This is required for non-Claude runtimes (OpenAI Codex, Antigravity, etc.) where `AskUserQuestion` is not available.
Use AskUserQuestion:

- header: "Codebase"
- question: "I detected existing code in this directory. Would you like to map the codebase first?"
- options:
  - "Map codebase first" — Run /gsd:map-codebase to understand existing architecture (Recommended)
  - "Skip mapping" — Proceed with project initialization

**If "Map codebase first":**

```
Run `/gsd:map-codebase` first, then return to `/gsd:new-project`
```

Exit command.
