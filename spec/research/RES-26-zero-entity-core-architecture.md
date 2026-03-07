# RES-26: Zero-Entity Core Architecture

**Date:** 2026-03-04
**Status:** ACCEPTED — This is THE architectural direction for SpecForge
**Synthesis:** 3-expert analysis (compiler architect, product strategist, Rust engineer)
**Inspired by:** Meta FAIR Large Concept Models research (LCM, December 2024)

---

## Executive Summary

SpecForge's core compiler should have **ZERO built-in entity types**. The compiler becomes a pure typed-graph engine. ALL domain vocabulary — every entity keyword, every edge type, every validation rule — comes from installable extensions.

This completes SpecForge's repositioning from "specification compiler for software projects" to **"the structured context standard for AI agents across all domains."**

The Terraform analogy is now exact:
- **Terraform core**: HCL parser + state engine + provider interface. Zero infrastructure knowledge.
- **SpecForge core**: .spec parser + graph engine + extension interface. Zero domain knowledge.

---

## 1. Architecture Overview

```
┌──────────────────────────────────────────────────────┐
│  SpecForge Core (zero domain knowledge)              │
│  ├── Tree-sitter parser (generic entity_block)       │
│  ├── Graph engine (petgraph, nodes + edges)          │
│  ├── KindRegistry + FieldRegistry (empty at boot)    │
│  ├── Reference resolver (cross-file, cross-extension)  │
│  ├── Structural validation (cycles, dupes, dangling) │
│  ├── Wasm runtime (loads extensions)                   │
│  ├── Export engine (Graph Protocol, JSON, DOT)       │
│  └── LSP framework (dynamic keywords from registry)  │
├──────────────────────────────────────────────────────┤
│  Extensions (ALL domain knowledge)                     │
│  ├── @specforge/software                         │
│  ├── @specforge/product                              │
│  ├── @specforge/governance                           │
│  ├── @specforge/atomic-design                        │
│  ├── @specforge/compliance                           │
│  ├── @specforge/api-design                           │
│  ├── @specforge/data-modeling                        │
│  ├── @specforge/business-model                       │
│  ├── @specforge/security                             │
│  ├── @specforge/infrastructure                       │
│  └── ... any domain ...                              │
└──────────────────────────────────────────────────────┘
```

An empty SpecForge install has NO keywords except `spec` (project config), `use` (imports), and `define` (inline meta-schema). Users run `specforge add @specforge/software` to get the software engineering vocabulary.

---

## 2. What Stays in Core

| Concern | Why Core |
|---------|----------|
| `spec` block parsing | Bootstraps config, loads specforge.json |
| `use` / import system | File resolution is structural |
| `define` blocks | Inline meta-schema for user-defined types |
| Generic `entity_block` parser | Parses any `keyword id { fields }` without knowing keywords |
| Graph engine (petgraph) | Domain-agnostic node/edge operations |
| KindRegistry + FieldRegistry | Populated by extensions at startup, empty by default |
| Structural validation (~15 codes) | E001 dangling ref, E002 duplicate, E003 cycles, etc. |
| Wasm runtime (Extism) | Loads and executes extensions |
| Export / Graph Protocol | Serializes any graph regardless of domain |
| LSP framework | Dynamic keyword completion, semantic tokens from registry |
| Diagnostic infrastructure | Severity, SourceSpan, ariadne rendering |
| String interning (lasso) | Performance, used for all keywords and IDs |

### What Core Does NOT Know

- What "behavior" means
- What "invariant" means
- What edge types exist
- Which entities are testable
- Which entities support verify/scenario syntax
- What validation rules apply to specific entity types
- What LSP icons or DOT shapes to use for specific entities

ALL of this comes from extensions.

---

## 3. What Moves to Extensions

| Current Location | Moves To |
|------------------|----------|
| `EntityKind` enum (16 variants) | Extension manifests |
| `EdgeType` enum (20 variants) | Extension manifests (field→edge mappings) |
| `EntityKind::is_testable()` | `testable: bool` in extension entity descriptor |
| `EntityKind::supports_gherkin()` | `supports_scenario: bool` in extension entity descriptor |
| `EntityKind::is_singleton()` | `singleton: bool` in extension entity descriptor |
| `EdgeType::from_field_name()` | Field-to-edge mapping in extension manifest |
| `EdgeType::target_kind()` | Target kind in extension manifest edge declarations |
| ~20 domain-specific validation codes | Declarative rules in extension manifests |
| LSP completion keywords | `KindRegistry.all_known_kinds()` |
| LSP semantic token types | `semantic_token` field per entity kind |
| DOT graph shapes | `dot_shape` field per entity kind |

---

## 4. Extension Manifest v2

Each vocabulary extension declares its entity kinds, edge types, validation rules, and metadata:

