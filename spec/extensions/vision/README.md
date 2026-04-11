# The Extension Ecosystem Vision

## Why Extensions Exist

SpecForge's compiler has zero built-in entity types. It is a pure typed-graph engine: parse, resolve, validate, export. All domain vocabulary comes from extensions. This is not a convenience feature. It is the conviction that makes SpecForge useful to domains its creators never imagined.

The test of this conviction: if a new domain requires a change to the compiler rather than a new extension, the architecture has failed.

This document defines the extension ecosystem — what each extension owns, how they compose, and the dependency hierarchy that governs their relationships.

---

## The Layer Model

SpecForge lives on the **spec layer only**. It never references implementation files (`.ts`, `.rs`, `.go`, `.py`, `.css`). The link to implementation goes through artifacts (JUnit XML, `specforge-report.json`), never through source file references. Only "bridge extensions" can cross the layer boundary.

```
SPEC LAYER (SpecForge lives here ONLY)
├── .spec files (entities, behaviors, invariants, types)
├── .feature files (Gherkin — acceptance criteria, still spec)
└── specforge.json (project config)

BRIDGE (extensions that cross the boundary)
├── @specforge/gherkin — reads .feature files into graph
├── @specforge/coverage — reads test artifacts (JUnit XML, reports)
└── @specforge/rust, @specforge/typescript — hypothetical code-layer experts

IMPLEMENTATION LAYER (SpecForge does NOT touch)
├── .ts .rs .go .py .css files
├── Test source files
└── Build artifacts
```

### Key Principles

1. **Core SpecForge + non-bridge extensions NEVER cross the boundary.** The compiler, product, software, governance, compliance — all live entirely on the spec layer.
2. **`.feature` files are spec-layer.** They describe WHAT (acceptance criteria), not HOW (implementation). Gherkin is a specification language, not a programming language.
3. **The link to code is always through artifacts.** Test results push up (JUnit XML → `specforge-report.json`). Specs don't reach down to source files.
4. **Bridge extensions are domain experts that CAN cross** (if installed). They are explicitly marked and their boundary-crossing is their entire purpose.

---

## The Dependency Hierarchy

```
                    CORE (spec layer only)
               ┌─────────────────────────────────┐
               │  graph engine, parser, resolver   │
               │  verify keyword, testable flag    │
               │  entity_enhancement               │
               │  (persona + channel in product)   │
               └────────────────┬─────────────────┘
                                │
  ┌──────────┬──────────┬───────┼───────┬──────────┬──────────┐
  │          │          │       │       │          │          │
  ▼          ▼          ▼       ▼       ▼          ▼          ▼
product   governance  compli-  feder-  embeddi-  markdown-  ...
(general)             ance     ation   ngs       renderer
  │
  │  software specializes product
  ▼
software
(specific)

BRIDGE EXTENSIONS (can cross to implementation layer):
  gherkin   — reads .feature files (spec↔spec actually)
  coverage  — reads test artifacts (JUnit XML, reports)
```

### The Chains

```
core → product → software     (domain specialization)
core → coverage                (traceability — entity-agnostic)
core → governance              (independent)
core → compliance              (independent)
core → federation              (independent)
core → gherkin                 (spec-layer file parser)
```

Product is general (any domain has deliverables and milestones). Software specializes it for code (behaviors, ports, invariants). Coverage is **entity-agnostic** — it traces ANY entity with `testable: true` in the extension manifest, regardless of which extension declared it. A compliance team with testable `control` entities uses coverage without software installed.

### The Independents

Coverage, governance, compliance, federation, gherkin, embeddings, and markdown-renderer all depend only on core. They can be installed standalone or alongside any combination of other extensions. Coverage is listed here because it discovers testable entities via the graph's `testable` flag — it never references specific entity kinds like "behavior" or "control."

---

## Core: What the Compiler Owns

The compiler is a typed-graph engine. It owns structural concepts only:

