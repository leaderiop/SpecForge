# `@gsd/security`

**Status: built, 1.0.0, tracking gsd-core 1.18.0** (plan 16). GSD's security capability: the threat model and
`NN-SECURITY.md` as kinds, `/gsd-secure-phase` and `gsd-security-auditor`, the advisory and the gate the ship workflow runs
for a phase with open threats at or above `workflow.security_block_on`. Peer: `@gsd/core`. Shape and decisions:
[ADR gsd/0002](../adr/0002-a-capability-extension-plugs-into-core-at-its-hook-points.md).

## What is here

- The model: [spec/README.md](spec/README.md) (kinds `trust_boundary`, `threat`, `security_audit`, `accepted_risk`,
  `audit_run`; computed values; the codes `gsd.security/E101` and `W301`; the four settings; the hooks).
- Commands (`src/commands/`): `security_hooks <point>`, `threat_ids`, `security_gate`, `record_security_audit`,
  `accept_risk` (human-attested).
- The prompt `secure_phase` and its slash `/gsd-secure-phase` (`src/prompts.rs`), the agent `gsd-security-auditor` in
  a full and a compact body (`src/agents.rs`). Ported assets are under `assets/`, MIT ([LICENSE-GSD](LICENSE-GSD)).
- Tests: `tests/` (declaration, ports, accounting of 74 upstream cases, parity roster and rules), `tests/cli/` (through
  the `specforge` binary, with core loaded), goldens under `tests/golden/`. The parity scenario compares the vault
  fixture (`../parity/fixtures/vault/`) with gsd-tools' answers (`../parity/goldens/vault/`).
- Counts and the divergence tags: [ACCEPTANCE.md](../parity/ACCEPTANCE.md), section `@gsd/security`.

## Divergences from upstream

Named in ACCEPTANCE under Known divergences, by tag: `[blocking-needs-security]`, `[fragment-repointed]`,
`[audit-advisory-every-check]`, `[accept-risk-human]`, `[accepted-risk-not-human-kind]`, `[no-superseded-plans]`,
`[stride-enum]`, `[retro-threat-in-plan]`, `[threats-open-live]`, `[security-absent-off]`, `[verify-post-produces]`,
`[skipped-hooks-empty]`, `[injection-key-explicit]`, `[surface-peer-setting]`.

Two need a word here. `verify:post` keeps upstream's `produces: ["SECURITY.md"]` although plan 16's table said
`security_audit`: the hook text is upstream's and agents read the artifact name from it. `skippedHooks` is always empty,
as upstream's is without `--after-fingerprint`.

## What stays open

- `accepted_risk` is not a human-only kind. The host refuses a human command's plan when it would create a human-only
  entity, so `accept_risk` is human-attested through its `at`, a harness field only it writes, which an
  agent's `specforge new accepted_risk` cannot set (a risk without `at` accepts nothing). A candidate kernel
  round would let a human command create a human-only kind.
- The slash has no `when` gate (`[surface-peer-setting]`); it is available whenever the extension is loaded.
- Codex and Cursor installs are drafts, as core's (goldens only, unrun).
