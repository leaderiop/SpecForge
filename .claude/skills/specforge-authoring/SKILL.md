---
name: specforge-authoring
description: "Orchestrator for writing .spec DSL files that the SpecForge compiler parses. Delegates to 16 entity-specific child skills for block syntax, field reference, relationship edges, and validation rules. Use when writing new .spec files, adding entities to existing .spec files, auditing .spec file completeness, or converting markdown specs to .spec DSL."
---

# SpecForge Authoring (Orchestrator)

Orchestrator skill for writing and maintaining `.spec` DSL files — the source-of-truth format that the SpecForge compiler parses into an in-memory graph, validates, and emits as markdown, JSON, traceability reports, and generated code.

**This skill writes `.spec` files (compiled DSL). Not markdown spec documents.**

## When to Use This Skill

- Writing a new `.spec` file for a SpecForge project
- Adding entity blocks (invariants, behaviors, features, etc.) to existing `.spec` files
- Auditing `.spec` file completeness against the entity model
- Converting existing markdown specs into `.spec` DSL format
- Determining which child skill to invoke for a specific entity type

## Delegation Table

| Entity Type | Child Skill | Naming Convention | Module |
|-------------|-------------|-------------------|--------|
| `spec` | **specforge-spec-block** | singleton | core |
| `invariant` | **specforge-invariants-dsl** | `identifier` | core |
| `behavior` | **specforge-behaviors-dsl** | `identifier` | core |
| `feature` | **specforge-features-dsl** | `identifier` | core |
| `event` | **specforge-events-dsl** | `identifier` | core |
| `type` | **specforge-types-dsl** | `identifier` | core |
| `port` | **specforge-ports-dsl** | `identifier` | core |
| `ref` | **specforge-refs-dsl** | `scheme.kind:identifier` | core |
| `capability` | **specforge-capabilities-dsl** | `identifier` | @specforge/product |
| `deliverable` | **specforge-deliverables-dsl** | `identifier` | @specforge/product |
| `roadmap` | **specforge-roadmaps-dsl** | `identifier` | @specforge/product |
| `library` | **specforge-libraries-dsl** | `identifier` | @specforge/product |
| `glossary` | **specforge-glossary-dsl** | singleton | @specforge/product |
| `decision` | **specforge-decisions-dsl** | `identifier` | @specforge/governance |
| `constraint` | **specforge-constraints-dsl** | `identifier` | @specforge/governance |
| `failure_mode` | **specforge-failure-modes-dsl** | `identifier` | @specforge/governance |

## Entity Naming

All entity IDs are **variable-name identifiers** (not sequential numeric prefixes):

- Any valid identifier (letters, digits, underscores, 2-60 chars): `data_persistence`, `UserRepository`, `validate_input`
- Titles are **optional** — auto-derived from name if omitted (`auth_login` → "Auth Login")
- Names must be 2-60 characters, letters/digits/underscores only
- Reserved words (the 16 entity keywords) cannot be used as names

## File Organization

Canonical `.spec` file structure for a SpecForge project:

```
specforge.json           # project config (preferred) — created by `specforge init`
specforge.spec           # spec block (legacy config) — only if specforge.json is absent
invariants/
  data.spec              # invariant blocks grouped by domain
  auth.spec
behaviors/
  user-crud.spec         # behavior blocks grouped by capability
  auth.spec
  order-processing.spec
features/
  user-management.spec   # feature blocks
  auth.spec
events/
  user-events.spec       # event blocks
  order-events.spec
types/
  user.spec              # type definitions grouped by domain
  order.spec
  common.spec
ports/
  user.spec              # port interfaces grouped by domain
  email.spec
governance/
  decisions.spec         # decision blocks (ADRs)
  constraints.spec       # constraint blocks (NFRs)
  failure-modes.spec     # failure_mode blocks (FMEA)
product/
  capabilities.spec      # capability blocks (UX flows)
  deliverables.spec      # deliverable blocks
  roadmap.spec           # roadmap blocks
  libraries.spec         # library blocks
glossary.spec            # glossary block (singleton)
```

Files can be split further by domain as the project grows. The directory structure is a convention, not a compiler requirement — the compiler discovers all `.spec` files under the spec root.

## Import System

### `use` Directives

Files reference symbols from other files via `use` directives at the top of the file:

```spec
use invariants/data                      // imports all symbols from invariants/data.spec
use invariants/data { data_persistence } // selective import
use behaviors/user-crud
use types/user
```

### Resolution Rules

- `use invariants/data` resolves to `<spec_root>/invariants/data.spec`
- The spec root is the directory containing `specforge.spec`
- Paths use forward slashes (platform-independent)
- The `.spec` extension is implicit — never include it
- All top-level declarations in a file are public by default
- A file MUST `use` another file to reference its symbols — enforced at compile time

### When to Use Imports

- Reference an entity declared in another file → add a `use` directive
- Reference an entity declared in the same file → no import needed
- Import cycles are a compile error (`E003`)

## Common Patterns

### Entity Ordering Within Files

Group related entities together. Within a file, order entities by:

