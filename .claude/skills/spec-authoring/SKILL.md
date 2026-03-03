---
name: spec-authoring
description: "Orchestrator for creating, enhancing, and maintaining package specification documents following the hex-di spec conventions. Delegates to specialized skills for each document type. Use when writing new spec documents, enhancing existing specs, auditing spec completeness, creating governance scaffolding, or when you need to determine which specialized skill to invoke for a specific document type."
---

# Spec Authoring (Orchestrator)

Orchestrator skill for creating and maintaining specification documents in the hex-di monorepo. This skill owns cross-cutting concerns and delegates to specialized skills for each document type.

**All patterns are derived from the `specforge` specification suite (`spec/specforge/`), which is the canonical reference.**

## When to Use This Skill

- Determining which specialized skill to invoke for a document type
- Writing a new spec from scratch (use the enhancement workflow below)
- Auditing a spec directory for completeness against the canonical structure
- Understanding the overall spec structure, tiers, and conventions

## Delegation Table

| Document Type | Directory/File | Skill | ID Prefix |
|---------------|---------------|-------|-----------|
| Overview | `overview.md` | **spec-overview** | — |
| Features | `features/` | **spec-features** | `FEAT-XX-NNN` |
| Capabilities | `capabilities/` | **spec-capabilities** | `UX-XX-NNN` |
| Behaviors | `behaviors/` | **spec-behaviors** | `BEH-XX-NNN` |
| Invariants | `invariants/` | **spec-invariants** | `INV-XX-N` |
| Decisions | `decisions/` | **spec-decisions** | `ADR-NNN` |
| Glossary | `glossary.md` | **spec-glossary** | — |
| Traceability | `traceability/` | **spec-traceability** | `TRACE-XX-NNN` |
| Risk Assessment | `risk-assessment/` | **spec-risk-assessment** | `FM-XX-NNN` |
| Types | `types/` | **spec-types** | — |
| Type System | `type-system/` | **spec-type-system** | — |
| Process | `process/` | **spec-process** | — |
| Roadmap | `roadmap/` | **roadmap-spec-author** | `RM-NN` |
| Architecture | `architecture/` | **spec-architecture** (sub-orchestrator) | — |
| Visual | `visual/` | **spec-visual** (sub-orchestrator) | — |
| Compliance | `compliance/` | **spec-compliance** (sub-orchestrator) | — |
| Product | `product/` | **spec-product** | — |
| Research | `research/` | **spec-research** | `RES-NN` |
| Plugins | `plugins/` | **spec-plugins** | `PLG-*` |
| Deliverables | `deliverables/` | **spec-deliverables** | `DLV-XX-NNN` |
| Libraries | `libraries/` | **spec-libraries** | `LIB-XX-NNN` |
| References | `references/` | **spec-references** | — |

### Sub-Orchestrator Delegation

| Sub-Orchestrator | Delegates To |
|------------------|-------------|
| **spec-architecture** | c4-methodology, c4-mermaid-syntax, uml-diagrams |
| **spec-visual** | view-spec-author |
| **spec-compliance** | gxp-spec-review |

## Spec Directory Layout

Every package spec lives under `spec/` following this hierarchy:

```
spec/
  packages/<name>/          # Core DI kernel packages
  libs/<domain>/            # Feature libraries
  tooling/<name>/           # Developer tooling
  cross-cutting/<name>/     # Multi-package specs
  specforge/                # The canonical reference spec
```

### Physical Location Rules

- The spec tier matches the package tier: `packages/core` → `spec/packages/core/`
- Nested packages get nested specs: `packages/core/react` → `spec/packages/core/react/`
- Libraries use domain grouping: `libs/flow/core` → `spec/libs/flow/`
- Specs that span multiple packages live in `spec/cross-cutting/`

### Canonical Directory Structure

Every spec uses this single structure. Not every directory is required — see the completeness tiers — but when a directory/file exists, it must conform to the conventions in the delegated skill.

