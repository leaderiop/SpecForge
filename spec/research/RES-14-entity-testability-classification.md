---
id: RES-14
kind: research
title: "Entity Testability Classification — Which Spec Concepts Require Test Coverage?"
status: active
date: 2026-03-02
depends_on: []
---

# RES-14: Entity Testability Classification


## Problem Statement

SpecForge's standard extensions define 14 domain entities across `@specforge/software` (6), `@specforge/product` (5), and `@specforge/governance` (3) extensions (the core compiler itself has zero built-in entity types — see RES-26). When an AI agent or team uses SpecForge to specify a software system, **which of these concepts represent things that need automated test coverage in the target software**, and which are purely declarative metadata that the compiler validates structurally?

This distinction drives:
- **Test declaration**: which entities support `verify` and `scenario` blocks (RES-15)
- **Traceability**: which entities require `tests` field linkage to real test files (RES-15)
- **Coverage reporting**: which entities count toward "spec coverage" in `specforge trace`
- **Coverage reporting**: which entities require test coverage in the target software
- **Agent workflow**: which entities an agent must implement AND test vs. only implement
- **Extension API**: `EntityKind::is_testable()` for extension-defined entities

This analysis was produced by 10 specialized expert agents, each analyzing entities independently, then consolidated into a unified matrix.

---

## Classification Criteria

An entity is **TESTABLE** if it satisfies all three:

1. **Runtime presence** — it describes something that exists or happens when the software runs
2. **Falsifiability** — you can write a test that detects a violation
3. **Test declaration support** — the entity supports `verify` statements, `scenario` blocks, or both

An entity is **DECLARATIVE** if it:

1. Describes structure, metadata, or organizational groupings
2. Has no runtime behavior of its own
3. Is validated through methods appropriate to the domain — compiler structural checks for software, governance audits for compliance, product reviews for business contexts, etc. — rather than requiring automated test suites in target software

---

## Consolidated Matrix

| # | Entity | Extension | Classification | Test Syntax | Rationale |
|---|--------|---------|---------------|-------------|-----------|
| 1 | **behavior** | @specforge/software | **TESTABLE** | verify only | The atomic unit of "what the system does." Has `contract` (MUST/SHALL) + `verify` statements. Scenarios restricted to capability only (RES-15). |
| 2 | **invariant** | @specforge/software | **TESTABLE** | verify only | Runtime guarantee that must always hold. Falsifiable by definition. `verify property` is the natural fit. |
| 3 | **event** | @specforge/software | **TESTABLE** | verify only | Runtime phenomenon: trigger, payload shape, channel, consumers. Testable via producer/consumer/contract tests. |
| 4 | **constraint** | @specforge/governance | **TESTABLE** | verify only | Measurable NFR with thresholds (e.g., "p99 < 200ms"). `verify load/security` is the natural fit. |
| 5 | **capability** | @specforge/product | **TESTABLE** | verify e2e¹ + scenario | UX flow with persona + surface. Scenario for structured acceptance criteria. `verify e2e` for one-liner test declarations. |
| 6 | feature | @specforge/software | Declarative | — | Organizational grouping of behaviors. Testing its behaviors suffices. |
| 7 | type | @specforge/software | Declarative | — | Data shape definition (struct/union/enum). No runtime logic. |
| 8 | port | @specforge/software | Declarative | — | Interface contract (method signatures). Implementations are tested, not the definition. |
| 9 | spec | Core (structural) | Declarative | — | Project configuration. Configures the compiler, not the target software. |
| 10 | ref | Core (structural) | Declarative | — | External link (gh.issue:42). Metadata anchor with zero runtime behavior. |
| 11 | deliverable | @specforge/product | Declarative | — | Shipping manifest — bundles capabilities into artifacts. |
| 12 | library | @specforge/product | Declarative | — | Code package declaration. Structural metadata, not executable. |
| 13 | roadmap | @specforge/product | Declarative | — | Planning phase with status lifecycle. Temporal/scheduling metadata. |
| 14 | decision | @specforge/governance | Declarative | — | ADR documenting "why." Historical rationale, no runtime behavior. |
| 15 | failure_mode | @specforge/governance | Declarative | — | FMEA risk entry. Informs what to test, but is not itself testable. |
| 16 | glossary | @specforge/product | Declarative | — | Vocabulary definitions. Pure reference material. |

**Result: 5 testable / 11 declarative (= 16 total rows: 14 domain entities from 3 extensions + 2 structural core keywords).**

