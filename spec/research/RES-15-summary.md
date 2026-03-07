# RES-15 Executive Summary: Verify vs Scenario Dual Syntax

## TL;DR

**Recommendation: APPROVED with minimal viable syntax**

- Add `scenario` blocks with `given/when/then` keywords (only in `capability` entities)
- Keep `verify` statements for behaviors, invariants, events, constraints
- No `and`, `but`, or `examples` tables (YAGNI)
- Migration is smooth (current `verify e2e` usage is minimal: 2 occurrences)

---

## The Rule (1 sentence)

**Use `verify` to test code (unit/integration/property/load). Use `scenario` to test user flows (e2e acceptance).**

---

## Syntax Comparison

### Before (Status Quo)
```spec
behavior "Create User" {
  verify unit        "valid input creates user"
  verify integration "created user is queryable"
  verify e2e         "admin creates user through UI" // ← loses flow structure
}
```

### After (Proposed)
```spec
behavior "Create User" {
  verify unit        "valid input creates user"
  verify integration "created user is queryable"
}

capability "Admin Creates User" {
  scenario "successful creation" {
    given "admin is on user management page"
    when  "admin fills form and clicks Create"
    then  "success toast appears"
    then  "new user appears in list"
  }
}
```

---

## Why This Works

### 1. Entity-Driven Boundary
- `scenario` only lives in `capability` blocks (user stories)
- `verify` only lives in `behavior`, `invariant`, `event`, `constraint` blocks (code tests)
- The entity type tells you which syntax to use

### 2. Testing Pyramid Alignment
```
     scenario (e2e)      ← Top: user flows in capabilities
       /      \
  verify load|property   ← Middle: quality attributes
      /          \
verify unit|integration  ← Bottom: isolated tests in behaviors
```

### 3. Cross-Functional Readability
- PMs, QAs, designers can read scenarios
- `verify e2e "..."` is opaque; `given/when/then` is self-documenting

### 4. Minimal Syntax = Low Cognitive Load
- Only 3 keywords: `given`, `when`, `then`
- No step definitions, no parametric tables, no nested features
- Descriptive (not executable) — test framework handles execution

---

## Common Mistakes (and how compiler catches them)

### ❌ Using verify in capability
```spec
capability "Admin Creates User" {
  verify e2e "admin creates user"  // ← wrong
}
```
**Compiler:** `error[E011]: capabilities use 'scenario' blocks, not 'verify' statements`

### ❌ Using scenario in behavior
```spec
behavior "Create User" {
  scenario "create user" { ... }  // ← wrong
}
```
**Compiler:** `error[E011]: behaviors use 'verify' statements, not 'scenario' blocks`

### ❌ Empty scenario
```spec
scenario "successful creation" { }  // ← wrong
```
**Compiler:** `error[E014]: scenario missing required 'when' and 'then' clauses`

---

## Migration Path

1. **Current usage:** 2 occurrences of `verify e2e` in the codebase
2. **1.0 transition:** Emit `W010: deprecated syntax` for `verify e2e` in capabilities
3. **2.0 removal:** `verify e2e` is invalid in capabilities; scenarios are required
4. **Automated migration:** `specforge migrate` converts `verify e2e "..."` → `scenario "..." { /* TODO */ }`

**Verdict:** Smooth migration, low friction.

---

## LSP Support

| Context | Autocomplete | Diagnostic |
|---------|-------------|------------|
| Inside `behavior` | `verify unit\|integration\|property\|load` | W004 if no verify statements |
| Inside `capability` | `scenario "name" { given/when/then }` | W012 if no scenarios |
| Inside `scenario` | `given\|when\|then "..."` | E014 if missing when/then |

---

## AI Agent Consumption

Scenarios in capabilities serve as structured context for AI agents. Agents consume the entity graph and produce test code (e.g., Playwright tests) directly from scenario data — no code generation step needed.

---

## Alternatives Considered (and rejected)

### Option A: Keep verify e2e only
**Problem:** Loses Given/When/Then structure, poor cross-functional readability

### Option B: Keep scenario only
**Problem:** Too verbose for simple unit tests (unnecessary Given/When/Then for 1-line assertions)

### Option C: Add `and`, `but`, `examples` tables
**Problem:** Unnecessary complexity. SpecForge scenarios are descriptive (not executable), so Gherkin's step-reuse features don't apply.

---

## Validation Rules

| Code | Severity | Message |
|------|----------|---------|
| E011 | Error | Invalid syntax in entity (scenario in behavior / verify in capability) |
| E013 | Error | Empty scenario block |
| E014 | Error | Scenario missing required `when` or `then` clause |
| W012 | Warning | Capability lacks scenario blocks (e2e coverage missing) |

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Adoption rate | 80% of capabilities use scenarios within 6 months |
| Error rate | < 5% of compilations trigger E011 (wrong syntax in wrong entity) |
| Verbosity perception | 70%+ agree "scenarios improve capability readability" |
| Migration friction | 90%+ successfully migrate without errors |
| LSP usage | 60%+ of scenarios created via autocomplete |

---

## Decision: APPROVED

**Implement dual syntax with minimal viable scenario syntax (given/when/then only).**

- Clear entity-driven boundary (scenario in capability, verify in behavior/invariant/event/constraint)
- Testing pyramid alignment (scenario = e2e, verify = unit/integration/property/load)
- Low cognitive load (3 keywords, no step definitions)
- Smooth migration (minimal current usage, clear deprecation path)
- LSP support is straightforward (entity-aware autocomplete)

**Defer to 2.0 or later:** Inline test code annotations, parametric tables, `and`/`but` keywords.

---

## Next Steps

1. Extend tree-sitter grammar to support `scenario { given/when/then }`
2. Add validation rules (E011, E013, E014, W012)
3. Update LSP for scenario autocomplete and semantic highlighting
4. Document scenario syntax for AI agent consumption
5. Update docs with decision tree and examples
6. Add deprecation warning W010 for `verify e2e` in capabilities
7. Plan removal of `verify e2e` in 2.0