| Concept | What it is | Why it's core |
|---------|-----------|---------------|
| **Entity** | A named block with fields | Structural parsing — `keyword name { ... }` |
| **Edge** | A typed relationship between entities | Graph topology |
| **Reference** | A pointer from one entity to another | Resolution and validation |
| **Field** | A typed value on an entity | Data storage |
| **`verify`** | A test intent declaration | Structural keyword — any extension's entity can use it |
| **`testable` flag** | Manifest declaration on entity kinds | Structural — tells the compiler which entities can have verify |
| **`entity_enhancement`** | Fields an extension adds to another extension's entities | Structural — composition mechanism |
| **Channel** | A delivery channel (CLI, web, mobile, API) | Entity kind in `@specforge/product` |
| **Persona** | A user archetype | Entity kind in `@specforge/product` |
| **`surfaces.commands[]`** | CLI commands contributed by extensions | Dispatch mechanism — `cmd__{id}` Wasm exports |

The compiler parses ANY keyword. Validation of which keywords are legal comes from extensions. `verify` is a structural keyword the compiler knows; what it means (a behavior's acceptance criterion, a regulation's evidence requirement) is extension-defined semantics.

---

## @specforge/product — The General Layer

**Depends on:** core only.
**Depended on by:** @specforge/software (and any domain-specific specialization).

Product is the general layer that any domain-specific extension builds on. A compliance team, a design system team, a data pipeline team — all use product without software. Product answers: **what ships, to whom, when.**

### Entity Kinds (8)

| Entity | Purpose | Fields |
|--------|---------|--------|
| **journey** | What a persona can accomplish through a channel | `persona`, `channels`, `items` (generic reference list), `flow` (interaction steps) |
| **feature** | A user-facing unit of value (problem/solution) | `problem`, `solution`, `items` (generic reference list) |
| **deliverable** | A shippable artifact | `type` (open string), `journeys` |
| **milestone** | A planning phase with exit criteria | `status`, `items` (generic reference list), `exit_criteria` |
| **module** | A code package mapping features to ports with dependency DAG | `features`, `ports`, `dependencies` |
| **term** | Ubiquitous language entry with definition, aliases, context | `definition`, `aliases`, `context` |
| **persona** | A user archetype with goals and technical level | `description`, `technical_level`, `goals` |
| **channel** | An interaction medium (CLI, IDE, API, physical) | `description`, `interaction_model` |

### Design Principle: Generic Reference Fields

Product entities use **generic `items` fields** — reference lists that can point to any entity kind from any extension. In a software project, a journey's items are features. In a compliance project, they might be controls. In a design system project, components. Product does not know or care.

Domain-specific extensions enhance product entities with typed fields via `entity_enhancement`. Software adds `behaviors` on features, `behaviors` on milestones. This keeps product truly domain-agnostic while allowing specialization.

### Design Principle: Feature Is Domain-Neutral

Feature is deliberately domain-neutral: a **problem/solution pair** with no software assumptions. A compliance team uses features to describe regulatory capabilities. A design team uses features to describe user experience goals. The `problem` and `solution` fields are free-form text — they frame what matters without prescribing how.

Behaviors point UP to features via the `features [...]` field (cross-extension via peer_dependency). Features don't point down to behaviors. This keeps the dependency hierarchy clean: software depends on product, not vice versa.

### Validation Rules

| Code | Rule | Scope |
|------|------|-------|
| E007 | Module dependency cycle | Module has a dependency DAG |
| E008 | Undeclared persona reference | Validates against declared persona entity kinds |
| E009 | Undeclared channel reference | Validates against declared channel entity kinds |
| W041 | Orphan feature (not in any journey) | Pure product concern |
| W042 | Orphan journey (not in any deliverable) | Pure product concern |
| W043 | Deliverable with no journeys | Pure product concern |
| W044 | Orphan module (not in any deliverable) | Pure product concern |
| I010 | Unused term | Pure product concern |

### What Product Does NOT Own

- Behavior ranges in milestones — software-specific (contributed via entity_enhancement)
- Any concept that references code, tests, or implementation

---

## @specforge/software — The Software Specialization

**Depends on:** @specforge/product.
**Depended on by:** nothing (terminal specialization).

Software is the specialization for code. It adds everything a software engineering team needs to specify what the system does, how it's structured, and what must always be true. Software answers: **how is the software built.**

### Entity Kinds (5)

| Entity | Purpose | Testable |
|--------|---------|----------|
| **behavior** | A unit of work — what a function/method/operation does | Yes |
| **invariant** | A property that must always hold | Yes |
| **event** | A domain event, message, or signal | Yes |
| **type** | An algebraic data shape (struct, union, enum) | No |
| **port** | A hexagonal architecture boundary (inbound/outbound) | No |

