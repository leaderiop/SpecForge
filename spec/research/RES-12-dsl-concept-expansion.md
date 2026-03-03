---
id: RES-12
kind: research
title: "DSL Concept Expansion — Which Spec Concepts Belong in the Compiler?"
status: active
date: 2026-03-01
depends_on: RES-11a
---

# RES-12: DSL Concept Expansion Analysis

## Problem Statement

The specforge `.spec` DSL compiler (RES-11a) defines 10 core node types. The broader spec authoring system uses 15+ additional document types currently managed as markdown by Claude skills. The question: **which of these concepts should be promoted to first-class DSL constructs, and which should stay as markdown?**

This analysis was produced by 10 specialized agents examining the question from different angles: architecture, graph complexity, type systems, AI ergonomics, competitor analysis, GxP compliance, deliverables/libraries, visual/UI, migration strategy, and plugin extensibility.

---

## Current State

### Core Node Types (from RES-11a)

| Node Type | Role |
|---|---|
| `spec` | Project root config |
| `invariant` | Runtime guarantees |
| `decision` | Architecture decisions |
| `behavior` | Behavioral contracts |
| `feature` | Feature composition |
| `capability` | UX-level stories |
| `failure_mode` | FMEA risk assessment |
| `type` | Domain types |
| `port` | Interface contracts |

### Candidates for Promotion (15 concepts)

| Concept | Current Format |
|---|---|
| deliverable | Markdown + YAML |
| library | Markdown + YAML |
| plugin | Markdown + YAML |
| roadmap | Markdown + YAML |
| traceability | Markdown (hand-written matrices) |
| research | Markdown + YAML |
| compliance | Markdown + YAML |
| architecture | Mermaid + prose |
| type-system | Markdown (patterns) |
| visual | YAML triads |
| process | Markdown |
| glossary | Markdown |
| overview | Markdown (document map) |
| product | Markdown (pitch) |
| references | Markdown (external links) |

---

## Agent Consensus Matrix

Each agent classified concepts independently. The table below shows votes across all 10 agents.

Legend: **P** = Promote to DSL, **D** = Defer/Plugin, **K** = Keep as Markdown, **N** = Never (generated output), **—** = not assessed

| Concept | Arch | Graph | AI Ergo | Compete | GxP | Deliv/Lib | Visual | Types | Migration | Plugin | Consensus |
|---|---|---|---|---|---|---|---|---|---|---|---|
| **deliverable** | P | P | P | — | — | P | — | — | P | — | **PROMOTE** |
| **library** | P | P | P | — | — | P | — | — | P | — | **PROMOTE** |
| **roadmap** | P | P | P | — | — | — | — | — | P | — | **PROMOTE** |
| **compliance** | P | P | P | — | P | — | — | — | D | — | **INCLUDE VIA META-SCHEMA** |
| **plugin** | D | D | P | — | — | — | — | — | D | — | **INCLUDE VIA META-SCHEMA** |
| **glossary** | P | N | P | — | — | — | D | — | D | — | **INCLUDE VIA META-SCHEMA** |
| **research** | K | — | K | — | — | — | — | — | D | — | **KEEP MARKDOWN** |
| **architecture** | D | N | K | — | — | — | D | — | D | — | **PLUGIN ECOSYSTEM** |
| **visual** | D | — | P | — | — | — | P* | — | D | — | **PLUGIN ECOSYSTEM** |
| **type-system** | K | D | P | — | — | — | — | — | D | — | **KEEP MARKDOWN** |
| **process** | K | D | K | — | P* | — | — | — | D | — | **KEEP MARKDOWN** |
| **traceability** | K | N | P | — | N | — | — | — | N | — | **NEVER (generated)** |
| **overview** | K | N | K | — | — | — | N | — | N | — | **NEVER (generated)** |
| **product** | K | P | K | — | — | — | K | — | N | — | **KEEP MARKDOWN** |
| **references** | K | N | P | — | — | — | — | — | N | — | **KEEP MARKDOWN** |