```
spec/<tier>/<name>/
  overview.md                       # → spec-overview
  glossary.md                       # → spec-glossary
  features/                         # → spec-features
    index.yaml
    FEAT-XX-NNN-<name>.md
  capabilities/                     # → spec-capabilities
    index.yaml
    UX-XX-NNN-<name>.md
    <group>/                        # Optional sub-folders for large specs (>30 capabilities)
      UX-XX-NNN-<name>.md
  behaviors/                        # → spec-behaviors
    index.yaml
    BEH-XX-NNN-<capability>.md
  invariants/                       # → spec-invariants
    index.yaml
    INV-XX-N-<name>.md
  decisions/                        # → spec-decisions
    index.yaml
    ADR-NNN-<topic>.md
  traceability/                     # → spec-traceability
    index.yaml
    TRACE-XX-NNN-<name>.md
  risk-assessment/                  # → spec-risk-assessment
    index.yaml
    FM-XX-NNN-<name>.md
  roadmap/                          # → roadmap-spec-author
    index.yaml
    index.md
    RM-NN-<phase>.md
  architecture/                     # → spec-architecture
    index.yaml
    index.md
    c1-system-context.md
    c2-containers.md
    c3-<component>.md
    dynamic-<flow>.md
    deployment-<mode>.md
    ports-and-adapters.md
  types/                            # → spec-types
    index.yaml
    <domain>.md
  type-system/                      # → spec-type-system
    index.yaml
    phantom-brands.md
    structural-safety.md
  visual/                           # → spec-visual
    index.yaml
    index.md
    pages/
    components/
    elements/
    stores/
    events/
    actions/
    workflows/
  compliance/                       # → spec-compliance
    index.yaml
    gxp.md
  plugins/                          # → spec-plugins
    index.yaml
    PLG-<name>.md
  deliverables/                     # → spec-deliverables
    index.yaml
    DLV-XX-NNN-<name>.md
  libraries/                        # → spec-libraries
    index.yaml
    <family>/                       # One sub-folder per package family
      LIB-XX-NNN-<name>.md
  product/                          # → spec-product
    index.yaml
    pitch.md
  research/                         # → spec-research
    index.yaml
    RES-NN-<topic>.md
  references/                       # → spec-references
    index.yaml
    index.md
    <tool>/
  comparisons/                      # Free-form, no dedicated skill
  process/                          # → spec-process
    index.yaml
    definitions-of-done.md
    test-strategy.md
    requirement-id-scheme.md
    change-control.md
    document-control-policy.md
    ci-maintenance.md
  scripts/
    verify-traceability.sh
```

### index.yaml Convention

Every directory-based document type MUST have an `index.yaml` manifest that:
- Lists all files in the directory with their IDs, titles, status, and metadata
- Is machine-readable by bash verification scripts (parsed with `yq` or `grep`/`sed`)
- Serves as the single source of truth for what exists in the directory

Each specialized skill documents the specific `index.yaml` schema for its directory.

### YAML Frontmatter Convention

Every `.md` file in a spec directory **MUST** start with YAML frontmatter delimited by `---`, except:
- `index.md` files (navigation indexes, no frontmatter)
- `visual/**/*.md` files (use `spec.yaml` triads instead)
- `references/**/*.md` files (external content, exempt)

#### Common Fields (required on ALL frontmatter files)

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | The file's primary ID (e.g., `BEH-SF-001`, `ADR-005`, `INV-SF-7`). Omit for singletons like `overview.md` and `glossary.md`. |
| `kind` | string | The document kind from the registry below |
| `title` | string | Human-readable title |
| `status` | string | Lifecycle status (`draft`, `active`, `accepted`, `deprecated`, etc.) |

#### Kind-to-Directory Registry

| `kind` | Directory | Extra Required Fields |
|--------|-----------|----------------------|
| `behavior` | `behaviors/` | `id_range`, `invariants`, `adrs`, `types`, `ports` |
| `invariant` | `invariants/` | `enforced_by`, `behaviors`, `risk` |
| `decision` | `decisions/` | `date`, `supersedes`, `invariants` |
| `feature` | `features/` | `adrs`, `behaviors` |
| `capability` | `capabilities/` | `features`, `behaviors`, `persona`, `surface` |
| `traceability` | `traceability/` | `scope` |
| `risk-assessment` | `risk-assessment/` | `fm_range`, `invariants` |
| `roadmap` | `roadmap/` | `phase`, `dependencies` |
| `types` | `types/` | `behaviors`, `adrs` |
| `type-system` | `type-system/` | `invariants`, `adrs` |
| `architecture` | `architecture/` | `c4_level` |
| `research` | `research/` | `outcome`, `related_adr` |
| `plugin` | `plugins/` | `activation`, `plugin_type`, `behaviors_added` |
| `deliverable` | `deliverables/` | `deliverable_type`, `capabilities`, `depends_on` |
| `library` | `libraries/` | `library_type`, `family`, `npm_name`, `path`, `features`, `depends_on` |
| `process` | `process/` | _(common only)_ |
| `product` | `product/` | _(common only)_ |
| `compliance` | `compliance/` | `regulations` |
| `overview` | root | `package`, `version` |
| `glossary` | root | `package` |