### Entity Enhancements on Product

Software enriches product entities with software-specific fields:

| Product entity | Enhancement | What it adds |
|---------------|-------------|-------------|
| **feature** | `behaviors` field | Reference list targeting `behavior` entities |
| **milestone** | `behaviors` field | Reference list targeting `behavior` entities |
| **deliverable** | `modules` field | Reference list targeting `module` entities |

Behaviors also point UP to features via the `features [...]` field declared on the behavior entity kind itself. This is a native field (not an enhancement) because software depends on product — the reference direction follows the dependency DAG.

These enhancements only activate when both `@specforge/product` and `@specforge/software` are installed. A project with only `@specforge/product` never sees these fields.

### Validation Rules (Software-Specific)

| Code | Rule | Why it's software, not product |
|------|------|-------------------------------|
| E010 | Invalid behavior range in milestone | References software's `behavior` entity |

### Formal Methods

Software owns all formal methods constructs — they are unambiguously software-specific:

- **Design by Contract** — `requires`, `ensures`, `maintains` blocks on behaviors and invariants
- **B-Method** — `abstract`, `refines` annotations for stepwise refinement
- **CSP** — `sync` blocks on events for concurrent process modeling
- **Proof obligations** — verification conditions generated from formal annotations

### Surface Commands

- `specforge analyze contracts` — validate DbC consistency
- `specforge analyze refinement` — check B-Method refinement chains
- `specforge analyze concurrency` — detect CSP deadlocks and liveness issues

---

## @specforge/coverage — The Traceability Engine

**Depends on:** core only.
**Depended on by:** nothing.

Coverage closes the loop between what the spec says and what the tests prove. It is **entity-agnostic** — it traces the verify-to-test-to-result cycle for ANY entity with `testable: true` in the declaring extension's manifest. A software team traces behaviors. A compliance team traces controls. A game design team traces quest objectives. Coverage never knows which.

### What Coverage Owns

| Concern | What it provides |
|---------|-----------------|
| **`specforge-report.json` schema** | The universal report format: entity IDs, test names, pass/fail, duration |
| **`CoverageLevel` model** | Four levels: Declared, Specified, Executed, Passing |
| **Report merging** | Combines multiple report files into a single traceability matrix |
| **Coverage computation** | Counts testable entities at each level |
| **Threshold gating** | Fails the build if coverage drops below `--min=N` |
| **Traceability matrix** | Tabular output of entity → verify → test → status |

Coverage discovers testable entities by querying the graph for all entities whose kind has `testable: true`. It never imports or references entity kinds by name.

### Configuration

Coverage configuration is namespaced under the extension in `specforge.json`:

```json
{
  "extensions": {
    "@specforge/coverage": {
      "threshold": 80,
      "report_format": "summary"
    }
  }
}
```

It does NOT pollute core's config namespace. There is no `CoverageConfig` in core.

### Surface Commands

| Command | What it does |
|---------|-------------|
| `specforge collect` | Reads JUnit XML + mapping files, resolves entity IDs, produces `specforge-report.json` |
| `specforge coverage` | Merges reports, computes summary, gates on threshold |

Both are extension-contributed surface commands (`cmd__collect`, `cmd__coverage` Wasm exports), not core CLI commands.

### The Four-Level Model

```
Declared  →  Specified  →  Executed  →  Passing
  │             │             │            │
  │             │             │            └─ All results pass
  │             │             └─ Results in specforge-report.json
  │             └─ Gherkin scenarios exist in .feature files
  └─ verify declarations exist
```

No level requires opening a source code file. The link from spec to code is through artifacts (test results push up via `specforge-report.json`), not through file references reaching down to implementation.

Each level is a strict subset of the one before it. Everything passing was executed. Everything executed was specified. Everything specified was declared. The gaps between levels are where the work lives.

### Auto-Detection

`specforge collect` with no arguments auto-detects:
- Finds JUnit XML files in standard locations (`target/nextest/`, `test-results/`, etc.)
- Finds mapping files written by user-facing libraries (`target/specforge/*.json`)
- Applies two-level resolution: mapping files (explicit) > naming convention (implicit)
- When `@specforge/gherkin` is installed, `@specforge:entity_id` tags in `.feature` files provide an additional binding layer