\* With caveats — see detailed analysis below.

---

## Key Insights from Prior Art

The competitor analysis (10 tools studied: Protobuf, OpenAPI, Terraform, PlantUML/Mermaid, ADR tools, DOORS/Jama/Polarion, Cucumber/Gherkin, CUE, Backstage, Structurizr) revealed five patterns of successful specification tools:

1. **They formalize nouns, not verbs.** Define what exists and how things relate. Not what happens at runtime.
2. **They operate at exactly one abstraction level.** Protobuf: wire format. Gherkin: behavior specs. C4: architecture structure.
3. **They have a small stable core with an extension mechanism.** Terraform: ~10 block types + infinite providers. Backstage: 9 entity kinds + annotations.
4. **They separate structure from content.** Structurizr owns boxes/arrows; `!docs` holds prose. Gherkin owns Given/When/Then; step definitions hold implementation.
5. **They can be adopted incrementally.** One protobuf message without every service. One Backstage component without the whole org.

### The Danger Zones

- **"Everything is structured"** (Polarion trap): When every artifact has its own schema, overhead dominates. Voluntary adoption collapses.
- **"One schema to rule them all"** (CUE trap): Theoretical elegance doesn't translate to practical clarity.
- **"Scope creep through types"** (Mermaid pattern): Adding types is easy; maintaining quality across 20+ types is hard.

### The Sweet Spot

Successful tools have **5-9 core concept types** (the "Backstage number"). Specforge's current 10 is at the upper edge. Adding 15 more would push it into requirements-management-tool territory.

**Recommended model: Terraform's two-layer architecture** — tiny stable core grammar + extensible provider/plugin ecosystem for domain-specific concepts.

---

## Tier Recommendations

### Tier 1: Core DSL (3 concepts)

These complete the core traceability chain and have the highest cross-reference error rates.

#### `deliverable`

- **Rationale:** Bridges capabilities to libraries. Completes the chain: `deliverable -> capability -> feature -> behavior -> invariant`. Cross-references (`capabilities: [create_new_user]`, `depends_on: [core_lib]`) are exactly what the compiler eliminates.
- **Implementation cost:** Low. Structurally identical to `feature`.
- **New edges:** `bundles` (deliverable -> capability), `built_from` (deliverable -> library)

```spec
deliverable user_mgmt_mvp "User Management MVP" {
  type         app
  personas     [admin, developer]
  capabilities [create_new_user, manage_users]
  libraries    [core_lib, auth_lib]
  roadmap      phase_1_core
}
```

#### `library`

- **Rationale:** Bridges code packages to features. Creates the library -> port -> type chain that connects to RES-11b code generation. Dependency DAG benefits from cycle detection.
- **Implementation cost:** Medium. Needs dependency graph sub-validation.
- **New edges:** `provides` (library -> feature), `depends_on` (library -> library), `defines_port` (library -> port)
- **Design principle:** DSL models references and relationships. Concrete details (`npm_name`, `package.json` path) go to a `specforge verify npm` plugin.

```spec
library core_lib "@myservice/core" {
  family       core
  features     [user_management, billing]
  depends_on   [shared_utils]
  ports_defined [UserRepository, EmailService]
}
```

#### `roadmap`

- **Rationale:** Actively maintained, high reference density. 8 verification checks in current bash system. Provides the temporal dimension — when things ship.
- **Implementation cost:** High. Sub-entity hierarchy (phases, work items, exit criteria).
- **New edges:** `schedules` (roadmap -> feature/deliverable)

```spec
roadmap phase_1_core "Phase 1: Core" {
  status    in_progress
  behaviors [create_user, read_user_by_id, update_user_email]
  features  [user_management, auth_flow]
  criteria  [
    "All behaviors covered by tests",
    "specforge check --strict passes",
  ]
}
```

**Build order within Tier 1:**
1. `library` (aligns with `type`/`port` work)
2. `deliverable` (depends on `library` for `depends_on` references)
3. `roadmap` (highest cost, builds on library + deliverable)

