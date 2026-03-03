---
name: specforge-libraries-dsl
description: "Write library blocks in .spec DSL files (@specforge/product plugin). Each library declares a code package with LIB-{infix}-{n} IDs, mapping features to ports with a validated dependency DAG. Use when making the relationship between code packages and spec-level features explicit."
---

# SpecForge Libraries DSL

Rules and conventions for authoring **`library` blocks** in `.spec` files. Libraries represent structural code units — they map features to ports and form a dependency DAG the compiler validates.

**Requires:** `@specforge/product` plugin.

## When to Use

- Mapping code packages to the features they implement
- Declaring which port interfaces a library defines
- Establishing library-to-library dependencies (validated DAG)
- Connecting the code layer to the product layer for deliverable coverage validation

## Block Syntax

```spec
use features/user-management
use features/auth

library LIB-MS-001 "@myservice/core" {
  family       core
  features     [FEAT-MS-001, FEAT-MS-004]
  depends_on   [LIB-MS-003]
  ports_defined [UserRepository, EmailService]
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). Typically the package name. |
| `features` | reference list | Features this library implements. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `depends_on` | reference list | Other libraries this library depends on. Validated DAG (no cycles). |
| `ports_defined` | reference list | Port interfaces defined by this library. |
| `family` | enum / string | Logical grouping: `core`, `platform`, `plugin`, `integration`. |
| `refs` | reference list | External references linked to this library. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `provides` | Library implements these features |
| `library` | `depends_on` | Library depends on these libraries |
| `port` | `defines_port` | Library defines these port interfaces |
| `ref` | `links_to` | External references linked to this library |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `library` | `depends_on` | Other libraries depend on this one |
| `deliverable` | `built_from` | Deliverable uses this library |

## Writing Rules

1. **Maps to real code packages** — each library corresponds to an npm package, Go module, Rust crate, etc.
2. **Features are what it implements** — the spec-level features this code delivers.
3. **`ports_defined` are the interfaces it owns** — the port interfaces defined in this package.
4. **`depends_on` forms a DAG** — circular library dependencies are a compile error (E007).
5. **Import feature and port files** — `use` the files declaring referenced entities.
6. **DSL scope is references, not package details** — npm name, version, and path belong in external tooling.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `features`, `depends_on`, `ports_defined` must resolve. |
| E002 | No duplicate library IDs. |
| E007 | Circular library dependency — `depends_on` edges must form a DAG. |
| W009 | Orphan library — not referenced by any deliverable. |

## Examples

### Platform Library

```spec
library LIB-MS-001 "@myservice/core" {
  family       platform
  features     [FEAT-MS-001, FEAT-MS-002]
  depends_on   [LIB-MS-010]
  ports_defined [UserRepository, TokenService]
  refs         [gh.pr:187]
}
```

### Integration Library

```spec
library LIB-MS-005 "@myservice/email" {
  family       integration
  features     [FEAT-MS-008]
  depends_on   [LIB-MS-001]
  ports_defined [EmailService]
}
```

### Minimal Library

```spec
library LIB-MS-010 "@myservice/search" {
  features     [FEAT-MS-005]
  ports_defined [SearchIndex]
}
```

## What NOT to Do

- Do not write libraries without the `@specforge/product` plugin installed
- Do not create circular dependencies between libraries — E007 error
- Do not confuse libraries (code packages) with deliverables (shippable artifacts)
- Do not put npm-specific details (version, package.json path) in library blocks — use external tooling
- Do not reference features, libraries, or ports from other files without `use` imports