### What Coverage Does NOT Own

- Language-specific test parsing — that's the user-facing library's job
- Test execution — that's the test runner's job
- Entity kind definitions — that's the declaring extension's job (software, compliance, etc.)
- The `verify` keyword — that's core's structural concept
- Knowledge of specific entity kinds — coverage sees `testable: true` entities, nothing more

---

## @specforge/gherkin — The Gherkin Interpreter

**Depends on:** core only.
**Depended on by:** nothing.

Gherkin is a **pure file parser extension** — it reads `.feature` files and enriches entity nodes with parsed Gherkin step data. It declares no entity kinds of its own.

### What Gherkin Does

| Concern | What it provides |
|---------|-----------------|
| **Contributes** | `parsers: true`, `grammars: true` |
| **Activates when** | Any field with `file_reference=true` matches `*.feature` |
| **Reads** | `.feature` files, parses Given/When/Then steps |
| **Produces** | Structured step data injected as metadata on the referencing entity node |
| **Binding mechanism** | `@specforge:entity_id` tag in `.feature` files (rename-safe, standard Gherkin) |

### Entity-Agnostic Design

Gherkin is entity-agnostic: it works with software behaviors (`gherkin` field), compliance controls, or any extension that declares a field with `file_reference=true` pointing to `.feature` files. A software team gets behavior nodes enriched with Given/When/Then steps. A compliance team gets control nodes enriched with regulatory acceptance criteria. Gherkin never knows which.

### The Binding Mechanism

```gherkin
@specforge:user_login
Feature: User login

  Scenario: Valid credentials
    Given a registered user
    When they submit valid credentials
    Then they are authenticated
```

The `@specforge:entity_id` tag binds scenarios to entities. This is standard Gherkin (tags are first-class), rename-safe (the tag travels with the scenario), and requires no custom syntax.

---

## BDD Support: Two Complementary Paths

SpecForge supports BDD through two complementary paths, not a dedicated `scenario` entity kind.

### Path A: Inline Scenarios in Journeys

Structured Given/When/Then blocks inside journey entities. These are **agent prompts**, not executable tests. AI agents get acceptance criteria in the graph response (~100 tokens/scenario). No `.feature` files needed.

```
journey user_login {
  persona developer
  surface cli

  scenario "valid credentials" {
    given "a registered user"
    when "they submit valid credentials"
    then "they are authenticated"
  }
}
```

### Path B: Gherkin Field on Behaviors

The `gherkin` field with `file_reference=true` on behavior entities. The `.feature` file is the source of truth. `@specforge/gherkin` parser enriches the behavior node. `@specforge:entity_id` tags bind scenarios to entities. Coverage traces via Cucumber JSON reports.

```
behavior user_login {
  contract "MUST authenticate users with valid credentials"
  gherkin ["features/auth.feature"]
  features [user_authentication]
}
```

### Why No `scenario` Entity Kind

10-expert consensus: a dedicated `scenario` entity kind adds friction ("writing spec twice"), creates sync rot between `.feature` files and `.spec` files, and violates "structure is a spectrum." The existing dual-path design already handles:

- **Agent consumption** — inline scenarios in journeys provide structured prompts
- **Traceability** — `gherkin` field + `@specforge:entity_id` tags bind to entities
- **Coverage** — Cucumber JSON reports feed into `specforge-report.json`

---

## User-Facing Libraries — NOT Extensions

Language-specific test integration is provided by standalone libraries published to language-specific package registries. They are NOT SpecForge extensions. They have no Wasm, no extension manifest, no dependency on the compiler. They write files that coverage reads.

### The Contract

Each library provides:
1. **An annotation mechanism** — language-idiomatic metadata linking tests to spec entities
2. **A mapping file writer** — writes `target/specforge/<binary>.json` (or equivalent) at test exit
3. **A naming convention** — documents `{entity_id}__{description_slug}` for zero-config fallback. The `__` double-underscore is a **reserved separator** — entity IDs must not contain `__` to avoid ambiguous parsing

Each library produces:
- **Mapping files** — JSON: `[{entity_id, entity_kind, test_name, verify_slug, status}]`
- **JUnit XML** — via the language's standard test runner (nextest, jest, pytest, gotestsum)

The contract is the file format, not a Wasm interface. Coverage reads the files. The libraries write them. Neither knows the other's implementation.