#### Design Rules

1. **Frontmatter = file-level metadata, not section-level.** Per-behavior invariant blockquotes stay in prose.
2. **Bare IDs in frontmatter** — Use plain strings (`INV-SF-7`), not markdown links. Rich links stay in prose body.
3. **Structured key-value lines move to frontmatter** — Lines like `**Status:** Accepted`, `**Date:** 2025-03-01`, `**Enforced by:** ...` are REMOVED from prose and placed in frontmatter.
4. **Rich content stays in prose** — Invariant blockquotes, scope descriptions, prose paragraphs remain in the markdown body.
5. **`index.yaml` and frontmatter coexist** — `index.yaml` is the directory manifest; frontmatter is per-file metadata. Both exist.
6. **List fields use YAML arrays** — Fields like `invariants`, `adrs`, `behaviors` use `[INV-SF-7, INV-SF-10]` array syntax.

#### Example Frontmatter

```yaml
---
id: BEH-SF-001
kind: behavior
title: Graph Operations
status: active
id_range: "001--008"
invariants: [INV-SF-7, INV-SF-10]
adrs: [ADR-001, ADR-005]
types: [graph]
ports: [GraphQueryPort, GraphMutationPort, GraphStorePort, GraphSyncPort, NLQPort]
---
```

```yaml
---
id: ADR-005
kind: decision
title: Graph-First Architecture
status: Accepted
date: "2025-03-01"
supersedes: [ADR-006, ADR-007, ADR-010]
invariants: [INV-SF-7, INV-SF-8, INV-SF-10]
---
```

```yaml
---
id: DLV-SF-001
kind: deliverable
title: Desktop Application
status: active
deliverable_type: app
capabilities: [UX-SF-001, UX-SF-002, UX-SF-003]
depends_on: [LIB-SF-001, LIB-SF-002]
---
```

```yaml
---
id: LIB-SF-001
kind: library
title: DI Kernel
status: active
npm_name: "@hex-di/core"
path: "packages/core"
library_type: core
family: core
features: [FEAT-SF-001, FEAT-SF-002]
depends_on: []
---
```

## Completeness Tiers

| Tier | What It Has | When to Use |
|------|-------------|-------------|
| **Stub** | `overview.md` only | Placeholder for a planned package |
| **Technical-only** | `overview.md` + `behaviors/` | Early design, no governance yet |
| **Technical + behaviors** | Above + `decisions/`, `invariants/`, `glossary.md` | Active development, design rationale captured |
| **Full governance** | Above + `traceability/`, `risk-assessment/`, `roadmap/`, `process/`, `scripts/` | Production-quality spec |
| **Full + architecture** | Above + `architecture/`, `types/`, `type-system/`, `comparisons/` | Multi-component systems requiring C4 diagrams |
| **Full + optional** | Above + `product/`, `research/`, `plugins/`, `references/`, `compliance/`, `visual/`, `features/` | Feature-complete spec with all optional domains |
| **Full + deliverables** | Above + `deliverables/`, `capabilities/` | Spec describes what ships to users and bundles capabilities |
| **Full + libraries** | Above + `libraries/` | Spec maps code packages to features with port catalogs |

Promote a spec to the next tier when the package's maturity warrants it. Never create empty placeholder files.

### Folder Organization Principle

Directory structures follow the cardinality of their relationships:

| Relationship | Structure | Rationale | Example |
|-------------|-----------|-----------|---------|
| **1:many** | Hierarchical sub-folders | Parent naturally contains children | `libraries/<family>/LIB-*.md`, `capabilities/<group>/UX-*.md` |
| **many:many** | Flat files + frontmatter cross-refs | No single parent owns the entity | `deliverables/DLV-*.md` with `capabilities: [UX-...]` |

