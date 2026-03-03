---
name: spec-traceability
description: "Author traceability matrix files in a spec's traceability/ directory. Each file maps a concern through the full chain: requirement → behavior → ADR → invariant → test → risk. Use when creating traceability matrices, auditing trace chain completeness, or verifying coverage targets."
---

# Spec Traceability

Rules and conventions for authoring **traceability matrix files** in a spec's `traceability/` directory. The traceability matrix maps requirements through the full chain: requirement → behavior → ADR → invariant → test → risk.

## When to Use

- Creating a traceability matrix for a spec at Full governance tier
- Auditing trace chain completeness (requirements without tests, invariants without FMEA entries)
- Verifying coverage targets are met per risk level
- Adding new entries after behaviors, invariants, or tests change

## Directory Structure

```
traceability/
  index.yaml                    # Manifest of all traceability files
  TRACE-XX-001-capability.md    # Traceability for a capability domain
  TRACE-XX-002-invariants.md    # Invariant-focused traceability
  TRACE-XX-003-decisions.md     # ADR-focused traceability
  ...
```

### index.yaml Schema

```yaml
kind: traceability
package: "@hex-di/<name>"
infix: XX
entries:
  - id: TRACE-XX-001
    file: TRACE-XX-001-capability-level.md
    title: Capability-Level Traceability
    status: active              # active | draft | deprecated
    scope: capability           # capability | invariant | adr | test | dod
  - id: TRACE-XX-002
    file: TRACE-XX-002-invariant-traceability.md
    title: Invariant Traceability
    status: active
    scope: invariant
  - id: TRACE-XX-003
    file: TRACE-XX-003-adr-traceability.md
    title: ADR Traceability
    status: active
    scope: adr
```

**Rules:**
- Every `.md` file in `traceability/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- `scope` indicates the traceability concern

## Required Traceability Tables

At minimum, create files for these three concerns:

### 1. Capability-Level Traceability

Maps behavior files to invariants, ADRs, risk levels, and test files.

```markdown
# TRACE-XX-001: Capability-Level Traceability

| Behavior File | BEH IDs | Invariants | ADRs | Risk | Test Files |
|---------------|---------|------------|------|------|------------|
| BEH-XX-001-<name>.md | BEH-XX-001--008 | INV-XX-1, INV-XX-2 | ADR-001 | Med | tests/<name>.test.ts |
```

### 2. Invariant Traceability

Maps each invariant to its enforcement point, related behaviors, risk level, and FMEA entry.

```markdown
# TRACE-XX-002: Invariant Traceability

| Invariant | Enforcement Point | Behaviors | Risk Level | FMEA Entry |
|-----------|-------------------|-----------|------------|------------|
| INV-XX-1 | `Class.method()` | BEH-XX-001, BEH-XX-003 | High | FM-XX-001 |
```

### 3. ADR Traceability

Maps each ADR to the invariants and behavior files it affects.

```markdown
# TRACE-XX-003: ADR Traceability

| ADR | Status | Invariants Affected | Behavior Files Affected |
|-----|--------|---------------------|------------------------|
| ADR-001 | Accepted | INV-XX-1 | BEH-XX-001-graph-ops.md |
```

### Optional Additional Tables

- **Test File Map** — Maps test files to the behavior IDs they verify
- **DoD Traceability** — Maps DoD items to their acceptance status
- **Deliverable Coverage** — Maps deliverables to the capabilities they bundle (`DLV → UX` via `capabilities[]`)
- **Library Coverage** — Maps libraries to the features they implement (`LIB → FEAT` via `features[]`)

## Content Rules

1. **YAML frontmatter** — Every traceability file MUST start with `---` frontmatter containing `id`, `kind: traceability`, `title`, `status`, `scope`.
2. **Every requirement maps to a test** — Every requirement ID must map to at least one test file.
2. **Every invariant maps to coverage** — Every invariant must map to test coverage per its risk level.
3. **Every ADR maps to impact** — Every ADR must list which invariants and spec files it affects.
4. **Accurate counts** — Total requirement count must be maintained and accurate.
5. **Complete chain** — The traceability chain must be complete: requirement → source → test → invariant → FMEA → DoD.

## Cross-References

```markdown
# From traceability to behavior files:
| BEH-XX-001-<name>.md | BEH-XX-001--008 | ... |

# From traceability to invariants:
| INV-XX-1 | `Class.method()` | BEH-XX-001 | High | FM-XX-001 |

# From traceability to test files:
| tests/unit/graph.test.ts | BEH-XX-001, BEH-XX-002 |
```