### Per-Language Libraries

| Library | Registry | Annotation | Test Runners |
|---------|----------|-----------|-------------|
| `specforge-test` | crates.io | `#[specforge::test(behavior = "id")]` proc macro | cargo test, nextest |
| `specforge-test-js` | npm | `/** @specforge behavior id */` JSDoc | vitest, jest |
| `specforge-test-py` | PyPI | `@specforge.test(behavior="id")` decorator | pytest |
| `specforge-test-go` | pkg.go.dev | `//specforge:behavior id` build tag | go test, gotestsum |
| `specforge-test-java` | Maven Central | `@SpecforgeTest(behavior = "id")` annotation | JUnit 5 |

Each is idiomatic in its language. The proc macro composes with `#[tokio::test]`. The decorator composes with `@pytest.mark`. The JSDoc comment composes with any test framework. No library replaces the test runner or claims ownership of the test lifecycle.

### Why Libraries, Not Extensions

1. **Zero Wasm overhead.** A Rust developer adds a dev-dependency. No Wasm compilation, no extension loading.
2. **Independent release cycles.** When nextest adds a new output format, `specforge-test` updates. The compiler doesn't know.
3. **No domain knowledge in the compiler.** The compiler never sees Rust, TypeScript, or Go. Coverage sees JUnit XML and mapping files. The boundary is clean.
4. **Natural distribution.** Rust developers find crates on crates.io. TypeScript developers find packages on npm. Each library lives where its users already look.

---

## @specforge/governance — Independent

**Depends on:** core only.
**Depended on by:** nothing.

Governance answers: **why is it built this way, and what are the risks.**

### Entity Kinds (4)

| Entity | Purpose | Testable |
|--------|---------|----------|
| **decision** | Architecture Decision Record — documents a choice, its context, and consequences | No |
| **constraint** | Non-functional requirement with measurable thresholds (performance, security, reliability) | Yes |
| **failure_mode** | FMEA entry — severity, occurrence, detection scores, RPN, mitigation | No |
| **technical_debt** | Tracked compromise — known shortcuts, deferred work, with cost/impact assessment | No |

Constraint is testable because non-functional requirements have measurable thresholds (response time < 200ms, uptime > 99.9%). Coverage traces constraint verification the same way it traces any testable entity.

Governance entities cross-reference other extensions' entities via soft graph edges (a constraint may reference a behavior, a decision may protect an invariant), but governance has no hard dependency on any other extension. It can be installed standalone for teams that track decisions and risks without software specs.

---

## @specforge/compliance — Independent

**Depends on:** core only.
**Depended on by:** nothing.

Compliance answers: **does the system meet regulatory requirements.**

### Entity Kinds (5)

| Entity | Purpose | Testable |
|--------|---------|----------|
| **regulation** | A regulatory requirement (GDPR article, FDA section, SOX control) | No |
| **control** | An implementation that satisfies a regulation | Yes |
| **evidence** | Proof that a control is in place (test result, audit log, document) | No |
| **audit** | An assessment of control effectiveness | No |
| **risk** | A threat or vulnerability with likelihood/impact assessment and mitigation | No |

Control is testable because regulatory controls have verifiable criteria (access control tests, encryption validation, data retention checks). When coverage is installed, controls participate in the same four-level traceability model as any other testable entity.

### Entity Enhancements on Product

Compliance enriches product entities with compliance-specific fields:

| Product entity | Enhancement | What it adds |
|---------------|-------------|-------------|
| **journey** | `controls` field | Reference list targeting `control` entities |
| **deliverable** | `regulations` field | Reference list targeting `regulation` entities |

Compliance has its own traceability chain: regulation → control → evidence → audit. This is separate from software's verify → test → result chain. A regulated team may use both: software for code traceability, compliance for regulatory traceability.

---

## @specforge/federation — Independent

**Depends on:** core only.
**Depended on by:** nothing.

Federation enables **cross-project graph linking**. A microservices team with ten repos can reference entities across project boundaries using `project::entity_id` syntax. Federation loads remote graphs, resolves cross-project references, and validates edge compatibility.

---

## @specforge/embeddings — Independent

**Depends on:** core only.
**Depended on by:** nothing.