```json
{
  "extension": "@specforge/software",
  "manifest_version": "2",
  "version": "1.0.0",

  "entity_kinds": [
    {
      "name": "behavior",
      "keyword": "behavior",
      "testable": true,
      "singleton": false,
      "supports_verify": true,
      "supports_scenario": true,
      "allowed_verify_kinds": ["unit", "integration", "property", "load"],
      "fields": [
        { "name": "implements", "type": "reference_list", "edge": "implements", "target_kind": "feature" },
        { "name": "produces", "type": "reference_list", "edge": "produces", "target_kind": "event" },
        { "name": "consumes", "type": "reference_list", "edge": "consumes", "target_kind": "event" },
        { "name": "uses_type", "type": "reference_list", "edge": "uses_type", "target_kind": "type" },
        { "name": "uses_port", "type": "reference_list", "edge": "uses_port", "target_kind": "port" },
        { "name": "enforced_by", "type": "reference_list", "edge": "enforces", "target_kind": "behavior" },
        { "name": "tests", "type": "string_list" },
        { "name": "gherkin", "type": "string_list", "file_reference": true }
      ],
      "semantic_token": "keyword",
      "lsp_icon": "method",
      "dot_shape": "box"
    }
  ],

  "edge_types": [
    { "label": "implements", "description": "Feature implements behavior" },
    { "label": "produces", "description": "Behavior produces event" },
    { "label": "consumes", "description": "Behavior consumes event" }
  ],

  "validation_rules": [
    {
      "code": "W001",
      "severity": "warning",
      "message_template": "behavior `{id}` is not referenced by any feature",
      "check": "no_incoming_edges",
      "target_kind": "behavior",
      "edge_type": "implements"
    }
  ],

  "verify_kinds": ["unit", "integration", "property", "load", "e2e"]
}
```

Key additions over current `WasmEntityKind`:
- `singleton`, `supports_verify`, `supports_scenario`, `allowed_verify_kinds`
- `fields[].edge` and `fields[].target_kind` (replaces EdgeType dispatch tables)
- `fields[].file_reference` (for file-not-found validation)
- `semantic_token`, `lsp_icon`, `dot_shape` (editor metadata)
- `validation_rules` (declarative — core interprets, no Wasm needed for simple checks)
- `verify_kinds` (extension-defined verification kinds)

---

## 5. Bootstrapping: Two-Phase Parsing

**Problem:** How does the compiler parse `.spec` files if it doesn't know what keywords are valid until extensions are loaded?

**Solution:** The tree-sitter grammar parses generically. Semantic validation happens after extension loading.

### Phase 1 — Structural Parse (zero knowledge)

The grammar has ONE generic entity block rule:

```javascript
entity_block: ($) => seq(
    field("keyword", $.identifier),
    optional(field("id", $.identifier)),
    optional(field("title", $.string)),
    "{", repeat(choice($.key_value, $.verify_statement, $.scenario_block)), "}"
)
```

Only three keywords stay hardcoded: `spec`, `use`, `define`.

### Phase 2 — Semantic Validation (after extension loading)

1. Read `specforge.json` → get extension list (pure JSON, no .spec parsing needed)
2. Load extension manifests → populate KindRegistry and FieldRegistry
3. Parse `.spec` files → tree-sitter produces generic CST
4. CST-to-AST → validates each keyword against KindRegistry
5. Unknown keywords → diagnostic: `error[E024]: unknown entity kind 'behavior' — did you mean to install @specforge/software?`

**No circular dependency.** Extension list comes from JSON; parsing comes after.

---

## 6. Validation: Core vs Extension

### Core Validation (~15 codes — structural, domain-agnostic)

| Code | Check |
|------|-------|
| E001 | Unresolved reference |
| E002 | Duplicate entity ID |
| E003 | Circular import |
| E010 | Syntax error |
| E013 | Reserved word as identifier |
| E014 | Invalid identifier characters |
| E024 | Unknown entity kind (not declared by any extension) |
| I003 | Format version mismatch |
| I004 | Soft cross-extension reference |
| I005 | Provider not installed |
| W017 | Unused entity (generic) |

### Extension Validation (~20+ codes — semantic, domain-specific)

Extensions declare validation rules using a small set of declarative patterns:

1. **`no_incoming_edges(kind, edge_type)`** — orphan checks (W001, W003, W007...)
2. **`no_outgoing_edges(kind, edge_type)`** — missing links (W006, W019...)
3. **`missing_field_on_testable(field)`** — coverage gaps (W004, W018)
4. **`field_value_constraint(kind, field, predicate)`** — semantic checks (E005, E006)
5. **`cycle_detection(edge_type)`** — circular deps (E007)
6. **`file_exists(kind, field)`** — file references (E016)