---

### Tier 2: Meta-Schema Candidates (4 concepts)

These have moderate value but are either niche or need the meta-schema mechanism.

#### `compliance`

- **GxP agent recommendation:** Promote for regulated industries. Compiler-validated compliance mappings are significantly more auditor-friendly. Can enforce: "every high-risk invariant has a compliance mapping."
- **Concern:** Niche — only needed for GxP-regulated software.
- **Recommendation:** Ship as an official plugin or use the meta-schema `define` mechanism.

```spec
compliance data_integrity_controls "Data Integrity Controls" {
  regulation   "21 CFR Part 11.10(e)"
  satisfied_by [data_persistence, audit_immutability, create_user]
  evidence     "PostgreSQL audit trigger with immutable log"
}
```

#### `plugin`

- **Concern:** Naming collision with the compiler's own plugin system (RES-11a extensibility section). Rename to `extension_point` or similar.
- **Value:** The `behaviors_added` reference is useful but the concept is rarely authored.

#### `glossary`

- **Mixed signal:** Architecture agent says promote; graph/visual agents say defer.
- **The LSP already serves as an auto-glossary** via hover info on entity names.
- **If promoted:** Term definitions with `behaviors` reference lists. Orphan term detection.
- **Recommendation:** Implement via meta-schema `define` mechanism.

#### `research`

- **Low structural value:** Mostly narrative. Only `related_adr` and `outcome` fields are structural.
- **Recommendation:** Ideal candidate for the meta-schema mechanism — users who want it can define it.

---

### Tier 3: Plugin Ecosystem (3 concepts)

#### `visual` (7 entity types: page, component, element, action, event, store, wireframe)

- **Strongest structural case of any deferred concept.** The Flux cycle (element -> action -> event -> store -> component) is a real directed graph with real invariants. Current YAML system has 7 bash verify checks.
- **Problem:** Implementation cost is very high — essentially a second DSL with 7 new node types, 30+ grammar rules, and Flux cycle validation.
- **Recommendation:** Implement as a **separate `.view` file format** sharing the same in-memory graph, not shoehorned into `.spec`. Ship as `@specforge/views` plugin.

#### `architecture`

- **Problem:** Primary content is Mermaid/C4 diagrams, which don't fit a block-attribute grammar. Structurizr already has a dedicated DSL for this.
- **Recommendation:** Don't duplicate Structurizr. Instead, add a thin cross-reference layer:

```spec
architecture containers "Containers" {
  c4_level   L2
  file       "c2-containers.md"
  containers_map [
    { container: "Web App", deliverable: user_mgmt_mvp },
    { container: "API Server", deliverable: api_server },
  ]
}
```

#### `process`

- **Primarily prose** (DoD checklists, test strategies, change control workflows).
- **Partial exception:** Coverage thresholds per risk level are structural. These belong in the `spec` root block's `coverage` config, not a separate node type.

---

### Never: Stay as Generated Output or Markdown (5 concepts)

| Concept | Reason |
|---|---|
| **traceability** | Auto-generated by `specforge trace`. The entire compiler motivation is to eliminate hand-written trace matrices. All 10 agents agree. |
| **overview** | Auto-generated by compiler. Document map derived from graph. |
| **product** | Pure narrative (pitch, positioning, go-to-market). Zero cross-references. No validation possible. |
| **references** | External links. Nothing to validate. |
| **type-system** | Meta-documentation about type patterns (phantom brands, structural safety). The `type` blocks already handle actual types. |

---

## Small Additions to Core

Two tiny concepts should be added to the `spec` root block, not as separate node types:

### `persona` definitions

Capabilities reference `persona: admin` as unchecked strings. Define valid personas in the root spec:

```spec
spec "my-service" {
  version "1.0"

  persona admin       "System administrator"
  persona developer   "API integrator"
  persona viewer      "Read-only user"
}
```