---

## Entity × Test Syntax Matrix (aligned with RES-15)

```
                    verify    scenario    tests field
                    ──────    ────────    ───────────
  behavior           ✅         ❌           ✅
  invariant          ✅         ❌           ✅
  event              ✅         ❌           ✅
  constraint         ✅         ❌           ✅
  capability         ⚠️¹        ✅           ✅
  ─────────────────────────────────────────────────
  (11 declarative)   ❌         ❌           ❌
```

- **verify**: One-liner test intent. 5 kinds: `unit`, `integration`, `property`, `load`, `e2e`.
- **scenario**: Structured Given/When/Then acceptance criteria. Only where flows make sense.
- **tests**: File linkage to actual executable tests. The PRIMARY traceability mechanism.

¹ `verify e2e` on capability is supported but deprecated in v1.0 and will be removed in v2.0 (RES-15). Prefer `scenario` blocks for capabilities.

---

## Testable Entity Deep Dive

### 1. behavior — Primary test unit

**What it specifies:** A single atomic operation the system performs (e.g., "Create User", "Place Order").

**Why testable:**
- `contract` field uses RFC 2119 keywords (MUST, SHALL, SHOULD) defining precise, verifiable guarantees
- `verify` statements declare test type and expected outcome
- Scenarios restricted to capability only (RES-15)
- `tests` field links to actual test files for traceability
- Has triggers, state changes, return values, and error conditions — all observable

**What to test:**
- Preconditions/triggers activate under correct conditions
- State changes match contract (e.g., "MUST create a user record")
- Return values match declared types (e.g., `Result<User, DuplicateEmailError>`)
- Error handling produces specified error types
- Side effects fire (events emitted, records created)
- Referenced invariants hold before and after execution

**Test types:** Unit, integration, property-based, load.

**Agent workflow:** Agent reads `contract` + `verify` statements → writes implementation + test file → fills `tests` field → `specforge trace` validates.

### 2. invariant — Strongest guarantee

**What it specifies:** A runtime truth that must always hold (e.g., "No two active users share the same email").

**Why testable:**
- By definition falsifiable — if you cannot write a failing test, it is not an invariant
- `guarantee` statement is the assertion
- `enforced_by` references the code/behavior that upholds it
- `verify` statements describe how to confirm it holds

**What to test:**
- Positive: guarantee holds under normal operation
- Negative: attempt to violate the invariant, verify the system prevents it
- Boundary: guarantee holds under failure conditions, high load, edge cases
- Enforcement: the `enforced_by` components actually enforce the guarantee

**Test types:** Property-based (strongest fit), unit, integration.

**Why no scenario:** Invariants are timeless universal guarantees, not sequential flows. "Email uniqueness always holds" doesn't have a Given/When/Then structure — it's a property that must hold for ALL inputs.

### 3. event — Reactive glue

**What it specifies:** Something that happens in the system at runtime — produced by a behavior, consumed by others.

**Why testable:**
- `trigger` behavior must actually emit the event
- `payload` must match the declared type schema
- `channel` must be the correct routing destination
- `consumers` must receive and handle the event

**What to test:**
- Producer integration: trigger behavior emits event with correct payload
- Payload shape: emitted data matches declared type
- Channel routing: event published to correct channel/topic/queue
- Consumer integration: each declared consumer receives and processes the event
- Contract tests: producer and consumer agree on payload schema

**Test types:** Integration, contract.

**Why no scenario:** Events are data contracts between producers and consumers. Testing is about payload shape and delivery, not step-by-step user flows.

### 4. constraint — Quality attributes

**What it specifies:** A measurable non-functional requirement with a threshold (e.g., "API p99 < 200ms under 1000 concurrent users").

**Why testable:**
- Defines a falsifiable threshold — either the system meets it or it does not
- `verify` field explicitly declares how to test it (e.g., "k6 load test with 1000 VUs")
- Categories (performance, security, reliability, compatibility) each have established testing patterns

**What to test:**
- Performance: load tests, latency measurements, throughput benchmarks
- Security: penetration tests, encryption audits, OWASP compliance
- Reliability: chaos engineering, fault injection, recovery time measurements
- Compatibility: CI matrix across platforms/runtimes

**Test types:** Load, security, chaos, compliance.

**Why no scenario:** Constraints are measurable thresholds. "p99 < 200ms" isn't a flow — it's a metric. Load tests and security scans don't fit Given/When/Then.

