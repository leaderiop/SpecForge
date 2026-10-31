# Vault (fixture)

An encrypted store for secrets on the command line: `VAULT_KEY=<hex> node src/cli.js set <name> <secret>`, `get <name>`,
`list`. It exists to give the security parity suite a GSD 1.18.0 project with threat models to read; see
[gsd/parity/README.md](../../README.md) for how the goldens are recorded (`record-security.sh`).

- `.planning/` is hand-written from upstream's templates (plan 16, decision U3); `record-security.sh` runs
  `gsd-tools` over it and fails if a verb errors, so the verbs vouch for the files.
  - Phase 1 (executed): two plans whose `<threat_model>` registers both claim `T-01-02` (the duplicate that
    `init execute-phase` reports as `threat_id_duplicates`) and `T-01-SC` (shared by design); a summary with a
    `## Threat Flags` table; `01-SECURITY.md` in state A: three threats (one closed, one open and high, one accepted
    risk `AR-1`) and one audit trail block (2026-10-29: 3 found, 1 closed, 2 open, so the recorded
    `verification.append-audit` of 3/2/1 appends).
  - Phase 2 (planned): one plan without a threat register, no `SECURITY.md` (state B, the retroactive case).
- `src/`, `test/`: what phase 1 built. `npm test` runs `node --test`.
- `spec/`: the SpecForge twin: the phases and plans, and the security model's entities for them (trust boundaries,
  threats, the audit, the accepted risk and the audit run). It checks with `@gsd/core` and `@gsd/security` loaded and
  reports one thing, the duplicate `T-01-02` (`gsd.security/E101`, once per claim) that the fixture exists to carry.
  A threat's `status` and `evidence` and the audit run's counts are harness fields (written by
  `record-security-audit`, never by hand); the twin holds them as the audit left them, and `specforge-harness` vouches
  for each, as the taskboard's verdicts are. The threats' statuses are `01-SECURITY.md`'s (`T-01-02` of plan 01-02 open,
  the rest closed), so the audit's computed `threats_open` is 1, the frontmatter's.
- `.planning/config.json` sets `security.injection_blocking: false` explicitly: gsd-tools errors on a key that is unset.