Compiler validates that every `persona` in a `capability` block matches a defined persona.

### `surface` definitions

Similarly, `surface: [web, cli, api]` in capabilities should be validated:

```spec
spec "my-service" {
  version "1.0"

  surface web  "Web Dashboard"
  surface cli  "Command Line"
  surface api  "REST API"
}
```

---

## The Meta-Schema Question

### Consensus

All architecture-facing agents converged on the same conclusion: **hardcode core types and include the meta-schema mechanism from the start.**

### Meta-Schema Design Constraints

1. **Complexity explosion** — building a schema system is a project unto itself
2. **Performance** — runtime schema validation is 10-100x slower than compiled validation
3. **Error quality** — generic validators produce generic errors
4. **Learning curve** — users learn both the schema language AND the spec language
5. **Premature** — need real usage data to inform the design

### Meta-Schema Design

```spec
// specforge.spec
spec "my-service" {
  version "1.0"

  define research {
    attributes {
      outcome     enum [adr, behavior, deferred, rejected]
      related_adr ref? decision
      date        string
    }
  }
}

// Now usable as first-class syntax
research redis_evaluation "Evaluation of Redis" {
  outcome     adr
  related_adr postgres_over_mongodb
  date        2026-03-01
}
```

User-defined types get: attribute validation, reference resolution, orphan detection, LSP hover/go-to-def.
User-defined types do NOT get: custom graph-level validators (those go through the plugin API).

### Complexity Budget

The type system analysis recommends a maximum of **15-20 total node types** before validation becomes unwieldy. The identifier-based naming model simplifies this budget compared to prefix-based IDs — no prefix table to maintain, no collision between prefixes across plugins.

- 10 types, 6 edge types: ~15 valid edge combinations (manageable)
- 15 types: ~30-40 valid edges (max for hardcoded approach)
- 20 types: ~50-70 valid edges (requires meta-schema)

Beyond ~15 types, transition from hardcoded to schema-driven architecture.

---

## Progressive Adoption Model

Inspired by the competitor analysis, specforge supports progressive adoption by complexity, not by version. All types are available from the start — teams adopt what they need.

### Adoption by Complexity

**Behaviors-only** — Adoptable in an afternoon. Start with invariants, decisions, behaviors, features, and capabilities. `specforge check` + `specforge trace` provide full value immediately.

```
invariant, decision, behavior, feature, capability, failure_mode, spec
```

**Product-level** — For teams building products, add code generation and delivery planning.

```
+ type, port, library, deliverable, roadmap
```

**Domain-specific** — For regulated industries, complex domains, or custom workflows. Use the meta-schema `define` mechanism or the plugin ecosystem.

```
+ compliance, glossary, custom types via meta-schema `define`
+ visual (as @specforge/views plugin), architecture cross-refs
```

A team using only behaviors gets full value from `specforge check` + `specforge trace` without ever touching libraries, deliverables, or compliance. This matches the adoption pattern of every successful tool in the competitive landscape.

---

## Final Node Type Summary

| Version | Types | Grammar Rules | Approach |
|---|---|---|---|
| **1.0** | spec, invariant, constraint, decision, behavior, event, feature, capability, failure_mode, type, port, glossary, library, deliverable, roadmap + meta-schema `define` | ~75+ | Hardcoded core + schema-driven extensions |

---

## Open Questions for Future Research

1. **Roadmap sub-entities:** Should phases, work items, and exit criteria be nested blocks or separate node types? The current skill defines 6+ entity types within roadmap alone.
2. **Visual as separate format:** Should `.view` files share the `.spec` parser or have their own tree-sitter grammar?
3. **Meta-schema validation power:** How much validation logic can be expressed declaratively vs. requiring procedural plugins?
4. **Plugin naming collision:** The compiler's "plugin" (RES-11a extensibility) vs. the spec's "plugin" (product extension points) need disambiguation.
5. **Architecture diagram integration:** What's the right boundary between specforge and Structurizr/Mermaid?
