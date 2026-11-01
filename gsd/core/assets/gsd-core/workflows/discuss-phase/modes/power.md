<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/discuss-phase/modes/power.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
Apply response_language to all user-facing prose — narration between tool calls, status updates, progress notes, and findings included; preserve code, paths, and identifiers.

# --power mode — bulk question generation, async answering

> **Lazy-loaded.** Read this file from `workflows/discuss-phase.md` when
> `--power` is present in `$ARGUMENTS`. The full step-by-step instructions
> live in the existing `discuss-phase-power.md` workflow file (kept stable
> at its original path so installed `@`-references continue to resolve).

## Dispatch

```
Read @~/.claude/gsd-core/workflows/discuss-phase-power.md
```

Execute it end-to-end. Do not continue with the standard interactive steps.

## Summary of flow

The power user mode generates ALL questions upfront into machine-readable
and human-friendly files, then waits for the user to answer at their own
pace before processing all answers in a single pass.

1. Load the phase SPEC (`check_spec`, from the phase's `phase_spec` entity in the graph: `SPEC_ID=$(specforge query "$PHASE_ID" --depth 1 --pick '.nodes[kind=phase_spec].id' | head -1); HAS_SPEC=$([ -n "$SPEC_ID" ] && echo true || echo false)`), then run the same phase analysis (gray area identification) as standard mode, about HOW only when a SPEC locks the requirements
2. Write all questions to
   `{phase_dir}/{padded_phase}-QUESTIONS.json` and
   `{phase_dir}/{padded_phase}-QUESTIONS.html`
3. Notify user with file paths and wait for a "refresh" or "finalize"
   command
4. On "refresh": read the JSON, process answered questions, update stats
   and HTML
5. On "finalize": read all answers from JSON, generate CONTEXT.md in the
   standard format

## When to use

Large phases with many gray areas, or when users prefer to answer
questions offline / asynchronously rather than interactively in the chat
session.

## Combination rules

- `--power --auto`: power wins. Power mode is incompatible with
  autonomous selection — its purpose is offline answering.
- `--power --chain`: after the power-mode finalize step writes
  CONTEXT.md, the chain auto-advance still applies (Read `chain.md`).