### 5. capability — User flow (acceptance gate)

**What it specifies:** A UX flow — what a persona can do on a surface, with success/failure paths.

**Why testable:**
- `scenario` blocks define concrete Given/When/Then steps — structured acceptance criteria
- `verify e2e` provides quick one-liner test declarations
- Maps persona + surface to a testable user flow
- Bundles multiple features/behaviors into an integration scenario

**What to test:**
- Each step in the scenario (e.g., Playwright/Cypress for web surfaces)
- Success path completes end-to-end
- Failure path produces correct error UX
- Persona has appropriate permissions
- Multi-surface parity (if capability spans web + mobile + api)

**Test types:** E2e, integration, acceptance.

**Scenario role:** Scenarios in capabilities are **agent prompts** — structured acceptance criteria that AI agents use to generate both implementation and test code. The `tests` field closes the traceability loop.

---

## Test Coverage Chain

The 5 testable entities form a natural hierarchy that maps to the testing pyramid:

```
capability           → e2e / acceptance tests (scenario blocks)
  └─► behavior       → unit / integration tests (verify statements)
        ├─► invariant → property tests (verify property)
        ├─► event     → contract / integration tests (verify integration)
        └─► constraint → NFR tests (verify load/security)
```

- **behavior** is the workhorse — the highest volume of tests lives here
- **invariant** is the safety net — fewer tests, but they must never fail
- **event** tests the wiring — producer/consumer contracts
- **constraint** tests quality — measurable thresholds
- **capability** is the acceptance gate — full user scenarios

---

## Three-Layer Traceability (aligned with RES-15)

Each testable entity participates in the three-layer traceability model:

```
Layer 1: INTENT         verify / scenario        "What should be tested"
Layer 2: LINKAGE        tests field              "Where the test lives"
Layer 3: PROOF          specforge-report.json    "Did it pass"
```

### Per-entity traceability

| Entity | Layer 1 (Intent) | Layer 2 (Linkage) | Layer 3 (Proof) |
|--------|-----------------|-------------------|-----------------|
| behavior | `verify unit/integration/property/load` + optional `scenario` | `tests ["path/to/test.ts"]` | `specforge-report.json` via runner plugin |
| invariant | `verify property/unit` | `tests ["path/to/prop.ts"]` | `specforge-report.json` via runner plugin |
| event | `verify integration` | `tests ["path/to/event.test.ts"]` | `specforge-report.json` via runner plugin |
| constraint | `verify load/security` | `tests ["path/to/perf.k6.js"]` | `specforge-report.json` via runner plugin |
| capability | `verify e2e` + `scenario` blocks | `tests ["path/to/e2e.spec.ts"]` | `specforge-report.json` via runner plugin |

### Traceability matrix output

```bash
specforge trace --test-results specforge-report.json
```

```
Entity          | Intent              | Test File                       | Status
────────────────|─────────────────────|─────────────────────────────────|────────
create_user     | 3 verify (u/u/i)    | tests/users/create-user.test.ts | ✅ 3/3 PASS
unique_ids      | 2 verify (p/u)      | tests/invariants/unique.prop.ts | ✅ 2/2 PASS
create_user_ux  | 2 scenarios         | tests/e2e/create-user.spec.ts   | ✅ 2/2 PASS
user_created    | 2 verify (i/i)      | —                               | ⚠️ NO TEST
latency_p99     | 1 verify (load)     | tests/perf/latency.k6.js        | ❌ FAIL
```

---

## Declarative Entity Categories

The 11 declarative entities fall into three groups:

### Group 1: Composition / Grouping (3)

| Entity | What it groups | Why not testable |
|--------|---------------|-----------------|
| feature | Behaviors into a user story | Testing constituent behaviors covers it |
| deliverable | Capabilities into a shippable artifact | Bill of materials, not behavior |
| library | Features into a code package | Structural mapping, not executable |

These entities are **composition layers**. They don't add testable behavior — they organize testable things into higher-level units. The compiler validates their structural integrity (E001 dangling refs, E007 cycles, W009 orphans).

### Group 2: Structural Contracts (2)

| Entity | What it defines | Why not testable |
|--------|----------------|-----------------|
| type | Data shapes (struct/union/enum) | Passive schema — no logic, no side effects |
| port | Interface signatures (methods, params, return types) | Contract shape — implementations are tested, not the definition |

