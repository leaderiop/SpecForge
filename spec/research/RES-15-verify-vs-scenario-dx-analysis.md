---
id: RES-15
kind: research
title: "Verify vs Scenario Dual Syntax — Developer Experience Analysis"
status: active
date: 2026-03-02
depends_on: [RES-14, RES-11a]
---

# RES-15: Verify vs Scenario Dual Syntax — DX Analysis

## Problem Statement

SpecForge currently uses `verify unit|integration|property|load|e2e "description"` for all test declarations. RES-14 identified 5 testable entities (behavior, invariant, event, constraint, capability), where `capability` represents full user flows. The proposal: split test syntax into two constructs — `verify` for isolated tests and `scenario` blocks for user flows with Given/When/Then structure.

**This analysis evaluates the authoring ergonomics, cognitive load, and migration path for this dual syntax.**

---

## Proposed Syntax

### Current (Status Quo)
```spec
behavior "Create User" {
  verify unit        "valid input creates user and returns ID"
  verify integration "created user is queryable by ID"
  verify property    "email uniqueness under concurrent creation"
  verify load        "1000 concurrent requests under 200ms p99"
  verify e2e         "admin creates user through web UI"
}
```

### Proposed (Dual Syntax)
```spec
behavior "Create User" {
  verify unit        "valid input creates user and returns ID"
  verify integration "created user is queryable by ID"
  verify property    "email uniqueness under concurrent creation"
  verify load        "1000 concurrent requests under 200ms p99"
}

capability "Admin Creates User" {
  persona  admin
  surface  [web]
  features [user_management]

  scenario "successful creation" {
    given "admin is on user management page"
    when  "admin fills name, email, role and clicks Create"
    then  "success toast appears"
    then  "new user appears in user list"
  }
}
```

---

## 1. Cognitive Load Analysis

### Decision Rule Clarity

**The proposed rule:** "Use `verify` for isolated tests (unit/integration/property/load). Use `scenario` for user flows (e2e tests in capabilities)."

**Strengths:**
- **Entity-driven clarity**: `scenario` only lives in `capability` blocks. Capabilities already have persona + surface + flow, so the scenario syntax is contextually natural.
- **Testing pyramid alignment**: `verify` maps to the bottom/middle (unit/integration/property/load), `scenario` maps to the top (e2e acceptance).
- **Natural mental model**: Developers already distinguish between "testing a function" vs "testing a user story." This syntax mirrors that distinction.

**Weaknesses:**
- **New keyword to learn**: Developers must recognize `scenario` as distinct from `verify`.
- **Blurred line for integration tests**: Integration tests can test cross-service flows. When does an integration test become a scenario? Answer: if it requires user simulation (clicks, form fills), it's a scenario. If it's service-to-service API calls, it's `verify integration`.

**Verdict:** The rule is **clear enough for practitioners**. The entity boundary (behavior/invariant/event/constraint vs capability) creates a natural split. Confusion risk is low.

---

## 2. Verbosity Tradeoff

### Line Count Comparison

| Construct | Lines | Use Case |
|-----------|-------|----------|
| `verify e2e "admin creates user successfully"` | 1 | Quick assertion of e2e coverage |
| `scenario` block (minimal) | 5 | Given (1) + When (1) + Then (2) + braces (2) |
| `scenario` block (realistic) | 7-10 | Multiple Given/Then steps, descriptive names |

**When verbosity is justified:**
- **User acceptance criteria**: Scenarios in capability blocks serve as executable acceptance tests. The Given/When/Then structure documents what the user sees and does.
- **Cross-functional communication**: PMs, QAs, and designers can read scenarios without understanding implementation. `verify e2e "..."` is opaque.
- **Multi-step flows**: 3+ step flows (login → navigate → action → verify) are clearer as structured blocks than compressed strings.

**When verbosity is NOT justified:**
- **Simple "does this work e2e" smoke tests**: If the scenario has 1 given, 1 when, 1 then, a `verify e2e` is cleaner.
- **Behavior-level e2e**: Behaviors test single operations. Their e2e tests are often simple integration checks ("create user via API"). `verify integration` suffices.

