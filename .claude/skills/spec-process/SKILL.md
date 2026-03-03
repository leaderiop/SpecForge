---
name: spec-process
description: "Author process documents in a spec's process/ directory. Covers definitions-of-done, test-strategy, requirement-id-scheme, change-control, document-control-policy, and ci-maintenance conventions. Use when creating governance process docs, defining DoD checklists, writing test strategies, or documenting ID schemes."
---

# Spec Process

Rules and conventions for authoring **process documents** in a spec's `process/` directory. These documents define governance processes: how requirements are identified, tested, tracked, and controlled.

## When to Use

- Creating governance process docs for a spec at Full governance tier
- Writing or updating Definitions of Done checklists
- Defining test strategy with pyramid levels and coverage targets
- Documenting the requirement ID scheme for a package
- Adding change control or document control policies

## Directory Structure

```
process/
  index.yaml                    # Manifest of all process files
  definitions-of-done.md        # Per-document-type DoD checklists
  test-strategy.md              # Test pyramid, coverage targets, protocols
  requirement-id-scheme.md      # Formal ID formats and allocation ranges
  change-control.md             # Change categories, approval workflow
  document-control-policy.md    # Git-based versioning, approval evidence
  ci-maintenance.md             # CI pipeline stages, release process
```

### index.yaml Schema

```yaml
kind: process
package: "@hex-di/<name>"
entries:
  - id: PROC-001
    file: definitions-of-done.md
    title: Definitions of Done
    status: active              # active | draft | deprecated
  - id: PROC-002
    file: test-strategy.md
    title: Test Strategy
    status: active
  - id: PROC-003
    file: requirement-id-scheme.md
    title: Requirement ID Scheme
    status: active
  - id: PROC-004
    file: change-control.md
    title: Change Control
    status: active
  - id: PROC-005
    file: document-control-policy.md
    title: Document Control Policy
    status: active
  - id: PROC-006
    file: ci-maintenance.md
    title: CI Maintenance
    status: active
```

## Content Rules

1. **YAML frontmatter** — Every process file MUST start with `---` frontmatter containing `id`, `kind: process`, `title`, `status`.

## Definitions of Done (definitions-of-done.md)

One file with DoD checklists organized by document type. No dual-file structure.

### Required DoD Sections

| DoD Section | Key Criteria |
|-------------|-------------|
| **Behavior File** | All BEH IDs defined, Contract + Verification sections present, header links to invariants/ADRs/types, no duplicate IDs |
| **Architecture Diagram** | Mermaid + ASCII fallback, C4 cross-level consistency, cross-references present |
| **Type File** | `readonly` fields, unique `_tag` discriminants, behavior cross-references, no duplicates |
| **Governance File** | All referenced IDs exist, no broken cross-references, consistent formatting |
| **Feature/PR** | Spec updates, unit tests (>95% line, >90% branch), type tests, GxP tests (if applicable), mutation tests (>95%), traceability updated, build passes |

### Content Rules

- Every DoD item must reference the document type or spec section(s) it verifies
- Every DoD item must specify required test types when applicable
- DoD items for high-risk invariants must include mutation testing criteria (>= 80% score)
- `[OPERATIONAL]` requirements are excluded from automated test coverage calculations
- The DoD is a living document — mark items as `Done` or `Pending` to track progress

## Test Strategy (test-strategy.md)

### Required Sections

1. **Test Pyramid**: Table with Level, Scope, Tools, Target columns
2. **Coverage Targets**: Per risk-level coverage percentages
3. **File Naming Conventions**: Patterns for each test type
4. **Qualification Protocols** (GxP packages only): IQ/OQ/PQ test protocols
5. **Test-to-Requirement Traceability**: `@requirements` and `@invariants` annotations

### Test Pyramid Levels

| Level | File Pattern | Purpose | When Required |
|-------|-------------|---------|---------------|
| **Unit** | `tests/unit/*.test.ts` or `tests/*.test.ts` | Individual function behavior | Always |
| **Type** | `tests/*.test-d.ts` | Compile-time type contracts | When package exports complex types |
| **GxP Integrity** | `tests/unit/gxp-*.test.ts` or `tests/gxp/*.test.ts` | High-risk invariant verification | When invariant has High risk level |
| **Integration** | `tests/integration/*.test.ts` | Cross-module behavior | When package has DI container integration |
| **Mutation** | Stryker config | Mutation score for critical paths | High-risk invariants; target >= 80% |
| **Performance** | `tests/perf/*.bench.ts` | Latency and throughput baselines | When package has performance SLAs |

### Coverage Targets

| Risk Level | Branch Coverage | Line Coverage | Mutation Kill Rate |
|-----------|----------------|---------------|-------------------|
| High | >= 90% | >= 95% | >= 80% |
| Medium | >= 80% | >= 90% | >= 60% |
| Low | >= 70% | >= 85% | -- |

### Qualification Protocols (GxP packages only)

| Protocol | Purpose | Scope |
|----------|---------|-------|
| **IQ** | Verify package installs correctly | Dependencies, subpath exports |
| **OQ** | Verify package operates as specified | All DoD items, all test levels |
| **PQ** | Verify package performs under real conditions | Integration tests, benchmarks, stress |

## Requirement ID Scheme (requirement-id-scheme.md)

### Standard Format

```
BEH-<INFIX>-NNN    # Behavior IDs
FEAT-<INFIX>-NNN   # Feature IDs
INV-<INFIX>-N       # Invariant IDs
ADR-NNN             # Decision IDs (or ADR-<INFIX>-NNN)
FM-<INFIX>-NNN      # Failure mode IDs
TRACE-<INFIX>-NNN   # Traceability IDs
```

The `<INFIX>` is a 2-3 character code unique to the package.

### ID Uniqueness Rules

- Each ID appears exactly once in its document type directory
- IDs are never reused — deleted requirements keep their number reserved
- New behaviors append to the end of their allocation range or start a new file

### Allocation Ranges

Document allocation ranges per behavior file:

```markdown
### Allocation Ranges

| Range | File | Domain |
|-------|------|--------|
| 001--008 | `BEH-XX-001-graph-operations.md` | Graph store operations |
| 009--016 | `BEH-XX-009-session-materialization.md` | Session chunks |
```

## Other Process Documents

- **change-control.md**: Change categories, approval workflow, versioning rules
- **document-control-policy.md**: Git-based versioning, approval evidence, retention
- **ci-maintenance.md**: CI pipeline stages, automated checks, release process

These follow free-form prose structure. No specific template is mandated, but they must be internally consistent and cross-reference relevant spec documents.