These entities define **what things look like**, not **what things do**. The code that agents produce from their graph context (TypeScript interfaces, Go interfaces) is validated by the type checker at compile time, not by test suites at runtime.

### Group 3: Documentation / Metadata (6)

| Entity | What it documents | Why not testable |
|--------|------------------|-----------------|
| spec | Project configuration | Configures the compiler, not the target software |
| ref | External system links | Metadata anchor — zero runtime presence |
| roadmap | Planning phases and timelines | Scheduling metadata |
| decision | Architectural rationale (ADRs) | Historical "why" documentation |
| failure_mode | FMEA risk analysis | Analytical documentation — informs testing, but is not itself testable |
| glossary | Term definitions | Pure vocabulary reference |

These entities exist for **human communication and tooling metadata**. The compiler validates their structure, but they have no runtime representation in the target software.

---

## Implications for SpecForge

### Test Coverage

Only the 5 testable entities require test coverage in the target software. The three-layer traceability model (RES-15) applies: Intent (verify/scenario) → Linkage (tests field) → Proof (specforge-report.json). AI agents or developers implement the tests; SpecForge validates the chain.

| Entity | verify intent | scenario intent |
|--------|--------------|-----------------|
| behavior | One test per `verify` statement | Structured test with Given/When/Then steps (if scenario present) |
| invariant | Property-based test asserting the `guarantee` | — |
| event | Contract test checking payload shape + delivery | — |
| constraint | NFR test with threshold assertions | — |
| capability | E2E test per `verify e2e` | Playwright/Cypress test following scenario steps |

### Coverage Reporting (`specforge trace`)

Coverage computed across four levels (from RES-15):

| Level | Formula | Signal |
|-------|---------|--------|
| **Declared** | testable entities with verify/scenario / total testable | Has test intent |
| **Linked** | testable entities with `tests` field / total testable | Connected to real tests |
| **Executed** | entities with results in `specforge-report.json` / total testable | Tests have been run |
| **Passing** | entities with all tests passing / total testable | Full traceability |

Only the 5 testable entity kinds count. Declarative entities are excluded — they are validated by the compiler, not by tests.

### Agent Workflow

AI agents are SpecForge's primary consumer. The testability classification directly impacts agent workflow:

**For testable entities** — agent must:
1. Read spec (via `specforge show <entity> --depth=2`)
2. Write implementation code (guided by `contract` field)
3. Write test files (guided by `verify` statements and `scenario` steps)
4. Fill `tests` field (linking spec to test files)
5. Run tests and verify chain (via `specforge trace`)

**For declarative entities** — agent must:
1. Read spec
2. Write implementation code (if applicable — types generate interfaces, ports generate stubs)
3. No test writing needed — compiler validates structural integrity

### Validation Warnings

| Code | Condition | Message |
|------|-----------|---------|
| **W004** | Testable entity has no `verify` or `scenario` | `"{kind} '{id}' has no verify statements or scenarios"` |
| **W018** | Testable entity has verify/scenario but no `tests` field | `"'{id}' has test declarations but no linked test files"` |
| **E016** | `tests` field references a file that doesn't exist | `"test file '{path}' not found"` |

### Extension Testability API

```rust
// NOTE: In zero-entity core (RES-26), these hardcoded matches are replaced
// by dynamic KindRegistry lookups. Extension manifests declare testability.
impl EntityKind {
    pub fn is_testable(&self) -> bool {
        matches!(self,
            Self::Behavior | Self::Invariant | Self::Event
            | Self::Constraint | Self::Capability
        )
    }

    pub fn supports_scenario(&self) -> bool {
        matches!(self, Self::Capability)
    }
}
```

Extension authors declare testability in the extension manifest:
```toml
[[entity_kinds]]
keyword = "validation_rule"
testable = true
supports_verify = true
supports_scenario = false
```

---

## Decision

**The 5 testable entities are: behavior, invariant, event, constraint, capability.**

These are the concepts that require automated test coverage in the target software. They support `verify` statements (behavior, invariant, event, constraint), `scenario` blocks (capability only), and `tests` field linkage. They participate in the three-layer traceability model (intent → linkage → proof) and count toward spec coverage.

All other entities are validated through domain-appropriate methods (compiler structural checks, governance audits, product reviews, etc.) and do not need automated test suites in target software.

This classification informs coverage reporting, traceability chains, agent workflow, extension API, and developer documentation. See RES-15 for the full test declaration and traceability architecture.