**Verdict:** Verbosity is **justified for capabilities, overkill for behaviors**. This aligns with the proposal — scenarios live in capabilities only.

---

## 3. LSP Autocomplete Strategy

### Autocomplete Triggers

| Context | Trigger | Suggested Completions |
|---------|---------|---------------------|
| Inside `behavior`, `invariant`, `event`, `constraint` | Type `ver` | `verify unit`, `verify integration`, `verify property`, `verify load` |
| Inside `capability` | Type `sce` | `scenario "name" {` with Given/When/Then snippet |
| After `scenario` keyword | Type `"` | Auto-insert closing quote, braces, and template |

### Snippet Template

When autocompleting `scenario`, insert:
```spec
scenario "${1:scenario name}" {
  given "${2:precondition}"
  when  "${3:user action}"
  then  "${4:expected outcome}"
}
```

### Diagnostics

The LSP should:
- **W004 extension**: Warn on untested testable entities. For capabilities without scenarios, emit: `W012: capability lacks scenario blocks (e2e coverage missing)`
- **Inline suggestions**: When a capability has no scenarios, show a lightbulb code action: "Add scenario block"
- **Semantic token highlighting**: Color `given/when/then` keywords distinctly from `verify`

**Verdict:** LSP support is **straightforward**. The entity-driven rule (scenario only in capability) simplifies autocomplete logic.

---

## 4. Common Mistakes

### Mistake 1: Using `verify e2e` in capabilities

**User writes:**
```spec
capability "Admin Creates User" {
  verify e2e "admin creates user successfully"  // ← wrong syntax
}
```

**Compiler response:**
```
error[E011]: invalid syntax in capability block
  --> capabilities.spec:42:3
   |
42 |   verify e2e "admin creates user successfully"
   |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |   capabilities use 'scenario' blocks, not 'verify' statements
   |
   = help: replace with: scenario "admin creates user successfully" { ... }
```

**Fix difficulty:** Easy. Clear diagnostic with suggested fix.

---

### Mistake 2: Writing scenarios for unit tests

**User writes:**
```spec
behavior "Create User" {
  scenario "create user with valid email" {  // ← wrong entity
    given "valid user data"
    when  "CreateUser is called"
    then  "user ID is returned"
  }
}
```

**Compiler response:**
```
error[E011]: invalid syntax in behavior block
  --> behaviors.spec:15:3
   |
15 |   scenario "create user with valid email" {
   |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |   behaviors use 'verify' statements, not 'scenario' blocks
   |
   = help: replace with: verify unit "create user with valid email"
```

**Fix difficulty:** Easy. Clear diagnostic with suggested fix.

---

### Mistake 3: Empty scenarios

**User writes:**
```spec
scenario "successful creation" {
  // empty
}
```

**Compiler response:**
```
error[E013]: empty scenario block
  --> capabilities.spec:48:1
   |
48 | scenario "successful creation" {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   | scenarios must have at least one 'when' and one 'then' clause
   |
   = note: 'given' is optional, but 'when' and 'then' are required
```

**Fix difficulty:** Easy. Clear validation rule.

---

### Mistake 4: Scenarios without `when` or `then`

**User writes:**
```spec
scenario "successful creation" {
  given "admin is on user management page"
  given "user does not exist"
  // missing when/then
}
```

**Compiler response:**
```
error[E014]: incomplete scenario
  --> capabilities.spec:50:1
   |
50 | scenario "successful creation" {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   | scenario missing required 'when' clause
   |
   = note: scenarios must describe a user action (when) and an outcome (then)
```

**Verdict:** Common mistakes are **easily caught by the compiler** with clear error messages. Migration from `verify e2e` to `scenario` is mechanically checkable.

---

## 5. Teaching the Rule

### Documentation One-Liner

> **Use `verify` to test code (unit, integration, property, load). Use `scenario` to test user flows (e2e acceptance).**

### Extended Explanation (2 sentences)

> `verify` statements declare how to test isolated behaviors, invariants, events, and constraints — they map to unit tests, integration tests, property-based tests, and load tests. `scenario` blocks in capabilities declare how to test full user flows using Given/When/Then syntax — they map to e2e acceptance tests in Playwright, Cypress, or Selenium.

