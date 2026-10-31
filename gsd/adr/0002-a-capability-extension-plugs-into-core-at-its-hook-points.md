# A capability extension plugs into @gsd/core at its hook points through typed commands

**Status:** accepted (2026-10-31, @gsd/security round, plan 16). Extends [ADR gsd/0001](0001-one-cargo-workspace-built-on-demand.md).

## Context

Upstream gsd-core v1.18.0 splits GSD into a core and capabilities. A capability is a `capability.json` of steps,
contributions and gates at named points (`verify:post`, `plan:pre`, `ship:pre`, ...); `gsd-tools loop render-hooks
<point>` renders the active ones into a workflow, and `gsd-tools check predicate` evaluates a gate. `@gsd/core`
(r34-r44) ports the workflows with 22 stubs, `X_HOOKS_JSON="[]"`, where upstream ran `loop render-hooks`: the loop
capability is not part of it. `@gsd/security` is the first capability extension, and `@gsd/codebase`, `@gsd/ops`,
`@gsd/ai` and `@gsd/ui` follow, so the shape is decided once. The kernel needs no change for it (routes are keyed by
`(short, cli_name)`, enhancements, `scoped_by`, `human` commands and peer-scoped presets are built); the evidence rows
are in plan 16, "Kernel needs".

## Decision

### D-S-1 (no kernel change)
** See "Kernel needs". The ship gate is a verb the workflow runs, as upstream's is.

### D-S-2 (the crate layout; the template)
**

```
gsd/
├── Cargo.toml          members = ["kit", "core", "security"]
├── kit/                gsd-kit (rlib only, no guest): the kind DSL, the prompt pipeline pieces, agent attribution,
│                       the capability-hook renderer (T14); no assets, no export
├── core/               gsd-core, depends on gsd-kit
└── security/           gsd-security (cdylib + rlib), depends on gsd-kit; never on gsd-core
    ├── Cargo.toml      [package.metadata.gsd] tracks = "gsd-core@1.18.0"
    ├── README.md       status, model, rosters (no separate contributions/ folder: one command, one agent)
    ├── spec/README.md  the model: kinds, codes (a table T25 parses), settings, commands, hooks
    ├── LICENSE-GSD     upstream's MIT notice (copy of core's)
    ├── assets/         agents/gsd-security-auditor{,.compact}.md, commands/gsd/secure-phase.md,
    │                   gsd-core/workflows/secure-phase.md, capabilities/security/capability.json
    ├── src/            lib.rs, kinds.rs, settings.rs, codes.rs, passes.rs, commands/{hooks,gate,threat_ids,record,accept}.rs,
    │                   prompts.rs, agents.rs
    └── tests/          declaration.rs, settings_roster.rs, port.rs, accounting.rs, parity_roster.rs, parity_rules.rs,
                        cli/ (black-box; `#[path]` includes core's harness), golden/, projects/
```

Why a `kit` crate and not `gsd-core` as a dependency: `component_guest!` generates the component's `call` export
(`crates/runtime/extension-sdk/src/guest.rs:119-158`), so linking core's rlib into another guest exports it twice.
Why not `#[path]` includes in `src/`: five extensions would include core's files and break on core's first `crate::`
use. Tests do use `#[path]`, as core's own tests already do (`gsd/core/tests/port_workflows.rs:9`). Decision **U2**.

### D-S-3 (one hooks command per capability; core asks all five)
** `<cap>_hooks <point>` is the contract every
capability extension implements (ADR gsd/0002). The renderer is `gsd_kit::capability::render(capability_json,
settings, point)`, a port of the per-capability part of `loop-resolver.cts` render-hooks (`when` against the
extension's settings, `configValues` filled, `skippedHooks` for an off `when`). `shortcut:` core's line spawns
`specforge` once per capability name (five, ~0.1-0.3 s each with a warm compile cache); upgrade to one call when a
kernel round lets a command read the served commands.

### D-S-4 (threats live in the plan)
** A threat is scoped by its plan: the planner writes it beside the plan's tasks,
`T-03-SC` repeats per plan as upstream, and the cross-plan uniqueness is `E101`, faithful to upstream's own parser
(phase-scoping would make a repeated `-SC` an `E002` and drop it).

### D-S-5 (statuses are written once, the counts are computed)
** The auditor's verdict sets `threat.status` through
`record_security_audit` (single writer, #2119); `threats_open`, `blocking` and `status` are computed so a changed
`block_on` or an accepted risk takes effect at once. Judged mitigations (a verdict that goes stale when a cited file
changes) are gsd-plus and not built (What stays open).

### D-S-6 (the fixture)
** A new fixture `gsd/parity/fixtures/vault/` (a tiny Node CLI, `node:test`), not the taskboard
(new plan text moves its goldens). Its `.planning/` is written by hand from upstream's templates: phase 01 with two
plans whose registers both claim `T-01-02` (the duplicate) and `T-01-SC`, summaries (one with Threat Flags) and an
`01-SECURITY.md` (state A: one open high threat, one accepted risk, one trail block); phase 02 with one plan without a
register and its summary, no SECURITY.md (state B, retroactive). `record-security.sh` runs upstream's verbs over it and
fails if one errors, so the verbs vouch for the hand-written files. Decision **U3**.


### The command contract

`<cap>_hooks <point>` (`specforge gsd <cap>-hooks <point> --format json`) is the contract every capability extension
implements. It answers `{activeHooks, skippedHooks}` as upstream's `render-hooks --raw` does. A gate verb answers
`{block, message, details}` and exits 0 on a block, as `loop-hook-dispatch.md` says; the workflow reads `block`.
`@gsd/core`'s capability points name the five capabilities (`ai`, `codebase`, `ops`, `security`, `ui`) so a later
capability extension needs no core edit. A capability extension declares `@gsd/core` as a required peer, and its model
override and agent variant are presets following core's (`model_profile`, `workflow.compact_content`).

## Limits

- The `shortcut:` of D-S-3: one `specforge` spawn per capability name; one call when a kernel round lets a command read
  the served commands.
- The ship gate is agent-run, as upstream's: `security_gate` is the check the workflow runs, `/gsd-ship` itself is not
  refused (decision U1).

## Rejected

- A kernel "gate contribution" now (an extension adds its codes to a peer's gated command): more than upstream does.
- `gsd-core` as a dependency of the capability crates: `component_guest!` would export `call` twice.
- `#[path]` includes of core's files in a capability's `src`: they break on core's first `crate::` use.
- Phase-scoped threats: a repeated `T-03-SC` would be an `E002` and be dropped.
- Extending the taskboard fixture: new plan text moves its goldens.

## Consequences

Each later capability extension is a crate under `gsd/` depending on `gsd-kit`, its `<cap>_hooks` command, its settings
and presets, a section of ACCEPTANCE and a fixture. `gsd/kit` has no guest and no assets; a guest in it is a defect.
