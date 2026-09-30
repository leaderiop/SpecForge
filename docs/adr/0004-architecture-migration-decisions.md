# One project, four surfaces: the migration's decisions

**Status:** accepted (2026-09-30)

The architecture review in `.planning/architecture/` found that the CLI, `specforge watch`, the LSP
and the MCP server each assemble the compiler on their own, and so drift apart. It plans six
deepenings: a compiled-project module, one coverage rule, shared operations, an MCP tool table,
one registry build, and a diagnostic catalog without gaps. Each plan stops at decisions that change
linked obligations or user-visible output. These are the answers; the spec is edited to match in
the step that needs each one.

## Compiled project (plan 01)

- **D1-a** An import cycle is **W113**, as the code reports; the spec text saying E003 is wrong.
- **D1-b** `exclude` in `specforge.json` applies to `check`, watch, the LSP and MCP alike.
- **D1-c** One diagnostic type with **two presenters** (the CLI's flat JSON and the LSP/MCP shape) for now.
- **D1-d** `CompiledProject` and `ProjectSession` live in a **new crate, `specforge-project`**.

## Coverage (plan 02)

- **D2-a** An entity with zero obligations is **uncovered** even when tests pass: a test that proves
  nothing declared is not proof.
- **D2-b** Union and abstract types are **exempt** from A001, stats and the gate denominator, as they
  are from W004, so `--min 100` is reachable.
- **D2-c** Stats follows the spec: an entity is "declared" when it has a verify statement **or** a
  file-reference field; the figures are named "declared %" and "proof %".
- **D2-d** `inspect.testable` means the kind's testability; a separate `declared` field says whether
  the entity declares obligations.
- **D2-e** A malformed `specforge-report.json` is an **error** in MCP, as in the CLI.
- **D2-f** The coverage rule is a **shared pure crate owned by `@specforge/testing`**, linked by the
  host surfaces (this ADR records it; ADR 0002 stands for the runner split).

## Operations (plan 03)

- **D3-a** The MCP export tool emits **Graph Protocol V2 with the schema**, like the CLI and the resource.
- **D3-b** Installed extensions load from the **lock file**; the `specforge.json` entry is the bare
  name; a local install is labelled **`"local"`**.
- **D3-c** Provider configuration follows the spec's **`ProviderConfig`**; the JSON schema and the
  compiler are updated to it.
- **D3-d** MCP doctor reports **session state that is fresh by construction**.
- **D3-e** `init` accepts a **builtin or resolvable** extension, on both surfaces.
- **D3-f** MCP analyze passes **no proved claims unless prove ran**.

## MCP tool table (plan 04)

- **D4-a** Execution failures are **`isError` tool results**; protocol errors are JSON-RPC errors.
- **D4-b** `infer_session` is a **mutation** tool.

## Registry build (plan 05)

- **D5-a** Wire **I002**, the **I004** keyword hint and **peer-dependency checks** into compile; I005
  follows D3-c; define blocks and grammar contributions are decided after reading their spec.

## Diagnostic catalog (plan 06)

- **D6-a** E047 is **renumbered to a warning** (the guide already documents it as one).
- **D6-b** The canonical repository URL is the public one, **`leaderiop/SpecForge`**; `Cargo.toml` is
  fixed to match.
- **D6-c** R-, R-RES-, F- and W-REG- codes are **catalogued as they are**, not renumbered.