### Decision Tree (for docs)

```
Are you testing a single operation (function, API call, event emission)?
  ├─ YES → Use `verify` (in behavior/invariant/event/constraint)
  └─ NO → Are you testing a multi-step user flow?
            ├─ YES → Use `scenario` (in capability)
            └─ NO → Reconsider what you're testing
```

**Verdict:** The rule is **teachable in 1-2 sentences**. The entity-driven boundary makes it intuitive.

---

## 6. Scenario Syntax Details

### Minimal Viable Syntax (Recommendation)

Support **only** `given`, `when`, and `then`. No `and`, no `but`, no `examples` tables.

**Rationale:**
- **Simplicity**: 3 keywords are easy to parse, easy to teach, easy to remember.
- **Gherkin compatibility**: Most Gherkin features (and, but, tables) exist for Cucumber's step-definition reuse. SpecForge scenarios are **descriptive, not executable** — they document what the test should do, not how to do it.
- **Avoid scope creep**: `examples` tables add parametric test logic. That belongs in the test framework (Playwright's test.describe.parallel, Vitest's test.each), not the spec DSL.
- **and/but readability**: `and` can chain multiple clauses, but this obscures test structure. Force explicit `given`/`when`/`then` for clarity.

### Allowed Repetition

Multiple `given`, `when`, and `then` clauses are allowed:

```spec
scenario "successful creation" {
  given "admin is logged in"
  given "admin is on user management page"
  when  "admin fills name, email, role"
  when  "admin clicks Create"
  then  "success toast appears"
  then  "new user appears in user list"
}
```

This is **clearer than** `given "admin is logged in and on user management page"` because each clause is a distinct precondition or assertion.

### Optional `given`

`given` clauses are optional (some tests have no preconditions):

```spec
scenario "submit without login" {
  when "user navigates to protected page"
  then "login redirect occurs"
}
```

### Required `when` and `then`

Every scenario MUST have at least one `when` (the user action) and one `then` (the expected outcome). This is validated by the compiler (E014).

---

## 7. Migration Path

### Current Usage

SpecForge has **2 occurrences** of the string `e2e` in spec files (as of 2026-03-02):
1. Glossary definition of `verify` statement
2. One or two example usages (grep count: 2)

**Implication:** Migration surface is **tiny**. This change can be introduced in 1.0 without significant churn.

### Migration Strategy

**Step 1:** Add `scenario` syntax support in grammar and compiler.

**Step 2:** Emit a **deprecation warning** when `verify e2e` is used in a capability:

```
warning[W010]: deprecated syntax
  --> capabilities.spec:42:3
   |
42 |   verify e2e "admin creates user successfully"
   |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |   'verify e2e' in capabilities is deprecated
   |
   = help: use 'scenario' blocks instead
   = note: 'verify e2e' will be removed in version 2.0
```

**Step 3:** Allow `verify e2e` in behaviors/invariants/events/constraints (for backward compatibility until 2.0).

**Step 4:** In 2.0, make `scenario` the only syntax for e2e tests in capabilities.

### Automated Migration

`specforge migrate --from=1.0 --to=2.0` can mechanically transform:

```spec
capability "Admin Creates User" {
  verify e2e "admin creates user successfully"
}
```

Into:

```spec
capability "Admin Creates User" {
  scenario "admin creates user successfully" {
    // TODO: expand into given/when/then steps
  }
}
```

The migration tool cannot infer the Given/When/Then structure from the string, so it leaves a TODO comment. The developer fills in the details.

**Verdict:** Migration is **smooth** because current usage is minimal. Deprecation path is clear.

---

## 8. Comparison with Gherkin

