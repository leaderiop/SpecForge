---
name: spec-risk-assessment
description: "Author risk assessment files in a spec's risk-assessment/ directory using FMEA (Failure Mode and Effects Analysis). Each file documents failure modes with FM-XX-NNN IDs, RPN methodology, severity/occurrence/detection scoring, and mitigation controls. Use when assessing invariant risks, creating FMEA entries, or auditing risk-to-invariant traceability."
---

# Spec Risk Assessment

Rules and conventions for authoring **risk assessment files** in a spec's `risk-assessment/` directory using FMEA (Failure Mode and Effects Analysis) with formal failure mode IDs.

## When to Use

- Assessing risk for each invariant in a spec
- Creating new FMEA failure mode entries
- Auditing risk-to-invariant traceability
- Reviewing risk levels and mitigation controls

## Directory Structure

```
risk-assessment/
  index.yaml                    # Manifest of all risk assessment files
  FM-XX-001-<name>.md           # One file per failure mode (or grouped)
  FM-XX-010-<name>.md
  ...
```

### index.yaml Schema

```yaml
kind: risk-assessment
package: "@hex-di/<name>"
infix: XX
methodology:
  rpn_formula: "Severity x Occurrence x Detection"
  scale: "1-5 each"
  risk_levels:
    high: ">= 50"
    medium: "20-49"
    low: "< 20"
entries:
  - id: FM-XX-001
    file: FM-XX-001-graph-mutation.md
    title: Graph Mutation Failure
    status: active              # active | draft | deprecated
    invariant: INV-XX-1
    risk_level: High
    rpn: 60
  - id: FM-XX-002
    file: FM-XX-002-session-leak.md
    title: Session Memory Leak
    status: active
    invariant: INV-XX-2
    risk_level: Medium
    rpn: 36
```

**Rules:**
- Every `.md` file in `risk-assessment/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- Every entry MUST link to an invariant

## File Template

```markdown
---
id: FM-XX-NNN
kind: risk-assessment
title: "<Failure Mode Title>"
status: active
fm_range: "NNN--NNN"
invariants: [INV-XX-N]
---

# FM-XX-NNN: <Failure Mode Title>

## Failure Mode

<Description of what can go wrong.>

## Scoring

| Factor | Score | Justification |
|--------|-------|---------------|
| Severity | N | <Why this severity> |
| Occurrence | N | <Why this occurrence likelihood> |
| Detection | N | <Why this detection difficulty> |
| **RPN** | **NN** | **<Risk Level>** |

## Mitigation

<Compensating controls that reduce the risk.>

## Test Coverage

| Test Type | File | What It Verifies |
|-----------|------|-----------------|
| Unit | tests/unit/<name>.test.ts | <verification> |
| GxP Integrity | tests/gxp/<name>.test.ts | <verification> (High risk only) |
```

## RPN Methodology

Risk Priority Number = Severity x Occurrence x Detection (each on a 1-5 scale).

| Risk Level | RPN Range | Required Response |
|-----------|-----------|-------------------|
| **High** | >= 50 | Dedicated test (GxP integrity test for GxP packages), mutation testing >= 80% |
| **Medium** | 20-49 | Standard test coverage per risk-level targets |
| **Low** | < 20 | Explicit prose justification for low classification |

## Content Rules

1. **YAML frontmatter** — Every risk assessment file MUST start with `---` frontmatter containing `id`, `kind: risk-assessment`, `title`, `status`, `fm_range`, `invariants`. The `**Invariant:**` line is REMOVED from prose — that metadata lives in frontmatter.
2. **Unique IDs** — Every `FM-XX-NNN` ID is unique across the entire spec.
2. **Invariant linkage** — Every failure mode links to the invariant it targets.
3. **High-risk test requirement** — Every High-risk invariant needs a dedicated test.
4. **Low-risk justification** — Every Low-risk classification needs an explicit prose justification.
5. **Compensating controls** — Residual risks must have compensating controls documented.

## Cross-References

```markdown
# From risk assessment to invariants:
**Invariant:** [INV-XX-1](../invariants/INV-XX-1-<name>.md)

# From traceability to risk assessment:
| INV-XX-1 | ... | High | FM-XX-001 |

# From compliance to risk assessment:
## FMEA Summary — <risk level counts, highest RPN>
```
