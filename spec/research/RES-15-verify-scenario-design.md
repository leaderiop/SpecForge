---
id: RES-15
kind: research
title: "Test Declaration & Traceability — verify, scenario, tests, and execution proof"
status: active
date: 2026-03-02
depends_on: RES-14
---

# RES-15: Test Declaration & Traceability

## Problem Statement

RES-14 identified 5 testable entities (behavior, invariant, event, constraint, capability) that require test coverage in the target software. SpecForge's core mission is **traceability** — ensuring AI agents implement everything correctly and proving it through a compiler-verified chain from specification to passing tests.

Three design questions:

1. How should tests be declared in `.spec` files?
2. How does specforge link declarations to real, executable test files?
3. How does specforge prove tests actually pass?

This analysis was produced by 20 specialized expert agents across two rounds — the first examining syntax design, the second examining traceability architecture, agent workflows, test execution integration, and the role of scenarios as agent prompts.

---

## Decision Summary

| Decision | Choice |
|----------|--------|
| **Test intent** | `verify` (one-liner, 5 kinds) + `scenario` (structured Given/When/Then) |
| **Test linkage** | `tests` field — the PRIMARY traceability mechanism |
| **Test proof** | `specforge-report.json` consumed from test runner plugins |
| **verify kinds** | `unit`, `integration`, `property`, `load`, `e2e` |
| **scenario scope** | Allowed in **capability** and **behavior** only |
| **scenario syntax** | `given`, `when`, `then` only. No `and`/`but`/examples/tags/background. |
| **scenario role** | Structured acceptance criteria — agent prompt, not test documentation |
| **plugin testability** | `EntityKind::is_testable()` + plugin manifest `testable: bool` |

---

## The Three-Layer Traceability Model

Every test declaration in SpecForge flows through three layers:

```
Layer 1: INTENT         verify / scenario        "What should be tested"
Layer 2: LINKAGE        tests field              "Where the test lives"
Layer 3: PROOF          specforge-report.json    "Did it pass"
```

### Layer 1: Intent — `verify` and `scenario`

Declares WHAT should be tested. Lives inside `.spec` files. Two levels of detail:

**`verify`** — one-liner for any test kind:
```spec
verify unit        "valid input creates user and returns ID"
verify integration "created user is queryable by ID"
verify property    "email uniqueness under concurrent creation"
verify load        "1000 concurrent requests under 200ms p99"
verify e2e         "admin creates user and sees confirmation"
```

**`scenario`** — structured Given/When/Then for e2e flows:
```spec
scenario "admin creates user successfully" {
  given "admin is on user management page"
  when  "admin fills name, email, role and clicks Create"
  then  "success toast appears"
  then  "new user appears in user list"
}
```

Intent declarations alone are NOT traceability. They are acceptance criteria — instructions for the agent or developer implementing the tests.

### Layer 2: Linkage — `tests` field

The `tests` field is the **primary traceability mechanism**. It links a spec entity to actual executable test files:

```spec
behavior create_user "Create User" {
  verify unit "valid input creates user"
  verify unit "duplicate email returns error"
  tests ["tests/users/create-user.test.ts"]
}

capability create_user_ux "Create a New User" {
  scenario "admin creates user" { ... }
  tests ["tests/e2e/create-user.spec.ts"]
}
```

Without a `tests` field, a testable entity is **unlinked** — the compiler warns that intent exists but no implementation is connected.

The `tests` field supports any test framework:
```spec
tests [
  "tests/user_test.go::TestCreateUser",
  "tests/user.test.ts:45",
  "tests/test_user.py::test_create_user",
  "tests/e2e/create-user.feature",
  "tests/perf/latency.k6.js"
]
```

### Layer 3: Proof — `specforge-report.json`

SpecForge does NOT run tests. It consumes results from test runner plugins:

```bash
# Test runner emits results via specforge plugin
npm test -- --reporter=@specforge/vitest

# SpecForge validates the chain
specforge trace --test-results specforge-report.json
```