For complex validation beyond these patterns, extensions implement a Wasm `validate()` function.

---

## 7. Backward Compatibility

### Migration Path

1. **Auto-detection:** v2 compiler encounters keywords without extensions → suggests `specforge add @specforge/software`
2. **`specforge migrate`:** Adds required extensions to `specforge.json`
3. **Deprecation period:** For 2-3 minor versions, compiler ships `@specforge/software` as bundled default
4. **Clean break:** Next major version, no implicit extensions

### Spec Files Don't Change

```spec
// Before (v1): compiler hardcodes "behavior" keyword
behavior auth_login "User Login" {
  verify unit "validates credentials"
}

// After (v2): @specforge/software extension declares "behavior" keyword
// THE SPEC FILE IS IDENTICAL
behavior auth_login "User Login" {
  verify unit "validates credentials"
}
```

Only `specforge.json` changes — users must declare which vocabulary extensions they use.

---

## 8. Domain Extension Catalog

### Designed Extensions (15)

| Extension | Domain | Entity Types | AI Agent Use Case |
|---------|--------|-------------|-------------------|
| `@specforge/software` | Software engineering | behavior, invariant, feature, event, type, port | Coding agents produce correct code from structured specs |
| `@specforge/product` | Product management | capability, deliverable, roadmap, library, glossary | PM agents produce status reports, track delivery |
| `@specforge/governance` | Technical governance | decision, constraint, failure_mode | Architecture agents enforce decisions, assess risk |
| `@specforge/atomic-design` | UI/UX design | atom, molecule, organism, template, page | Design agents maintain component hierarchy, trace usage |
| `@specforge/business-model` | Business strategy | value_proposition, customer_segment, channel, revenue_stream, cost_structure | Strategy agents analyze market fit, generate pitch materials |
| `@specforge/compliance` | Regulatory | regulation, control, evidence, audit_finding, risk_assessment | Compliance agents generate audit trails, map controls to regulations |
| `@specforge/data-modeling` | Data architecture | schema, field_group, pipeline, lineage, migration | Data agents trace lineage, assess migration impact |
| `@specforge/api-design` | API contracts | endpoint, request_schema, response_schema, auth_scheme, rate_limit | API agents generate contracts, validate consistency |
| `@specforge/infrastructure` | Cloud/ops | service, deployment, network, secret, health_check | Infra agents plan provisioning, trace dependencies |
| `@specforge/security` | Security | threat, vulnerability, control, incident, attack_surface | Security agents assess risk, generate threat models |
| `@specforge/research` | Scientific research | hypothesis, experiment, dataset, finding, protocol | Research agents organize methodology, track results |
| `@specforge/education` | Curriculum design | curriculum, learning_objective, assessment, module, prerequisite | Education agents generate course material, validate coverage |
| `@specforge/legal` | Legal contracts | contract, clause, obligation, party, amendment | Legal agents track obligations, identify conflicts |
| `@specforge/okr` | Goal tracking | objective, key_result, initiative, metric, review | OKR agents track progress, identify misalignment |
| `@specforge/user-stories` | Agile PM | epic, story, acceptance_criteria, sprint, persona | Agile agents manage backlog, trace stories to implementation |

### Example: @specforge/atomic-design

```spec
atom primary_button "Primary Button" {
  description "Standard CTA button with brand colors"
  props {
    label: string
    onClick: callback
    disabled: boolean
    variant: enum [solid, outline, ghost]
  }
  tokens [color_primary, spacing_md, radius_sm]

  verify visual "renders correctly in all variants"
  verify a11y "meets WCAG AA contrast ratio"
}

molecule search_bar "Search Bar" {
  atoms [text_input, icon_button, primary_button]
  description "Combined search input with submit action"

  verify integration "debounces input and triggers search"
}

organism navigation_header "Navigation Header" {
  molecules [search_bar, user_menu, nav_links]
  template_slot header
  responsive {
    mobile: "hamburger menu"
    desktop: "full horizontal nav"
  }
}
```

### Example: @specforge/compliance

```spec
regulation gdpr_article_17 "GDPR Article 17 — Right to Erasure" {
  jurisdiction eu
  effective_date "2018-05-25"
  controls [data_deletion_control, erasure_request_handling]
}

control data_deletion_control "Data Deletion Control" {
  implements [gdpr_article_17]
  description "Ensures personal data is deleted within 30 days of request"
  evidence [deletion_audit_log, request_tracking_report]

  verify integration "deletion completes within SLA"
  verify audit "audit log captures all deletion events"
}

audit_finding finding_001 "Incomplete Deletion Logging" {
  control data_deletion_control
  severity high
  status open
  description "Audit log missing entries for batch deletion operations"
  remediation "Add batch operation logging to deletion service"
}
```