- **Group → Capability** is 1:many → capabilities may use group sub-folders (optional for large specs)
- **Family → Library** is 1:many → libraries use family sub-folders (required)
- **Deliverable → Capability** is many:many → deliverables are flat files with `capabilities[]` frontmatter
- **Library → Feature** is many:many → libraries are flat within their family, with `features[]` frontmatter

## ID Scheme Registry

All ID prefixes used across the spec system:

| Prefix | Format | Scope | Skill |
|--------|--------|-------|-------|
| `BEH` | `BEH-<INFIX>-NNN` | Behavioral contracts | spec-behaviors |
| `FEAT` | `FEAT-<INFIX>-NNN` | Feature specifications | spec-features |
| `UX` | `UX-<INFIX>-NNN` | User capabilities | spec-capabilities |
| `INV` | `INV-<INFIX>-N` | Runtime invariants | spec-invariants |
| `ADR` | `ADR-NNN` or `ADR-<INFIX>-NNN` | Architecture decisions | spec-decisions |
| `FM` | `FM-<INFIX>-NNN` | FMEA failure modes | spec-risk-assessment |
| `TRACE` | `TRACE-<INFIX>-NNN` | Traceability entries | spec-traceability |
| `RM` | `RM-NN` | Roadmap phases | roadmap-spec-author |
| `RES` | `RES-NN` | Research documents | spec-research |
| `PLG` | `PLG-<name>` | Plugin extensions | spec-plugins |
| `DLV` | `DLV-<INFIX>-NNN` | Deliverables (what ships) | spec-deliverables |
| `LIB` | `LIB-<INFIX>-NNN` | Libraries (what code implements) | spec-libraries |

The `<INFIX>` is a 2-3 character code unique to the package.

### Existing Package Infixes

| Package | Infix | Behavior Pattern | Invariant Pattern |
|---------|-------|-----------------|-------------------|
| result (core) | XX (chapter) | `BEH-XX-NNN` | `INV-N` |
| result-react | R | `BEH-RXX-NNN` | `INV-RN` |
| result-testing | T | `BEH-TXX-NNN` | `INV-TN` |
| clock | CK | `CLK-{DOMAIN}-NNN` | `INV-CK-N` |
| guard | GD | `REQ-GUARD-NNN` | `INV-GD-N` |
| flow | FL | `FLW-NNN` | `INV-FL-N` |
| specforge | SF | `BEH-SF-NNN` | `INV-SF-N` |

New specs should use the `BEH-<INFIX>-NNN` format. Legacy domain-prefixed formats remain valid for their packages.

## Cross-Reference Conventions

### Internal Links (within same spec)

```markdown
See [BEH-XX-001](behaviors/BEH-XX-001-<name>.md#beh-xx-001-descriptive-slug).
See [INV-XX-1](invariants/INV-XX-1-<name>.md).
See [ADR-001](decisions/ADR-001-topic.md).
See [FEAT-XX-001](features/FEAT-XX-001-<name>.md).
See [UX-XX-001](capabilities/UX-XX-001-<name>.md).
See [DLV-XX-001](deliverables/DLV-XX-001-<name>.md).
See [LIB-XX-001](libraries/core/LIB-XX-001-<name>.md).
See [GxP compliance](compliance/gxp.md).
```

### Links from Sub-Documents

```markdown
See [INV-XX-1](../invariants/INV-XX-1-<name>.md).
See [ADR-001](../decisions/ADR-001-topic.md).
See [types/<domain>.md](../types/<domain>.md).
See [UX-XX-001](../capabilities/UX-XX-001-<name>.md).
See [DLV-XX-001](../deliverables/DLV-XX-001-<name>.md).
See [LIB-XX-001](../libraries/core/LIB-XX-001-<name>.md).
```

### Cross-Package Links

Use relative paths that navigate up to the spec tier level, then down:

```markdown
See [integration patterns](../../cross-cutting/integration/flow-saga.md).
```

### Cross-Cutting GxP Links

```markdown
# From spec/libs/<name>/compliance/gxp.md:
See [regulatory framework](../../../cross-cutting/gxp/01-regulatory-framework.md).
```

### Invariant-to-Behavior Cross-References

```markdown
# In invariants/:
**Referenced from:** [BEH-XX-001-<name>.md](../behaviors/BEH-XX-001-<name>.md) (BEH-XX-001, BEH-XX-003)

# In behaviors/:
> **Invariant:** [INV-XX-N](../invariants/INV-XX-N-<name>.md) -- <Invariant Name>
```