Report format:
```json
{
  "specforge": "1.0",
  "runner": "@specforge/vitest",
  "timestamp": "2026-03-02T14:30:00Z",
  "results": {
    "create_user": {
      "file": "tests/users/create-user.test.ts",
      "tests": [
        { "name": "valid input creates user", "status": "pass", "duration_ms": 45 },
        { "name": "duplicate email returns error", "status": "pass", "duration_ms": 32 }
      ]
    }
  }
}
```

Test runner plugins: `@specforge/vitest`, `@specforge/pytest`, `@specforge/playwright`, `@specforge/go`, `@specforge/k6`.

---

## Scenario: Structured Acceptance Criteria for Agents

### The mental model

Scenarios are NOT test documentation. They are **agent prompts** — structured acceptance criteria that AI agents use to generate implementation and test code.

```
Human writes scenario (acceptance criteria)
         ↓
Agent reads scenario (structured prompt)
         ↓
Agent generates implementation + test file
         ↓
Agent fills `tests` field (closes the loop)
         ↓
specforge trace (validates the chain)
```

This is not duplication — it's the input/output relationship:
- **Input**: Scenario block in `.spec` file (what the human wants)
- **Output**: Test file generated by agent (executable proof)
- **Link**: `tests` field (agent confirms it's done)
- **Proof**: `specforge trace` (compiler verifies everything)

### Why scenarios help agents

Compare what an agent gets from each:

**verify e2e only:**
```spec
verify e2e "admin creates user successfully"
```
Agent must INFER: What page? What fields? What success looks like? How many steps?

**scenario:**
```spec
scenario "admin creates user successfully" {
  given "admin is on user management page"
  when  "admin fills name, email, role and clicks Create"
  then  "success toast appears"
  then  "new user appears in user list"
}
```
Agent gets: Setup phase (navigate to page), action (fill 3 fields + click), two assertions (toast + list). Unambiguous.

### Where each syntax is allowed

| Entity | `verify` | `scenario` | Rationale |
|--------|----------|-----------|-----------|
| **behavior** | Yes | Yes | Both available. Developer picks the right level of detail. |
| **invariant** | Yes | No | Timeless guarantees, not sequential flows. `verify property` fits. |
| **event** | Yes | No | Data contracts (producer/consumer). Not step-based flows. |
| **constraint** | Yes | No | Measurable thresholds. Load/security tests don't fit Given/When/Then. |
| **capability** | Yes | Yes | `verify e2e` for quick declaration, `scenario` when steps matter. |

```
                    verify    scenario    tests
                    ──────    ────────    ─────
  behavior           ✅         ✅         ✅
  invariant          ✅         ❌         ✅
  event              ✅         ❌         ✅
  constraint         ✅         ❌         ✅
  capability         ✅         ✅         ✅
```

### Scenario syntax

**Included:**

| Feature | Notes |
|---------|-------|
| `given` / `when` / `then` keywords | The three core step types |
| Multiple steps of each type | Multiple givens, whens, thens allowed |
| Empty scenario validation | E004 error |
| Missing when/then validation | W015/W016 warnings |

**Excluded:**

| Feature | Rationale |
|---------|-----------|
| `and` / `but` keywords | Syntactic sugar, adds ambiguity. Repeat `given`/`when`/`then` instead. |
| Scenario outlines + examples tables | Multiple scenario blocks achieve the same goal. |
| Tags/labels (`@smoke`) | Test execution concern, not specification. |
| Background/setup blocks | Action-at-a-distance. Use `contract` field for shared context. |
| Step parameters (`{email}`) | Requires scenario outlines. |
| Nested scenarios | Flat is better. Behavior block is the grouping mechanism. |

---

## Agent Workflow

### Step-by-step: Agent implements a capability

1. **Agent receives task**: "implement capability `create_user_ux`"

2. **Agent loads spec** (selective context — only what's needed):
   ```bash
   specforge show create_user_ux --depth=2
   ```
   Returns: capability spec + referenced features + behaviors + invariants + types.

3. **Agent reads acceptance criteria**: scenarios provide structured instructions.

4. **Agent writes implementation code**: guided by behavior contracts.

5. **Agent writes test files**: guided by `verify` statements and `scenario` steps.
   - Unit tests for behaviors (from `verify unit "..."`)
   - E2e tests for capabilities (from `scenario` blocks)

6. **Agent fills `tests` field**: links spec to generated test files.
   ```spec
   capability create_user_ux {
     scenario "admin creates user" { ... }
     tests ["tests/e2e/create-user.spec.ts"]  // ← agent adds this
   }
   ```

7. **Agent runs tests**: executes via test runner with specforge plugin.

8. **Agent verifies chain**:
   ```bash
   specforge trace create_user_ux
   ```
   Output: `✅ create_user_ux: 2 scenarios, all linked, all pass`

### Test file annotations

Test files should reference back to the spec entity for bidirectional traceability:

```typescript
// @specforge-capability create_user_ux
// @specforge-file capabilities/user-management.spec

test.describe('create_user_ux', () => {
  test('admin creates user successfully', async ({ page }) => {
    // @specforge-scenario "admin creates user successfully"

    // given "admin is on user management page"
    await page.goto('/admin/users');

    // when "admin fills name, email, role and clicks Create"
    await page.fill('[name="name"]', 'Alice');
    await page.fill('[name="email"]', 'alice@example.com');
    await page.selectOption('[name="role"]', 'admin');
    await page.click('button:has-text("Create")');

    // then "success toast appears"
    await expect(page.locator('.toast')).toContainText('User created');

    // then "new user appears in user list"
    await expect(page.locator('table')).toContainText('Alice');
  });
});
```

Annotations enable:
- `specforge trace` → bidirectional validation (spec references test, test references spec)
- LSP "Go to Definition" → jump from test file to spec scenario
- Code generation → preserve hand-written test bodies during regeneration

---

## Traceability Matrix

`specforge trace` generates a live coverage matrix by merging all three layers:

```
Entity          | Intent              | Test File                       | Status
────────────────|─────────────────────|─────────────────────────────────|────────
create_user     | 3 verify (u/u/i)    | tests/users/create-user.test.ts | ✅ 3/3 PASS
unique_ids      | 2 verify (p/u)      | tests/invariants/unique.prop.ts | ✅ 2/2 PASS
create_user_ux  | 2 scenarios         | tests/e2e/create-user.spec.ts   | ✅ 2/2 PASS
user_created    | 2 verify (i/i)      | —                               | ⚠️ NO TEST
latency_p99     | 1 verify (load)     | tests/perf/latency.k6.js        | ❌ FAIL
delete_user     | —                   | —                               | 🔇 NO INTENT
```

Data sources:
- **Entity + Intent** → from `.spec` files (`verify`/`scenario` declarations)
- **Test File** → from `tests` field in `.spec` files
- **Status** → from `specforge-report.json` (emitted by test runner plugins)

### Coverage levels

| Level | Condition | Signal |
|-------|-----------|--------|
| **Declared** | Has `verify`/`scenario` statements | Entity has test intent |
| **Linked** | Has `tests` field with existing files | Entity connected to real tests |
| **Executed** | Test result exists in report | Tests have been run |
| **Passing** | All tests show `status: "pass"` | Full traceability achieved |

### Coverage gating in CI

```bash
specforge trace --test-results specforge-report.json --min-coverage 95
# Exit code 1 if coverage < 95%
```

---

## Code Generation

### verify generates test stubs

```typescript
// From: verify unit "valid input creates user and returns ID"
// @specforge-behavior create_user
describe('create_user', () => {
  it('valid input creates user and returns ID', () => {
    // @specforge-verify unit create_user
    // TODO: implement test
  });
});
```

### scenario generates structured e2e test code

Default: Playwright. Cucumber `.feature` output available via `@specforge/gen-cucumber` plugin.

```typescript
// From: scenario "admin creates user successfully" { given/when/then }
// @specforge-capability create_user_ux
test.describe('create_user_ux', () => {
  test('admin creates user successfully', async ({ page }) => {
    // @specforge-scenario create_user_ux "admin creates user successfully"

    // given "admin is on user management page"
    // TODO: implement setup

    // when "admin fills name, email, role and clicks Create"
    // TODO: implement action

    // then "success toast appears"
    // TODO: implement assertion

    // then "new user appears in user list"
    // TODO: implement assertion
  });
});
```

### Cucumber `.feature` generation (opt-in plugin)

```bash
specforge gen cucumber --out features/
```

Generates `features/create-user.feature` FROM inline scenarios:
```gherkin
# @specforge-capability create_user_ux
Feature: Create a New User

  Scenario: admin creates user successfully
    Given admin is on user management page
    When admin fills name, email, role and clicks Create
    Then success toast appears
    Then new user appears in user list
```

The `.spec` file is the source of truth. Generated `.feature` files are build artifacts.

---

## Plugin Testability API

### Built-in entities

```rust
impl EntityKind {
    pub fn is_testable(&self) -> bool {
        matches!(
            self,
            Self::Behavior | Self::Invariant | Self::Event
            | Self::Constraint | Self::Capability
        )
    }

    pub fn supports_scenario(&self) -> bool {
        matches!(self, Self::Behavior | Self::Capability)
    }
}
```

### Plugin manifest

Plugin authors declare testability in `plugin.toml`:

```toml
[[entities]]
keyword = "validation_rule"
testable = true
supports_verify = true
supports_scenario = false
```

`PluginRegistry` merges built-in + custom entity metadata. Validators and generators query `registry.is_testable()` instead of hardcoding entity kinds.

### Grammar for plugin entities

Tree-sitter grammars compile ahead of time. Solution: a generic fallback block type for custom plugin entities that accepts verify/scenario. Post-parse validation checks if the entity kind actually supports them.

---

## Validation Rules

### Extended existing rule

| Code | Change | Message |
|------|--------|---------|
| **W004** | Extended to all 5 testable entities | `"{kind} '{id}' has no verify statements or scenarios"` |

### New rules for scenarios

| Code | Severity | Condition | Message |
|------|----------|-----------|---------|
| **E004** | Error | Empty scenario block (no steps) | `"scenario '{title}' has no steps"` |
| **E015** | Error | Duplicate scenario title within entity | `"duplicate scenario title '{title}' in {kind} '{id}'"` |
| **W015** | Warning | Scenario without `when` step | `"scenario '{title}' has no 'when' step"` |
| **W016** | Warning | Scenario without `then` step | `"scenario '{title}' has no 'then' step (no assertion)"` |

### New rules for traceability

| Code | Severity | Condition | Message |
|------|----------|-----------|---------|
| **E016** | Error | `tests` field references a file that doesn't exist | `"test file '{path}' not found"` |
| **W017** | Warning | Plugin entity marked testable but grammar has no verify/scenario | `"plugin entity '{kind}' marked testable but no grammar support"` |
| **W018** | Warning | Testable entity has verify/scenario but no `tests` field | `"'{id}' has test declarations but no linked test files"` |
| **I002** | Info | Testable entity uses only one verification mechanism | `"{kind} '{id}' uses scenarios but no verify statements"` |
| **I006** | Info | Plugin entity has verify but not marked testable | `"plugin entity '{kind}' supports verify but not marked testable"` |

---

## Grammar Additions

### Scenario block (~15 lines)

```javascript
scenario_block: ($) =>
  seq(
    "scenario",
    field("title", $.string),
    "{",
    repeat1($._scenario_step),
    "}",
  ),

_scenario_step: ($) =>
  choice($.given_step, $.when_step, $.then_step),

given_step: ($) => seq("given", field("description", $.string)),
when_step:  ($) => seq("when",  field("description", $.string)),
then_step:  ($) => seq("then",  field("description", $.string)),
```

### New AST types

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    pub title: String,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioStep {
    pub kind: ScenarioStepKind,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScenarioStepKind {
    Given,
    When,
    Then,
}
```

New `FieldValue` variant: `ScenarioList(Vec<Scenario>)`.

---

## Competitive Positioning

| Dimension | SpecForge | Cucumber | Gauge | SysML | OpenAPI |
|-----------|-----------|----------|-------|-------|---------|
| Primary consumer | AI agents | Human QA/dev | Human QA/dev | Systems engineers | API consumers |
| Test syntax | Dual (verify + scenario) | Single (Gherkin) | Single (markdown) | N/A | N/A |
| Traceability | Compiler-checked chain | Manual tags | Manual tags | Separate artifacts | No test concept |
| Coverage proof | `specforge-report.json` | Cucumber reports | Gauge reports | External tools | N/A |
| Domain scope | Full system (arch + behavior + governance) | Behavior only | Behavior only | Very broad | API only |
| Agent workflow | First-class (scenario as agent prompt) | Not designed for agents | Not designed for agents | Not designed for agents | Partial (codegen) |

**SpecForge's differentiator:** The first specification language designed for AI-assisted implementation. Scenarios are structured acceptance criteria that agents consume to generate both implementation and test code. The `tests` field closes the loop. `specforge trace` proves the chain.

---

## Full Example

```spec
// Capability — scenarios as acceptance criteria + tests for linkage
capability create_user_ux "Create a New User" {
  persona  admin
  surface  [web]
  features [user_management]

  verify e2e "admin creates user and sees confirmation"

  scenario "admin creates user successfully" {
    given "admin is on user management page"
    when  "admin fills name, email, role and clicks Create"
    then  "success toast appears"
    then  "new user appears in user list"
  }

  scenario "duplicate email rejection" {
    given "user with email alice@test.com already exists"
    when  "admin creates another user with email alice@test.com"
    then  "inline error: Email already in use"
  }

  tests ["tests/e2e/create-user.spec.ts"]
}

// Behavior — verify for unit/integration, scenario optional for API flows
behavior create_user "Create User" {
  invariants [unique_ids]
  contract """The system MUST create a user record with unique email."""

  verify unit        "valid input creates user and returns ID"
  verify unit        "duplicate email returns DuplicateEmailError"
  verify integration "created user is queryable by ID"
  verify property    "email uniqueness under concurrent creation"

  scenario "full creation flow via API" {
    given "no user with email bob@test.com exists"
    when  "POST /users with valid payload"
    then  "201 response with user ID"
    then  "GET /users/:id returns the created user"
  }

  tests ["tests/users/create-user.test.ts"]
}

// Invariant — verify only + tests linkage
invariant unique_ids "Unique Entity IDs" {
  guarantee """No two entities SHALL share the same qualified ID."""
  enforced_by [create_user]

  verify property "10000 random entities maintain ID uniqueness"
  verify unit     "duplicate ID insertion returns E002"

  tests ["tests/invariants/unique-ids.prop.ts"]
}

// Event — verify only + tests linkage
event user_created "User Created" {
  trigger  create_user
  payload  UserCreatedPayload
  channel  "user-events"
  consumers [send_welcome_email, update_search_index]

  verify integration "create_user emits event with correct payload"
  verify integration "consumers receive and process event"

  tests ["tests/events/user-created.test.ts"]
}

// Constraint — verify only + tests linkage
constraint latency_p99 "Incremental Compilation Latency" {
  category performance
  priority must

  verify load "benchmark incremental recompile with 500 files, assert < 100ms"

  tests ["tests/perf/latency.k6.js"]
}
```