---

## 9. Codebase Impact Assessment

### Quantitative Summary

| Metric | Value |
|--------|-------|
| Rust source files needing change | ~25 files |
| Lines to refactor | ~2,500-3,000 (~10% of 28,233 LOC) |
| `EntityKind` references to eliminate | 496 across 25 files |
| `EdgeType` references to eliminate | ~200 across 15 files |
| Validation codes moving to extensions | ~20 of 38 |
| Infrastructure already built | ~60% |
| **Estimated effort** | **5-7 weeks (phased)** |

### Key Files Requiring Change

| File | Change |
|------|--------|
| `entity_kind.rs` | `EntityKind` enum → interned string wrapper |
| `edge_type.rs` | `EdgeType` enum → interned string wrapper |
| `grammar.js` | 16 specific `*_block` rules → 1 generic `entity_block` |
| `kind_registry.rs` | `with_builtins()` → `new()` (empty) |
| `field_registry.rs` | `with_builtins()` → `new()` (empty) |
| `cst_to_ast.rs` | 16 block handlers → 1 generic handler |
| `passes.rs` | ~20 validation passes → declarative extension rules |
| `manifest.rs` | `WasmEntityKind` expanded with 8+ new fields |
| `completion.rs` | Hardcoded keywords → registry queries |
| `semantic_tokens.rs` | Hardcoded tokens → registry-driven |

### What's Already Built (60%)

- **`EntityKind::Custom(String)`** — the escape hatch that becomes the only variant
- **`KindRegistry`** — manages builtin + plugin + define with conflict detection
- **`FieldRegistry`** — fully data-driven, looks up by string not enum
- **`WasmEntityKind`** — already has name, testable, fields, reference_targets
- **`qualified_entity_block`** — grammar already parses `@extension/kind id { fields }`
- **Graph builder** — already generic, uses string-based entity_kind

---

## 10. Implementation Phases

| Phase | Scope | Duration |
|-------|-------|----------|
| **1. String-ify EntityKind + EdgeType** | Replace enums with interned strings, keep grammar hardcoded for now | 2-3 weeks |
| **2. Generic grammar** | Collapse 16 `*_block` rules into 1 `entity_block` | 2-3 weeks |
| **3. Extract @specforge/software** | Move 6 core entities (behavior, invariant, feature, event, type, port — per RES-27) + edge types + validation to extension manifest | 2-3 weeks |
| **4. Extract @specforge/product + governance** | Move remaining 8 entities + 11 edge types | 1-2 weeks |
| **5. Migration + backward compat** | `specforge migrate`, auto-detection, deprecation | 1-2 weeks |
| **6. First non-software extension** | `@specforge/atomic-design` or `@specforge/compliance` as proof of concept | 1-2 weeks |
| **7+. Ecosystem growth** | Community extensions, registry, marketplace | Ongoing |

---

## 11. Competitive Positioning After Pivot

| Before | After |
|--------|-------|
| "A specification language for software engineering" | **"A typed-graph engine for domain specifications"** |
| Competes with Gherkin, OpenAPI, ADR tools | Competes with nothing, enables everything |
| Serves software teams | Serves **any team using AI agents for structured work** |
| 16 hardcoded entity types | Unlimited domain vocabularies via extensions |
| TAM: AI-assisted development ($8B→$127B) | TAM: AI agent infrastructure across all domains |

The Terraform trajectory:
1. Terraform started as "AWS provisioning" → became "infrastructure as code for any provider"
2. SpecForge starts as "software specification" → becomes "specification as a graph for any domain"

---

## 12. The Naming Question Is Resolved

With zero-entity core, users never see a generic umbrella term. They see extension-defined keywords: `behavior`, `atom`, `regulation`, `endpoint`. The internal term ("entity", "node", "block") is invisible infrastructure.

**The entity/concept/definition debate is moot.** The vocabulary IS the extension.

---

## 13. LCM-Inspired Insights That Led Here

Meta FAIR's Large Concept Models research showed that operating at the right abstraction level yields outsized gains. The key transfer to SpecForge:

1. **Abstraction level matters** — LCMs moved from tokens to concepts. SpecForge moves from hardcoded entity types to domain-defined vocabularies.
2. **Language-agnostic space** — LCMs use SONAR to map 200 languages to one space. SpecForge's Graph Protocol maps any domain vocabulary to one graph format.
3. **Multi-resolution queries** — LCMs showed fixed granularity is a weakness. SpecForge's multi-resolution graph queries let agents request context at any zoom level.

The zero-entity core is the logical conclusion: if SpecForge is the structured context standard for ALL AI agents, it cannot hardcode one domain's vocabulary.

---

*RES-26. 3-expert synthesis. This is the foundational architectural direction for SpecForge.*