1. Types (data shapes used by behaviors below)
2. Invariants (guarantees referenced by behaviors below)
3. Behaviors (contracts that reference types and invariants)
4. Events (produced by behaviors above)
5. Features (compose behaviors above)

### Grouping Conventions

- **By domain**: `user.spec`, `order.spec`, `billing.spec` — each file contains all entity types for that domain
- **By entity type**: `invariants/data.spec`, `behaviors/user-crud.spec` — separate directories per entity type
- **Hybrid**: types and ports in `types/` and `ports/`, behaviors and features in domain files

The entity-type-per-directory pattern is recommended for projects with >20 entities.

## Cross-Reference Conventions

### Entity Name References

Reference entities by their name in reference lists:

```spec
invariants [data_persistence, email_uniqueness]   // reference list
behaviors  [create_user, validate_email]
features   [user_management]
refs       [gh.issue:42, jira.epic:PROJ-123]
```

### Inline References in Prose

Use bracket syntax in triple-quoted strings:

```spec
contract """
  The system MUST validate email uniqueness per [email_uniqueness].
  See [gh.issue:42] for the original requirements.
"""
```

### Cross-File References

Always `use` the target file before referencing its entities:

```spec
use invariants/data
use types/user

behavior create_user "Create User" {
  invariants [data_persistence]    // declared in invariants/data.spec
  types      [User, UserRole]     // declared in types/user.spec
  contract "..."
}
```

## Validation Quick Reference

### Errors (compilation fails)

| Code | Description |
|------|-------------|
| E001 | Unresolved reference — name not found |
| E002 | Duplicate entity name — same name in multiple files |
| E003 | Circular import — `use` statements form a cycle |
| E005 | RPN mismatch — severity x occurrence x detection != rpn |
| E006 | Event trigger invalid — trigger must be an existing behavior |
| E007 | Circular library dependency |
| E008 | Persona not defined in spec root |
| E009 | Surface not defined in spec root |
| E011 | Invalid ref target format |
| E012 | Unknown provider kind |
| E013 | Reserved word used as identifier |
| E014 | Invalid identifier characters |

### Warnings (compilation succeeds)

| Code | Description |
|------|-------------|
| W001 | Orphan behavior — not in any feature |
| W002 | Orphan feature — not in any capability |
| W003 | Unused invariant — not referenced by any behavior |
| W004 | Unverified behavior — no `verify` statement |
| W005 | Unmitigated high-risk invariant |
| W006 | Unconstrained behavior |
| W007 | Orphan event — no consumers |
| W008 | Uncovered capability in deliverable |
| W009 | Orphan library |
| W010 | Deprecated feature |
| W011 | Orphan capability |
| W012 | Orphan ref — declared but never referenced |
| W013 | Vague entity name |

### Info

| Code | Description |
|------|-------------|
| I001 | Stale proposal — decision proposed >30 days ago |
| I003 | Newer format features available |
| I004 | Unknown entity in reference field — suggests installing a plugin |
| I005 | Unknown provider scheme — suggests installing a provider |

## Authoring Workflow

1. **Start with `specforge init`** — creates `specforge.json` (project config: name, version, plugins, providers, personas, surfaces, gen targets). Alternatively, invoke **specforge-spec-block** to declare config in a `spec` block (legacy path)
2. **Define types** — invoke **specforge-types-dsl** for domain data shapes
3. **Define ports** — invoke **specforge-ports-dsl** for interface contracts
4. **Write invariants** — invoke **specforge-invariants-dsl** for runtime guarantees
5. **Write behaviors** — invoke **specforge-behaviors-dsl** for behavioral contracts referencing invariants, types, and ports
6. **Define events** — invoke **specforge-events-dsl** for domain events produced by behaviors
7. **Compose features** — invoke **specforge-features-dsl** to group behaviors into user-facing capabilities
8. **Add refs** — invoke **specforge-refs-dsl** for external issue/ticket/design references
9. **Add product entities** (if @specforge/product installed):
   - **specforge-capabilities-dsl** for UX flows
   - **specforge-deliverables-dsl** for shippable artifacts
   - **specforge-libraries-dsl** for code packages
   - **specforge-roadmaps-dsl** for planning phases
   - **specforge-glossary-dsl** for domain vocabulary
10. **Add governance entities** (if @specforge/governance installed):
    - **specforge-decisions-dsl** for ADRs
    - **specforge-constraints-dsl** for NFRs
    - **specforge-failure-modes-dsl** for FMEA entries
11. **Validate** — run `specforge check` to verify all references resolve and no orphans exist

## What NOT to Do

- Do not mix markdown spec conventions with `.spec` DSL syntax — they are different formats for different purposes
- Do not duplicate entity names across files — each name is globally unique
- Do not use `use` for entities in the same file — only for cross-file references
- Do not hand-write `index.yaml` files — the compiler generates them
- Do not hand-write traceability matrices — `specforge trace` generates them
- Do not add traceability or overview as source entities — they are compiler outputs
- Do not exceed the ~15-20 hardcoded type budget — use `define` blocks for custom entity types
- Do not confuse plugins (entity model), providers (ref validation), and generators (output formats)
- Do not include the `.spec` extension in `use` paths — it is implicit