### Heading Anchor Format

GitHub auto-generates anchors from headings:
- `## BEH-SF-001: Graph Node Creation` → `#beh-sf-001-graph-node-creation`
- `## INV-SF-1: Blackboard Append-Only History` → `#inv-sf-1-blackboard-append-only-history`

Use lowercase, hyphens, strip special characters.

## Spec Enhancement Workflow

When enhancing an existing spec to match the canonical standard:

1. **Audit current state** — List what documents exist and what's missing against the canonical structure
2. **Add overview.md** (if missing) — Invoke **spec-overview**
3. **Create behaviors/** — Invoke **spec-behaviors** for BEH-XX-NNN entries with Contract + Verification sections
4. **Create features/** (if applicable) — Invoke **spec-features** to organize behaviors into user-facing capabilities
4.5. **Create capabilities/** (if applicable) — Invoke **spec-capabilities** to describe "what the user can do" with interaction flows and diagrams
5. **Extract decisions** — Invoke **spec-decisions** for formal ADRs
6. **Identify invariants** — Invoke **spec-invariants** for runtime guarantees
7. **Build glossary** — Invoke **spec-glossary** for domain terms
8. **Create type files** (if applicable) — Invoke **spec-types** for domain interfaces
9. **Create type system docs** (if applicable) — Invoke **spec-type-system** for phantom brands / structural safety
10. **Create deliverables** (if applicable) — Invoke **spec-deliverables** to describe what ships to users (apps, services, CLIs, extensions)
11. **Create libraries** (if applicable) — Invoke **spec-libraries** to map code packages to features and behaviors
12. **Create traceability** — Invoke **spec-traceability** for the full requirement chain
13. **Assess risk** — Invoke **spec-risk-assessment** for FMEA entries
14. **Create DoD + test strategy + ID scheme** — Invoke **spec-process**
15. **Add compliance** (if GxP) — Invoke **spec-compliance**
16. **Create roadmap** — Invoke **roadmap-spec-author**
17. **Add architecture** (if multi-component) — Invoke **spec-architecture**
18. **Add visual** (if UI specs needed) — Invoke **spec-visual**
19. **Add verification script** — Create `scripts/verify-traceability.sh`
20. **Verify cross-references** — Ensure all links resolve correctly

## Verification Script (scripts/verify-traceability.sh)

Every spec at the **Full governance** or **Technical + behaviors** tier must include a verification script.

### Script Architecture

```bash
#!/usr/bin/env bash
set -euo pipefail

STRICT=false
[[ "${1:-}" == "--strict" ]] && STRICT=true

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SPEC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$SPEC_DIR/<relative-path-to-repo-root>" && pwd)"
```

### Required Checks

| # | Check | Validates |
|---|-------|-----------|
| 1 | Spec file existence | Every spec file linked in traceability exists on disk |
| 2 | Invariant completeness | Every INV-* in invariants/ has a traceability entry |
| 3 | ADR completeness | Every decisions/*.md has a traceability entry |
| 4 | Test file existence | Every test file in traceability exists under tests/ |
| 5 | Forward traceability | Every requirement ID has @traces annotations in tests |
| 6 | No orphaned test files | Every *.test.ts appears in traceability |
| 7 | index.yaml consistency | Every index.yaml entry has a corresponding file |

Code-side checks (4-6) skip when the package doesn't exist yet. With `--strict`, SKIPs become FAILs.

### macOS Compatibility

Use `sed` instead of `grep -P` (macOS grep lacks PCRE):

```bash
# CORRECT:
sed -n 's/.*](\([^)]*\)).*/\1/p' file.md
```

## What NOT to Do

- Do not copy GxP compliance content from another package unless genuinely needed
- Do not create governance documents for packages still in early design (stick to the appropriate tier)
- Do not add Document Control headers to technical-only specs
- Do not assign requirement IDs retroactively without verifying each ID maps to a genuine testable requirement
- Do not create empty placeholder files — every file must contain substantive content
- Do not duplicate content between overview and behavior specs
- Do not mix domain type definitions (`types/`) with compile-time safety patterns (`type-system/`)
- Do not use a flat alphabetical list in the glossary — group terms in logical sections
- Do not create multiple DoD files — one file with per-document-type checklists
- Do not put behaviors in root-level numbered chapters — all behavioral contracts live in `behaviors/`
