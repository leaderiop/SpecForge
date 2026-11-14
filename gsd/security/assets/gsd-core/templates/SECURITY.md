<!-- Ported from gsd-core v1.19.0 gsd-core/templates/SECURITY.md, MIT, Copyright (c) 2026 Open GSD; see gsd/security/LICENSE-GSD -->
# Phase {N} — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

A phase's security audit is entities: its `security_audit` (`<phase>/NN-SECURITY`), the `trust_boundary` and `threat`
entities of its plans, one `accepted_risk` per accepted risk and one `audit_run` per audit that changed the counts.
`specforge schema --kind security_audit` (likewise `trust_boundary`, `threat`, `accepted_risk` and `audit_run`) lists
the fields. Nothing writes them by hand: `specforge gsd record-security-audit` records a verdict, a person's
`specforge gsd accept-risk` an accepted risk, and the audit's `threats_open` and `status` are computed from the threats.

*Status: open · closed · open — below {block_on} threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

*Accepted risks do not resurface in future audit runs.*

## Sign-Off

- [ ] All threats have a disposition (mitigate / accept / transfer)
- [ ] `threats_open: 0` confirmed