| Feature | Gherkin (Cucumber) | SpecForge `scenario` (Proposed) |
|---------|-------------------|--------------------------------|
| **Purpose** | Executable BDD tests | Descriptive acceptance criteria |
| **Step definitions** | Required (step implementation files) | Not required (test framework handles it) |
| **Keywords** | given, when, then, and, but, background, scenario outline, examples | given, when, then only |
| **Data tables** | Yes (for parametric tests) | No (use test framework's parametric features) |
| **Reusability** | Steps are reused across scenarios | Scenarios are self-contained descriptions |
| **Nesting** | Features > Scenarios > Steps | Capabilities > Scenarios > Clauses |

**Key difference:** Gherkin scenarios are **executable** (Cucumber runs step definitions). SpecForge scenarios are **descriptive** (they document what the test should do; the test framework like Playwright executes it).

**Advantage of SpecForge approach:** No step definition boilerplate. Scenarios live alongside capabilities, not in a separate `.feature` file.

---

## 9. Alternative Considered: Unified Syntax

### Option A: Keep `verify e2e` only

**Pros:**
- Simplicity: one syntax for all test types
- No new keyword to learn

**Cons:**
- **Loss of structure**: e2e tests are multi-step flows. Compressing them into a single string loses the Given/When/Then structure that makes them readable.
- **Poor cross-functional communication**: PMs and QAs cannot easily read `verify e2e "admin creates user successfully"` and understand the steps.
- **No validation**: The compiler cannot check that a scenario has a user action and an outcome.

**Verdict:** Rejected. Capabilities deserve structured acceptance criteria.

---

### Option B: Keep `scenario` only

**Pros:**
- Simplicity: one syntax for all test types
- Consistency with BDD tools

**Cons:**
- **Verbosity for simple tests**: `verify unit "valid input creates user"` is cleaner than a Given/When/Then block for a 1-line test.
- **Mismatch with test types**: Unit tests don't have "given/when/then" structure — they have "arrange/act/assert" or "setup/exercise/verify". Forcing Given/When/Then on unit tests is awkward.

**Verdict:** Rejected. Behaviors deserve terse test declarations.

---

## 10. Recommendation: Minimal Viable Scenario Syntax

### Approved Syntax

```spec
scenario "scenario name" {
  given "precondition"  // optional, can have 0+ clauses
  when  "user action"   // required, can have 1+ clauses
  then  "expected outcome" // required, can have 1+ clauses
}
```

### Constraints

1. **Only 3 keywords**: `given`, `when`, `then`. No `and`, `but`, `examples`, `background`.
2. **Only in capabilities**: `scenario` blocks are invalid in behavior/invariant/event/constraint blocks.
3. **Required clauses**: At least one `when` and one `then`. `given` is optional.
4. **Free-form strings**: Clause contents are arbitrary strings. No step definition pattern matching.

### Validation Rules

| Code | Severity | Message |
|------|----------|---------|
| E011 | Error | `scenario` block in non-capability entity (behaviors use `verify` statements) |
| E013 | Error | Empty scenario block |
| E014 | Error | Scenario missing required `when` clause |
| E014 | Error | Scenario missing required `then` clause |
| W012 | Warning | Capability lacks scenario blocks (e2e coverage missing) |

---

## 11. Code Generation Implications

### Test Scaffold Generation

When `specforge gen typescript` runs on a capability with scenarios:

**Input:**
```spec
capability UX-001 "Admin Creates User" {
  persona  admin
  surface  [web]

  scenario "successful creation" {
    given "admin is on user management page"
    when  "admin fills name, email, role and clicks Create"
    then  "success toast appears"
    then  "new user appears in user list"
  }
}
```

**Generated output (Playwright example):**
```typescript
// tests/capabilities/UX-001-admin-creates-user.spec.ts
import { test, expect } from '@playwright/test';

test.describe('UX-001: Admin Creates User', () => {
  test('successful creation', async ({ page }) => {
    // Given: admin is on user management page
    // TODO: navigate to user management page

    // When: admin fills name, email, role and clicks Create
    // TODO: fill form and click create

    // Then: success toast appears
    // TODO: assert toast is visible

    // Then: new user appears in user list
    // TODO: assert user is in list
  });
});
```

The generator emits **TODO comments** for each clause. The developer fills in the Playwright commands.

### Alternative: Annotated Scenarios

For advanced users, allow optional test code annotations:

```spec
scenario "successful creation" {
  given "admin is on user management page"
    test.ts """await page.goto('/users')"""

  when "admin fills name, email, role and clicks Create"
    test.ts """
      await page.fill('[name=name]', 'Jane Doe')
      await page.fill('[name=email]', 'jane@example.com')
      await page.click('button:has-text("Create")')
    """

  then "success toast appears"
    test.ts """await expect(page.locator('.toast-success')).toBeVisible()"""
}
```

**Decision:** Defer this feature to 2.0 or later. The minimal viable syntax (string-only clauses) is sufficient for 1.0.

---

## 12. Metrics for Success

How do we measure whether the dual syntax improves DX?

| Metric | Target |
|--------|--------|
| **Adoption rate** | 80% of capabilities use `scenario` blocks within 6 months of 1.0 release |
| **Error rate** | < 5% of compilations trigger E011 (wrong syntax in wrong entity) |
| **Verbosity perception** | User surveys: 70%+ agree "scenarios make capabilities more readable" |
| **Migration friction** | 90%+ of users successfully migrate from `verify e2e` to `scenario` without errors |
| **LSP usage** | 60%+ of scenario blocks created via LSP autocomplete (not hand-typed) |

---

## Decision

**APPROVED: Implement the dual syntax with minimal viable scenario syntax.**

### Summary

- **Add `scenario` blocks** to the DSL, allowed only in `capability` entities.
- **Keep `verify` statements** for behavior, invariant, event, and constraint entities.
- **Scenario syntax:** Only `given`, `when`, `then` keywords. No `and`, `but`, `examples`.
- **Validation:** Scenarios must have at least one `when` and one `then`. `given` is optional.
- **Deprecation path:** Emit W010 for `verify e2e` in capabilities. Remove in 2.0.
- **LSP support:** Autocomplete snippets for scenarios, semantic highlighting for keywords.
- **Code generation:** Emit test scaffolds with TODO comments for each Given/When/Then clause.

### Rationale

1. **Entity-driven clarity:** Scenarios live in capabilities, verify statements live in behaviors/invariants/events/constraints. The boundary is natural.
2. **Testing pyramid alignment:** Verify maps to the bottom/middle, scenario maps to the top.
3. **Cross-functional communication:** Scenarios are readable by PMs, QAs, designers. Verify statements are developer-focused.
4. **Minimal syntax:** 3 keywords (given/when/then) keep the grammar simple.
5. **Migration is smooth:** Current usage of `e2e` is minimal. Deprecation path is clear.

### Open Questions for Implementation

1. **Should scenarios support inline test code annotations?** (Defer to 2.0)
2. **Should scenarios support parametric tables?** (Defer to 2.0)
3. **Should scenarios support nested steps?** (No — keep flat structure)
4. **Should `and`/`but` be added for readability?** (No — explicit given/when/then is clearer)

---

## Appendix: Grammar Rules

### Tree-sitter Grammar Addition

```javascript
scenario: $ => seq(
  'scenario',
  field('name', $.string),
  field('body', $.scenario_body)
),

scenario_body: $ => seq(
  '{',
  repeat(choice(
    $.given_clause,
    $.when_clause,
    $.then_clause
  )),
  '}'
),

given_clause: $ => seq('given', $.string),
when_clause:  $ => seq('when',  $.string),
then_clause:  $ => seq('then',  $.string),
```

### Validation AST Checks

```rust
fn validate_scenario(scenario: &Scenario) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  // E014: scenario missing 'when' clause
  if scenario.when_clauses.is_empty() {
    diagnostics.push(Diagnostic::error(E014, "scenario missing required 'when' clause"));
  }

  // E014: scenario missing 'then' clause
  if scenario.then_clauses.is_empty() {
    diagnostics.push(Diagnostic::error(E014, "scenario missing required 'then' clause"));
  }

  diagnostics
}
```

---

## References

- RES-11a: Core compiler architecture
- RES-14: Entity testability classification (5 testable entities)
- W004: Existing warning for behaviors without verify statements
- E011: Proposed error code for invalid syntax in capability blocks
- E013: Proposed error code for empty scenario blocks
- E014: Proposed error code for incomplete scenarios
- W012: Proposed warning for capabilities without scenarios
