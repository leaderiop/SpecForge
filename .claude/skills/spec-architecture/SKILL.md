---
name: spec-architecture
description: "Sub-orchestrator for the architecture/ directory in a spec. Manages C4 diagrams at multiple abstraction levels, dynamic sequence diagrams, deployment topologies, and ports-and-adapters mappings. Delegates to c4-methodology for C4 concepts, c4-mermaid-syntax for Mermaid rendering, and uml-diagrams for UML diagram syntax."
---

# Spec Architecture (Sub-Orchestrator)

Sub-orchestrator for the `architecture/` directory in a spec. Manages the directory structure, cross-level consistency, and delegates to specialized skills for diagram content.

## When to Use

- Creating an `architecture/` directory for a multi-component package
- Adding a new C4 diagram at any level (context, container, component, dynamic, deployment)
- Auditing cross-level consistency between C4 diagrams
- Creating or updating the architecture index

## Delegation

| Task | Delegate To |
|------|-------------|
| C4 modeling concepts, abstraction levels, element types | **c4-methodology** |
| Mermaid C4 diagram syntax, rendering, ASCII fallbacks | **c4-mermaid-syntax** |
| UML diagrams (class, sequence, state, component) | **uml-diagrams** |

**This skill owns:** directory structure, index template, cross-level consistency rules, and diagram file structure conventions.

## Directory Structure

```
architecture/
  index.yaml                    # Manifest of all architecture files
  index.md                      # Navigation index with diagram inventory table
  c1-system-context.md          # L1: System + users + external systems
  c2-containers.md              # L2: Internal containers and interconnections
  c3-<component>.md             # L3: One file per C2 container
  dynamic-<flow>.md             # Runtime sequence diagrams
  deployment-<mode>.md          # Deployment topologies
  ports-and-adapters.md         # Port registry, adapter mapping
```

### index.yaml Schema

```yaml
kind: architecture
package: "@hex-di/<name>"
entries:
  - id: ARCH-001
    file: c1-system-context.md
    title: System Context
    status: active              # active | draft | deprecated
    c4_level: L1                # L1 | L2 | L3 | dynamic | deployment | mapping
  - id: ARCH-002
    file: c2-containers.md
    title: Containers
    status: active
    c4_level: L2
  - id: ARCH-003
    file: c3-runtime.md
    title: Runtime Component
    status: active
    c4_level: L3
  - id: ARCH-004
    file: dynamic-session-lifecycle.md
    title: Session Lifecycle Flow
    status: active
    c4_level: dynamic
  - id: ARCH-005
    file: ports-and-adapters.md
    title: Port Registry
    status: active
    c4_level: mapping
```

**Rules:**
- Every `.md` file in `architecture/` (except `index.md`) MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries

## Index File Template

```markdown
# Architecture Overview

**Scope:** Navigation index for all C4 architecture diagrams.

---

## Diagram Inventory

| # | File | C4 Level | Description |
|---|------|----------|-------------|
| 1 | [c1-system-context.md](./c1-system-context.md) | L1 Context | <Scope> |
| 2 | [c2-containers.md](./c2-containers.md) | L2 Container | <Scope> |
...

## How to Read These Diagrams

Each diagram file contains:
1. A **Mermaid** code block for rendering in compatible tools
2. An **ASCII fallback** for terminal/plain-text review
3. **Cross-references** to behavioral specs, type definitions, and architectural decisions

## Diagram Relationships

<ASCII tree showing how diagrams relate across levels.>
```

## Diagram File Structure

Each diagram file contains:

1. A **Mermaid** code block (use **c4-mermaid-syntax** skill for syntax)
2. An **ASCII fallback** for terminal/plain-text review
3. **Cross-references** to behavioral specs, type definitions, and architectural decisions

## Content Rules

1. **YAML frontmatter** — Every architecture `.md` file (except `index.md`) MUST start with `---` frontmatter containing `id`, `kind: architecture`, `title`, `status`, `c4_level`.

## Cross-Level Consistency Rules

These rules MUST be enforced across all architecture files:

1. Every element at **C3** must appear in its parent **C2** container
2. Every **C2** container must appear in **C1** system boundary
3. **Dynamic** diagrams reference components from **C3**
4. **Deployment** diagrams reference containers from **C2**
5. **Ports-and-adapters** references ports defined in type specifications

## Cross-References

```markdown
# From architecture to behaviors:
See [BEH-XX-001-graph-ops.md](../behaviors/BEH-XX-001-graph-ops.md)

# From architecture to types:
See [types/graph.md](../types/graph.md)

# From architecture to ADRs:
See [ADR-001](../decisions/ADR-001-closures-over-classes.md)

# From roadmap to architecture:
### Architecture Coverage
- [c2-containers.md](./architecture/c2-containers.md) -- Phase 1 containers
```
