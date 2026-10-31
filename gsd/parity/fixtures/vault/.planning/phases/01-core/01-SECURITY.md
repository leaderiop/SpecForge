---
phase: "01"
slug: core
status: draft
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 1
asvs_level: 1
created: "2026-10-30"
---

# Phase 1 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| process → vault.bin | the sealed file is on a disk other users can read and write | encrypted secrets |
| shell → process | the secret is typed as an argument and the key is an environment variable | secrets and the key, in the clear |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-01-01 | Information disclosure | src/vault.js seal | high | mitigate | AES-256-GCM under a key that never reaches the disk | closed |
| T-01-02 | Information disclosure | src/cli.js argv | high | mitigate | accept the secret on stdin instead of argv | open |
| T-01-03 | Information disclosure | src/cli.js list | low | accept | list prints names only, never values | closed |

*Status: open · closed · open — below {block_on} threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-1 | T-01-03 | the names are not secret; the values are never printed | maintainer | 2026-10-30 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

## Security Audit 2026-10-29

| Metric | Count |
|---|---|
| Threats found | 3 |
| Closed | 1 |
| Open | 2 |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [ ] `threats_open: 0` confirmed
- [ ] `status: verified` set in frontmatter

**Approval:** pending
