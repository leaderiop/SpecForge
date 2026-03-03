---
name: spec-compliance
description: "Sub-orchestrator for the compliance/ directory in a spec. Manages GxP compliance mapping structure including GAMP 5 classification, ALCOA+ mapping, FMEA summary, and qualification protocol coverage. Delegates to gxp-spec-review for regulatory audit and gap analysis."
---

# Spec Compliance (Sub-Orchestrator)

Sub-orchestrator for the `compliance/` directory in a spec. Manages the directory structure, cross-cutting GxP reference table, and per-package compliance content. Delegates to **gxp-spec-review** for regulatory audit and gap analysis.

## When to Use

- Creating a `compliance/` directory for a GxP-regulated package
- Adding or updating GxP compliance mapping
- Auditing compliance document completeness
- Reviewing the split between cross-cutting and per-package content

## Delegation

| Task | Delegate To |
|------|-------------|
| Reviewing specs for GxP regulatory compliance | **gxp-spec-review** |
| Gap analysis against FDA 21 CFR Part 11, EU GMP Annex 11 | **gxp-spec-review** |
| ALCOA+ data integrity audit | **gxp-spec-review** |

**This skill owns:** directory structure, cross-cutting reference table format, per-package vs shared methodology split, and compliance file template.

## Directory Structure

```
compliance/
  index.yaml                    # Manifest of all compliance files
  gxp.md                        # Governance index with GAMP 5, ALCOA+, FMEA summary
```

### index.yaml Schema

```yaml
kind: compliance
package: "@hex-di/<name>"
entries:
  - id: COMP-001
    file: gxp.md
    title: GxP Compliance
    status: active              # active | draft | deprecated
    regulations:
      - "21 CFR Part 11"
      - "EU GMP Annex 11"
      - "GAMP 5"
```

**Rules:**
- Every `.md` file in `compliance/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk

## File Template

```markdown
---
id: COMP-NNN
kind: compliance
title: "GxP Compliance"
status: active
regulations: ["21 CFR Part 11", "EU GMP Annex 11", "GAMP 5"]
---

# @hex-di/<package> -- GxP Compliance

## Cross-Cutting GxP Framework

This document applies the shared GxP methodology maintained in `spec/cross-cutting/gxp/`.

| Cross-Cutting Document | Methodology Applied |
|---|---|
| [01 -- Regulatory Framework](...) | 21 CFR Part 11, EU GMP Annex 11 |
| [02 -- GAMP 5 Classification](...) | Category 5 classification criteria |
| [03 -- ALCOA+ Mapping](...) | Package-specific feature mapping |
...

---

## GAMP 5 Software Classification

<Category 5 justification.>

## ALCOA+ Principle Mapping

| Principle | Library Feature | Requirement IDs |
|-----------|----------------|-----------------|
...

## FMEA Summary

<Risk level counts, highest RPN; link to risk-assessment/.>

## Qualification Protocol Coverage

| Protocol | Tests | Reference |
|----------|-------|-----------|
...
```

## Cross-Cutting vs Per-Package Content

### Shared methodology (DO NOT duplicate)

Lives in `spec/cross-cutting/gxp/` — 10 files covering:
- `01-regulatory-framework.md` — 21 CFR Part 11, EU GMP Annex 11
- `02-gamp5-classification.md` — GAMP 5 category criteria
- `03-alcoa-mapping.md` — ALCOA+ principle definitions
- `04-risk-management.md` — ICH Q9 methodology
- `05-fmea-methodology.md` — FMEA scoring and RPN calculation
- `06-iq-oq-pq.md` — Qualification protocol definitions
- `07-rtm.md` — Requirements Traceability Matrix methodology
- `08-change-control.md` — Change control procedures
- `09-data-retention.md` — Data retention policy
- `10-supplier-assessment.md` — Supplier assessment criteria

### Per-package content (lives in compliance/)

- Package-specific FMEA failure modes
- Package-specific ALCOA+ feature mappings
- Package-specific RTM entries
- Package-specific test protocol details

**Rule:** Never duplicate shared methodology. Always reference the cross-cutting document.

## Cross-References

```markdown
# From compliance to cross-cutting GxP:
See [regulatory framework](../../../cross-cutting/gxp/01-regulatory-framework.md).
See [FMEA methodology](../../../cross-cutting/gxp/05-fmea-methodology.md).

# From compliance to risk assessment:
See [risk-assessment/](../risk-assessment/).

# From compliance to test strategy:
See [process/test-strategy.md](../process/test-strategy.md).
```
