---
name: specforge-failure-modes-dsl
description: "Write failure_mode blocks in .spec DSL files (@specforge/governance plugin). Each failure_mode declares an FMEA entry with FM-{infix}-{n} IDs, severity/occurrence/detection scores (1-10), RPN calculation, cause/effect/mitigation narrative, and post-mitigation reassessment. Use when analyzing what can go wrong with invariants and how bad it would be."
---

# SpecForge Failure Modes DSL

Rules and conventions for authoring **`failure_mode` blocks** in `.spec` files. Failure modes are FMEA (Failure Mode and Effects Analysis) entries — structured risk assessments tied to invariants.

**Requires:** `@specforge/governance` plugin.

## When to Use

- Analyzing what can go wrong with a runtime invariant
- Scoring risk: severity, occurrence, and detection (1-10 scales)
- Computing Risk Priority Numbers (RPN = S x O x D)
- Documenting cause, effect, and mitigation for each failure scenario
- Reassessing risk after mitigation (post_mitigation block)

## Block Syntax

```spec
use invariants/data

failure_mode FM-MS-001 "Write Acknowledged but Lost" {
  invariant  INV-MS-1
  severity   8
  occurrence 2
  detection  3
  rpn        48

  cause      "Crash between ACK and fsync"
  effect     "Silent data loss — user believes write succeeded"
  mitigation "Write-ahead log with fsync before ACK"

  post_mitigation {
    severity   8
    occurrence 1
    detection  2
    rpn        16
  }
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). |
| `invariant` | reference | The invariant this failure mode threatens. Must exist. |
| `severity` | number | Impact if failure occurs (1-10). 10 = catastrophic. |
| `occurrence` | number | Likelihood of failure (1-10). 10 = near-certain. |
| `detection` | number | Ability to detect before user impact (1-10). 10 = undetectable. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `rpn` | number | Risk Priority Number = S x O x D. Auto-computed if omitted; validated if provided. |
| `cause` | string | Root cause or trigger of the failure. |
| `effect` | string | Impact on user or system when failure occurs. |
| `mitigation` | string | Actions taken to reduce risk. |
| `post_mitigation` | block | Reassessed S/O/D scores after mitigation. |
| `refs` | reference list | External references linked to this failure mode. |

### Post-Mitigation Sub-Block

| Field | Type | Description |
|-------|------|-------------|
| `severity` | number | Reassessed severity (1-10). |
| `occurrence` | number | Reassessed occurrence (1-10). |
| `detection` | number | Reassessed detection (1-10). |
| `rpn` | number | Reassessed RPN. Auto-computed if omitted. |

## FMEA Scoring Guide

### Severity (S)

| Score | Impact |
|-------|--------|
| 1-2 | Negligible — minor inconvenience |
| 3-4 | Low — degraded experience, workaround available |
| 5-6 | Moderate — feature unavailable, manual recovery |
| 7-8 | High — data loss or corruption |
| 9-10 | Critical — safety risk, regulatory violation |

### Occurrence (O)

| Score | Likelihood |
|-------|-----------|
| 1-2 | Rare — once per year or less |
| 3-4 | Uncommon — a few times per year |
| 5-6 | Occasional — monthly |
| 7-8 | Frequent — weekly |
| 9-10 | Near-certain — daily |

### Detection (D)

| Score | Detectability |
|-------|--------------|
| 1-2 | Immediate — automated monitoring |
| 3-4 | Quick — detected within minutes |
| 5-6 | Moderate — detected within hours |
| 7-8 | Slow — detected by user reports |
| 9-10 | Undetectable — silent failure |

### RPN Thresholds

| RPN Range | Risk Level | Action |
|-----------|-----------|--------|
| 1-50 | Low | Accept, document, monitor |
| 51-100 | Medium | Mitigate, add monitoring |
| 101-200 | High | Mitigate urgently, add redundancy |
| 201+ | Critical | Block release until mitigated |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `invariant` | `mitigates` | Failure mode threatens this invariant |
| `ref` | `links_to` | External references linked to this failure mode |

### Incoming edges

None. Failure modes are leaf nodes in the traceability chain.

## Writing Rules

1. **One failure mode per invariant threat** — each FM describes one way an invariant can break.
2. **Severity rarely changes after mitigation** — the impact of the failure is inherent; mitigation reduces occurrence and improves detection.
3. **RPN is validated** — if provided, must equal S x O x D. Omit to auto-compute.
4. **Cause-effect-mitigation pattern** — tell a complete story: what triggers → what happens → what we do about it.
5. **Post-mitigation is essential** — always reassess after mitigation to show risk reduction.
6. **Import invariant files** — `use` the files declaring referenced invariants.
7. **High-risk invariants need FMs** — if `INV` has `risk: high` and no FM, `W005` fires.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | `invariant` must resolve to an existing invariant. |
| E002 | No duplicate failure mode IDs. |
| E005 | RPN mismatch — `rpn != severity * occurrence * detection`. |
| E005 | Same check for `post_mitigation` sub-block. |

## Examples

### Data Integrity

```spec
failure_mode FM-MS-001 "Write Acknowledged but Lost" {
  invariant  INV-MS-1
  severity   8
  occurrence 2
  detection  3
  rpn        48

  cause      "Crash between ACK and fsync"
  effect     "Silent data loss — user believes write succeeded"
  mitigation "Write-ahead log with fsync before ACK"

  post_mitigation {
    severity   8
    occurrence 1
    detection  2
    rpn        16
  }
}
```

### Security

```spec
failure_mode FM-MS-005 "Email Uniqueness Bypass via Race Condition" {
  invariant  INV-MS-2
  severity   6
  occurrence 3
  detection  4
  rpn        72

  cause      "Two concurrent requests create users with the same email"
  effect     "Two active users share an email — login ambiguity"
  mitigation "Database-level UNIQUE constraint + serializable isolation"
  refs       [gh.issue:31]

  post_mitigation {
    severity   6
    occurrence 1
    detection  1
    rpn        6
  }
}
```

### Operational

```spec
failure_mode FM-MS-010 "Audit Log Corruption" {
  invariant  INV-MS-3
  severity   9
  occurrence 1
  detection  5
  rpn        45

  cause      "Disk corruption or software bug overwrites existing audit entries"
  effect     "Regulatory non-compliance — audit trail integrity compromised"
  mitigation "Append-only table with no UPDATE/DELETE grants, WAL archiving, daily checksums"

  post_mitigation {
    severity   9
    occurrence 1
    detection  2
    rpn        18
  }
}
```

## What NOT to Do

- Do not write failure modes without the `@specforge/governance` plugin installed
- Do not provide an incorrect RPN — it must equal severity x occurrence x detection (E005)
- Do not skip post_mitigation — it demonstrates the value of the mitigation
- Do not change severity in post_mitigation without justification — the impact is inherent
- Do not use scores outside 1-10 range
- Do not reference invariants from other files without a `use` import
