---
name: specforge-constraints-dsl
description: "Write constraint blocks in .spec DSL files (@specforge/governance plugin). Each constraint declares a non-functional requirement with CON-{infix}-{n} IDs, category taxonomy (performance, security, reliability, etc.), priority levels (must/should/may), measurable metrics, and verify statements. Use when defining quality attributes with thresholds."
---

# SpecForge Constraints DSL

Rules and conventions for authoring **`constraint` blocks** in `.spec` files. Constraints declare non-functional requirements — measurable quality attributes with thresholds and verification methods.

**Requires:** `@specforge/governance` plugin.

## When to Use

- Defining performance requirements with measurable thresholds
- Specifying security requirements (encryption, access control)
- Setting reliability targets (uptime, recovery time)
- Documenting compatibility, usability, legal, or operational requirements
- Constraining specific behaviors or protecting specific invariants

## Block Syntax

```spec
constraint CON-MS-001 "API Latency Under Load" {
  category    performance
  priority    must

  metric """
    response_time_p99 < 200ms
    at 1000 concurrent users
    sustained for 5 minutes
  """

  affects     [BEH-MS-001, BEH-MS-002, BEH-MS-003]
  invariants  [INV-MS-1]

  verify load "k6 load test with 1000 VUs for 5 minutes, assert p99 < 200ms"
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). |
| `category` | enum | Quality attribute category (see taxonomy below). |
| `priority` | enum | `must`, `should`, or `may` — RFC 2119 priority. |
| `description` / `metric` | string / triple-string | Quality requirement. `metric` is an alias for quantifiable constraints. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `behaviors` / `affects` | reference list | Behaviors this constraint applies to. Both field names accepted. |
| `invariants` | reference list | Invariants this constraint helps protect. |
| `verify` | verify statement(s) | How to test this constraint. |
| `refs` | reference list | External references linked to this constraint. |

### Category Taxonomy

| Category | Covers |
|----------|--------|
| `performance` | Latency, throughput, resource usage |
| `security` | Authentication, authorization, encryption |
| `reliability` | Availability, fault tolerance, recovery |
| `scalability` | Load capacity, elasticity, data volume |
| `compatibility` | Platform support, backward compatibility |
| `usability` | Accessibility, learnability, error prevention |
| `maintainability` | Modularity, testability, analyzability |
| `portability` | Installability, adaptability, replaceability |
| `legal` | Regulatory compliance, licensing, data residency |
| `operational` | Deployment, monitoring, logging, backup |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `behavior` | `constrains` | Quality requirement applies to these behaviors |
| `invariant` | `constrains` | Quality requirement protects these invariants |
| `ref` | `links_to` | External references linked to this constraint |

### Incoming edges

None. Constraints are leaf nodes in the traceability chain.

## Writing Rules

1. **Measurable metrics** — "p99 < 200ms" not "the system is fast".
2. **Categorize correctly** — use the standard category taxonomy.
3. **Scope to behaviors** — name which behaviors this constraint applies to.
4. **Add verify statements** — describe how to test the constraint.
5. **Priority reflects RFC 2119** — `must` = absolute, `should` = recommended, `may` = optional.
6. **Import behavior and invariant files** — `use` the files declaring referenced entities.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `affects` and `invariants` must resolve. |
| E002 | No duplicate constraint IDs. |
| W006 | Unconstrained behavior — behavior with no security or performance constraint coverage. |

## Examples

### Performance

```spec
constraint CON-MS-001 "API Latency Under Load" {
  category    performance
  priority    must

  metric """
    response_time_p99 < 200ms
    at 1000 concurrent users
    sustained for 5 minutes
  """

  affects     [BEH-MS-001, BEH-MS-002, BEH-MS-003]

  verify load "k6 load test with 1000 VUs, assert p99 < 200ms"
}
```

### Security

```spec
constraint CON-MS-002 "PII Encryption at Rest" {
  category    security
  priority    must

  metric "All PII fields encrypted with AES-256 in the database."

  invariants  [INV-MS-5]
  affects     [BEH-MS-001, BEH-MS-003]
  refs        [gh.issue:55]

  verify audit "Database column inspection confirms encryption"
}
```

### Compatibility

```spec
constraint CON-MS-003 "Runtime Compatibility" {
  category    compatibility
  priority    must

  metric "The CLI runs on Node.js 18+, Bun 1.0+, and Deno 2.0+."

  verify integration "CI matrix tests across all supported runtimes"
}
```

### Legal

```spec
constraint CON-MS-004 "GDPR Data Residency" {
  category    legal
  priority    must

  metric "All EU user data stored in EU-West-1 region. No cross-region replication of PII."

  affects     [BEH-MS-001, BEH-MS-003]
  invariants  [INV-MS-6]

  verify audit "Infrastructure audit confirms data residency"
}
```

## What NOT to Do

- Do not write constraints without the `@specforge/governance` plugin installed
- Do not confuse constraints (graduated quality) with invariants (binary violated-or-not)
- Do not use vague metrics — "fast", "secure", "reliable" are not measurable
- Do not skip the `category` field — it organizes constraints for reporting
- Do not reference behaviors or invariants from other files without `use` imports
