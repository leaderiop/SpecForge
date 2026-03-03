---
name: spec-invariants
description: "Author invariant specification files in a spec's invariants/ directory. Each file documents a runtime guarantee with INV-XX-N IDs, Enforced-by annotations, and Referenced-from cross-links. Use when identifying runtime guarantees, creating invariant specs, or auditing invariant-to-behavior/FMEA traceability."
---

# Spec Invariants

Rules and conventions for authoring **invariant specification files** in a spec's `invariants/` directory. Invariants are runtime guarantees the system maintains at all times.

## When to Use

- Identifying runtime guarantees a package maintains
- Creating new invariant specifications
- Auditing invariant completeness (missing Enforced-by/Referenced-from)
- Checking invariant-to-behavior and invariant-to-FMEA traceability

## Directory Structure

```
invariants/
  index.yaml                    # Manifest of all invariant files
  INV-XX-1-<name>.md            # One file per invariant
  INV-XX-2-<name>.md
  ...
```

### index.yaml Schema

```yaml
kind: invariants
package: "@hex-di/<name>"
infix: XX                       # 2-3 char package infix
entries:
  - id: INV-XX-1
    file: INV-XX-1-immutable-graph-nodes.md
    title: Immutable Graph Nodes
    status: active              # active | draft | deprecated
    risk_level: High            # High | Medium | Low
  - id: INV-XX-2
    file: INV-XX-2-session-isolation.md
    title: Session Isolation
    status: active
    risk_level: Medium
```

**Rules:**
- Every `.md` file in `invariants/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- `risk_level` aligns with the FMEA assessment in `risk-assessment/`

## File Naming

- ID-prefixed: `INV-XX-N-<name>.md`
- Kebab-case name describing the guarantee
- Examples: `INV-SF-1-blackboard-append-only.md`, `INV-GD-3-policy-immutability.md`

## File Template

```markdown
---
id: INV-XX-N
kind: invariant
title: "<Invariant Name>"
status: active
enforced_by: ["Class.method()", "Component"]
behaviors: [BEH-XX-NNN]
risk: FM-XX-NNN
---

# INV-XX-N: <Invariant Name>

<Prose description of the guarantee. What the system promises.
What consumers can rely on.>
```

## Content Rules

1. **YAML frontmatter** — Every invariant file MUST start with `---` frontmatter containing `id`, `kind: invariant`, `title`, `status`, `enforced_by`, `behaviors`, `risk`. The `**Enforced by:**`, `**Referenced from:**`, `**Risk:**` lines are REMOVED from prose — that metadata lives in frontmatter.
2. **One guarantee per file** — Don't combine unrelated guarantees in a single invariant.
3. **Enforced by** — Names the code components (classes, functions, modules) that enforce this guarantee (in frontmatter).
4. **Referenced from** — Lists behavior IDs that depend on this invariant (in frontmatter `behaviors` field).
5. **Risk link** — Links to the FMEA failure mode ID (in frontmatter `risk` field).
6. **Traceability** — Every invariant must appear in `traceability/` and `risk-assessment/`.
7. **Consumer perspective** — The prose description states what consumers can rely on, not implementation details.

## Cross-References

```markdown
# From invariants to behaviors:
**Referenced from:** [BEH-XX-001-graph-ops.md](../behaviors/BEH-XX-001-graph-ops.md) (BEH-XX-001, BEH-XX-003)

# From behaviors to invariants (blockquote in behavior file):
> **Invariant:** [INV-XX-1](../invariants/INV-XX-1-immutable-graph-nodes.md) -- Immutable Graph Nodes

# From risk assessment to invariants:
| FM-XX-001 | <Failure> | INV-XX-1 | ... |

# From traceability to invariants:
Maps each invariant to its enforcement point, related behaviors, risk level, and FMEA entry
```