Embeddings provides **semantic search over entities**. It generates vector embeddings from entity content, caches them by content hash, and returns ranked similarity results. An agent querying "what handles authentication?" gets the most relevant entities regardless of naming.

---

## @specforge/markdown-renderer — Independent

**Depends on:** core only.
**Depended on by:** nothing.

Markdown-renderer is a **renderer** — it transforms the entity graph into Markdown documentation. It generates index files grouped by entity kind, supports selective rendering, and produces human-readable output from the validated graph.

---

## The entity_enhancement Pattern

Extensions compose through `entity_enhancement` — one extension adds fields to another extension's entities without the target extension knowing about it.

```
// In @specforge/software's manifest:
entity_enhancements: [
  { target_kind: "feature",     fields: [{ name: "behaviors",  type: "reference_list", target: "behavior" }] },
  { target_kind: "milestone",   fields: [{ name: "behaviors",  type: "reference_list", target: "behavior" }] },
  { target_kind: "deliverable", fields: [{ name: "modules",    type: "reference_list", target: "module" }] },
]

// In @specforge/compliance's manifest:
entity_enhancements: [
  { target_kind: "journey",     fields: [{ name: "controls",    type: "reference_list", target: "control" }] },
  { target_kind: "deliverable", fields: [{ name: "regulations", type: "reference_list", target: "regulation" }] },
]
```

When `@specforge/product` and `@specforge/software` are both installed, feature gains a `behaviors` field. When only `@specforge/product` is installed, feature has only its generic `items` field. The product extension never imports software. Software enhances product. The dependency flows one way.

This is how the ecosystem scales. `@specforge/compliance` enhances `journey` with a `controls` field. `@specforge/atomic-design` enhances `deliverable` with a `components` field. Each enhancement composes without coordination.

### Rules

1. **Enhancements follow the dependency DAG.** If extension A depends on extension B, A may enhance B's entities. B must not enhance A's entities. Circular enhancement is forbidden — the loader rejects it.
2. **Minimum peer version.** Enhancements declare a minimum version of the target extension (`peer_dependencies`). If coverage v2 expects a field that software v1 doesn't declare, the manifest loader surfaces a version mismatch diagnostic before graph construction.
3. **Graceful absence.** When the target extension is not installed, enhancements are silently ignored. No errors, no warnings — the fields simply don't exist.

---

## Extension Combinations

### Software Team (most common)
```
@specforge/product + @specforge/software + @specforge/gherkin + @specforge/coverage + @specforge/governance
```
Full stack: what ships (product), how it's built (software), BDD scenarios (gherkin), is it tested (coverage), why decisions were made (governance).

### Regulated Software Team
```
@specforge/product + @specforge/software + @specforge/coverage + @specforge/compliance
```
Software traceability plus regulatory traceability. Two independent chains in one graph.

### Compliance-Only Team
```
@specforge/product + @specforge/compliance
```
No software entities. Journeys map to controls (via entity_enhancement from compliance). Milestones schedule regulatory checkpoints.

### Compliance with BDD
```
@specforge/product + @specforge/compliance + @specforge/gherkin + @specforge/coverage
```
Controls are testable. Gherkin scenarios describe regulatory acceptance criteria. Coverage traces control verification — four-level model, threshold gating, traceability matrix. No software extension needed.

### Design System Team
```
@specforge/product + @specforge/atomic-design
```
No software entities. Journeys map to design components. Deliverables are component modules.

### Polyglot Microservices
```
@specforge/product + @specforge/software + @specforge/coverage + @specforge/federation
```
Multiple repos, multiple languages, one federated graph. Each repo has its own spec files. Federation links them.

---

## What This Is NOT

**Not a plugin marketplace.** Extensions are not optional nice-to-haves. They ARE the domain vocabulary. Without at least one extension, SpecForge has nothing to say.

**Not a monolith.** Each extension is independently versioned, independently developed, and independently installable. The only ordering constraint is `core → product → software` for domain specialization. Coverage, governance, compliance, federation, gherkin, and all others depend only on core.

**Not locked to first-party extensions.** A maritime logistics company builds `@specforge/shipping`. A game studio builds `@specforge/game-design`. A clinical research group builds `@specforge/clinical-trials`. The compiler does not change. Only extensions exist.

---

*The graph is the product. Extensions give it vocabulary. The compiler gives it truth. Together, they make AI agents reliable.*
