---
id: RES-16
kind: research
title: "Test Execution Integration — How SpecForge Knows Tests Pass"
status: active
date: 2026-03-02
depends_on: [RES-11a, RES-11b, RES-15]
---

# RES-16: Test Execution Integration

## Problem Statement

SpecForge builds a traceability matrix: **spec entity → code file → test file → pass/fail status**. The `tests` field links entities to test files:

```spec
behavior create_user "Create User" {
  verify unit "valid input creates user and returns ID"
  tests ["tests/user_test.go::TestCreateUser", "tests/user.test.ts:45"]
}
```

But SpecForge doesn't know if these tests **pass**. Four options exist:

- **Option A:** SpecForge runs tests itself
- **Option B:** SpecForge consumes test results (standard formats)
- **Option C:** SpecForge uses annotation-based tracing
- **Option D:** SpecForge only validates file existence

This analysis evaluates all four options against SpecForge's architecture, agent workflows, and multi-language requirements.

---

## Architectural Context

### Current State (from RES-11b)

SpecForge already defines a **test framework plugin model**:

```typescript
// @specforge/vitest plugin generates specforge-report.json
{
  "specforge": "1.0",
  "runner": "@specforge/vitest",
  "timestamp": "2026-03-01T14:30:00Z",
  "behaviors": {
    "BEH-MS-001": {
      "tests": [
        {
          "name": "creates user with unique email",
          "file": "tests/user-crud.test.ts",
          "line": 12,
          "status": "pass",
          "duration_ms": 45
        }
      ],
      "status": "covered"
    }
  }
}
```

**Key insight:** RES-11b already chose Option B (consume test results) + annotation-based linkage via `spec()` wrappers.

### Dual Mechanism Design

RES-11b defines **two ways to link tests to specs**:

1. **Static linkage** (Option D): `tests ["path/to/test.ts:45"]` in `.spec` files
2. **Runtime linkage** (Option C): `spec("BEH-MS-001")` wrapper in test code

The question is: how do these two mechanisms interact, and which one knows pass/fail?

---

## Analysis Grid

| Dimension | Option A: Run Tests | Option B: Consume Results | Option C: Annotations | Option D: File Existence |
|-----------|---------------------|---------------------------|----------------------|--------------------------|
| **Complexity (SpecForge)** | 🔴 High — must know how to run every framework (jest, vitest, pytest, go test, cucumber) | 🟢 Low — read standard JSON/XML | 🟡 Medium — parse test output for annotations | 🟢 Low — stat() files |
| **Complexity (User)** | 🟢 Low — `specforge check --run-tests` | 🟡 Medium — run tests, then run specforge | 🟢 Low — add `@specforge` comments | 🟢 Low — no extra steps |
| **Traceability Strength** | 🟢 Strong — knows pass/fail/duration/error | 🟢 Strong — knows pass/fail/duration/error | 🟡 Medium — knows which tests ran, not if they passed (unless parsed from runner output) | 🔴 Weak — only knows file exists |
| **Framework Agnostic** | 🔴 Poor — N framework integrations | 🟢 Excellent — JUnit XML / JSON are standard | 🟡 Medium — depends on runner output format | 🟢 Excellent — file paths are universal |
| **Agent Workflow** | 🟢 Agent can prove work: "I ran tests, here's the proof" | 🟢 Agent can prove work: "Tests passed, here's the report" | 🟡 Agent must: write code → add annotations → run tests → specforge parses output | 🔴 Agent can't prove tests pass, only that files exist |
| **CI Integration** | 🔴 Tight coupling — CI runs specforge, specforge runs tests | 🟢 Loose coupling — CI runs tests independently, specforge reads results | 🟡 Medium — requires parsing test output | 🟢 Loose coupling — no test execution needed |
| **Incremental Development** | 🔴 Slow — must re-run all tests for full coverage report | 🟢 Fast — merge multiple report files from parallel test runs | 🟢 Fast — annotations parsed from test output | 🟢 Fast — file existence checks are instant |
| **Multi-Language** | 🔴 N × M problem (N runners × M config formats) | 🟢 1 standard format (JUnit XML / JSON) | 🟡 N output parsers (one per runner) | 🟢 Universal (file paths) |
| **Failure Details** | 🟢 Direct access to failure message/stacktrace | 🟢 Included in JUnit XML / JSON | 🟡 Depends on parser depth | 🔴 No failure info |
| **Coverage Gating** | 🟢 Can gate on pass/fail | 🟢 Can gate on pass/fail | 🟡 Can gate on "test exists" but not pass/fail (unless parsed) | 🔴 Can only gate on "test exists" |

---

## Decision Recommendation

### Primary Mechanism: **Option B (Consume Test Results)**

**Rationale:**

1. **Already designed in RES-11b** — the `specforge-report.json` format is defined and the plugin model exists
2. **Multi-language native** — JUnit XML is the de facto standard (Java, Python, Go, JS all support it)
3. **CI-friendly** — tests run independently, specforge reads results afterward
4. **Agent-friendly** — agent writes code → runs tests → verifies specforge coverage report shows green
5. **Framework agnostic** — specforge doesn't need to know about pytest vs unittest vs vitest vs jest
6. **Parallel execution** — test runners emit reports, specforge merges them

### Secondary Mechanism: **Option D (File Existence)**

**When to use:** Early development, when tests are declared but not yet written.

**Behavior:**

```spec
behavior create_user "Create User" {
  verify unit "valid input creates user"
  tests ["tests/user_test.go::TestCreateUser"]  // File exists but no report yet
}
```

```bash
$ specforge trace create_user

create_user "Create User"
  ✓ verify: unit "valid input creates user"
  ⚠ tests: tests/user_test.go::TestCreateUser (file exists, no test result)
    help: Run tests and provide --test-results to see pass/fail status
```

**Coverage levels:**

| Level | Criteria | Status Indicator |
|-------|----------|-----------------|
| **Declared** | Entity has `verify` or `scenario` statements | ℹ️ (info) |
| **Linked** | Entity has `tests` field pointing to existing files | ⚠️ (warning - file exists, no results) |
| **Executed** | Test results file shows test ran | ⚠️ (warning - if failed) / ✅ (pass) |
| **Passing** | Test results show `status: "pass"` | ✅ (success) |

### Tertiary Mechanism: **Option C (Annotations) — But Only for Runtime Linkage**

**RES-11b already defines this:**

```typescript
spec("BEH-MS-001", () => {
  it("creates user", () => { ... })
})
```

**Purpose:** Dynamic linkage when test file paths aren't known at spec-writing time, OR when multiple test files test the same behavior.

**SpecForge doesn't parse annotations** — the test runner plugin does. The plugin emits `specforge-report.json` which maps `BEH-MS-001` → test results.

**Relationship to static linkage:**

```spec
behavior create_user {
  verify unit "creates user"
  tests ["tests/user_test.go"]  // Optional — for documentation/navigation
}
```

```go
// tests/user_test.go
func TestCreateUser(t *testing.T) {
    specforge.Spec(t, "create_user")  // Runtime linkage
    // test code
}
```

The `tests` field is **optional documentation**. The `spec()` wrapper is **mandatory for coverage tracking**.

---

## Implementation Design

### Phase 1: File Existence (Validation Only)

**When:** Now (early compiler development)

**What:**

```rust
// In validator passes
fn validate_tests_field(entity: &Entity, graph: &SpecGraph) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if let Some(FieldValue::StringList(paths)) = entity.fields.get("tests") {
        for path in paths {
            if !file_exists(path) {
                diagnostics.push(Diagnostic::warning(
                    "W018",
                    format!("Test file not found: {}", path),
                    entity.span,
                ));
            }
        }
    }
    diagnostics
}
```

**CLI:**

```bash
$ specforge check
warning[W018]: test file not found: tests/user_test.go
  --> behaviors/user.spec:8:10
   |
 8 |   tests ["tests/user_test.go"]
   |          ^^^^^^^^^^^^^^^^^^^^^ file does not exist
```

**No test execution.** Just validation.

---

### Phase 2: Consume Test Results (Coverage Gating)

**When:** After core compiler is stable (RES-11a steps 1-7 complete)

**What:**

```bash
# CI pipeline
npm test -- --reporter=@specforge/vitest   # Emits specforge-report.json
go test -json ./... | specforge collect go # Emits specforge-report.json

# Merge and validate
specforge coverage --test-results specforge-report.json --min 95
```

**CLI implementation:**

```bash
$ specforge coverage --test-results reports/*.json

BEHAVIOR COVERAGE
═══════════════════════════════════════════════════════════════
ID          Title               Tests  Pass  Fail  Skip  Status
───────────────────────────────────────────────────────────────
create_user Create User         3      3     0     0     ✅ pass
read_user   Read User by ID     2      2     0     0     ✅ pass
update_user Update User Email   1      0     1     0     ❌ FAIL
delete_user Delete User         0      —     —     —     ⚠️  no tests
───────────────────────────────────────────────────────────────
Total: 3/4 behaviors passing (75%)
       6 tests | 5 pass | 1 fail | 0 skip
Threshold: 95%    ❌ FAIL

FAILING TESTS
═══════════════════════════════════════════════════════════════
update_user "Update User Email"
  ❌ tests/user_test.go:45 "concurrent updates"
     AssertionError: expected 1 success, got 2
     Duration: 230ms

Exit code: 1
```

**Report format support:**

| Format | Support | Notes |
|--------|---------|-------|
| `specforge-report.json` | ✅ Primary | Generated by `@specforge/vitest`, `@specforge/pytest`, `specforge collect go` |
| JUnit XML | ✅ Fallback | Universal, but lossy (no behavior ID mapping unless test names match) |
| JSON (vitest/jest native) | ⚠️ Possible | Framework-specific, requires per-runner parser |
| TAP | ⚠️ Possible | Old standard, limited metadata |

**Behavior ID resolution:**

1. **Explicit linkage** (primary): Test result contains behavior ID via `spec()` wrapper
2. **Test path matching** (fallback): Match test file path from `tests` field to report file path
3. **Test name pattern** (heuristic): If test name contains entity ID (e.g., `TestCreateUser_BEH_MS_001`), link it

---

### Phase 3: Plugin Ecosystem (Future)

**When:** After initial users validate the model

**What:**

```bash
npm install @specforge/vitest @specforge/pytest
brew install specforge-go-plugin
```

**Plugin API:**

```rust
// Plugin receives graph via stdin, emits augmented graph via stdout
pub trait CoveragePlugin {
    fn augment_coverage(&self, graph: &SpecGraph) -> CoverageReport;
}
```

**Community plugins:**

- `@specforge/cucumber` — Gherkin → `scenario` block mapping
- `@specforge/playwright` — E2E test linkage
- `@specforge/k6` — Load test linkage to `verify load` statements

---

## Agent Workflow Integration

### Scenario: Agent implements a behavior and proves it works

**Step 1: Agent reads spec**

```bash
$ specforge show create_user

behavior create_user "Create User"
  contract: "MUST create user with unique email"
  verify unit "valid input creates user and returns ID"
  verify unit "duplicate email returns DuplicateEmailError"
  tests: (none)
  status: ⚠️ missing tests
```

**Step 2: Agent writes code + tests**

```go
// internal/user/repository.go
func (r *Repository) Create(cmd CreateUserCommand) (*User, error) {
    // implementation
}

// tests/user_test.go
func TestCreateUser(t *testing.T) {
    specforge.Spec(t, "create_user")
    // ... 2 test cases
}
```

**Step 3: Agent runs tests**

```bash
$ go test -json ./... | specforge collect go
✅ Generated: specforge-report.json (1 behavior, 2 tests, all pass)
```

**Step 4: Agent verifies traceability**

```bash
$ specforge coverage

BEHAVIOR COVERAGE
═══════════════════════════════════════════════════════════════
create_user Create User    2      2     0     0     ✅ pass
───────────────────────────────────────────────────────────────
Total: 1/1 behaviors passing (100%)
```

**Step 5: Agent commits**

```bash
$ git add internal/user tests/user_test.go specforge-report.json
$ git commit -m "Implement create_user behavior with full test coverage"
```

**Agent can prove its work:** Not just "I wrote code" but "I wrote code, tests pass, coverage is green."

---

## Edge Cases & Decisions

### 1. Multiple test runners for one behavior

**Scenario:** TypeScript unit tests + Go integration tests both test `create_user`

**Solution:** Merge reports. All must pass for behavior to be "passing."

```json
// ts-report.json
{ "behaviors": { "create_user": { "tests": [{"status": "pass"}], "status": "covered" }}}

// go-report.json
{ "behaviors": { "create_user": { "tests": [{"status": "pass"}], "status": "covered" }}}
```

```bash
$ specforge coverage --test-results ts-report.json go-report.json
✅ create_user: 2 tests (1 ts, 1 go), all pass
```

### 2. Test file declared but no runtime linkage

**Scenario:**

```spec
behavior create_user {
  tests ["tests/user_test.go"]
}
```

But test file doesn't have `specforge.Spec(t, "create_user")`.

**Behavior:**

```bash
$ specforge coverage --test-results report.json
⚠️  create_user: test file declared (tests/user_test.go) but no coverage data in report
    help: Add specforge.Spec(t, "create_user") to link test results
```

**Rationale:** Static linkage (`tests` field) is documentation. Runtime linkage (`spec()` wrapper) is truth for coverage.

### 3. Runtime linkage but no static declaration

**Scenario:**

```go
func TestCreateUser(t *testing.T) {
    specforge.Spec(t, "create_user")
}
```

But `.spec` file has no `tests ["tests/user_test.go"]`.

**Behavior:**

```bash
$ specforge coverage --test-results report.json
✅ create_user: 1 test, pass
```

**Rationale:** The `tests` field is **optional**. Runtime linkage is sufficient for coverage.

### 4. Test result for unknown behavior ID

**Scenario:**

```go
specforge.Spec(t, "create_orderrrr")  // Typo
```

**Behavior:**

The test **fails at runtime** (plugin validates IDs against `.spec` files).

```bash
$ go test
--- FAIL: TestCreateOrder (0.00s)
    specforge_test.go:12: unknown behavior ID "create_orderrrr"
        Did you mean: create_order
        Available: create_user, read_user, update_user
```

**Rationale:** Fail fast. Prevent drift between specs and tests.

---

## Comparison to Alternatives

### vs. Cucumber/Gherkin

| Aspect | SpecForge | Cucumber |
|--------|-----------|----------|
| **Test = Spec** | No. Spec is contract. Test is verification. | Yes. `.feature` files are executable. |
| **Tooling** | Standard test runners (vitest, pytest, go test) + lightweight plugin | Custom runner (cucumber-js, behave) |
| **Non-testable entities** | Declarative entities (type, port, decision, ref) don't require tests | Everything is a scenario (awkward for data models) |
| **Traceability** | Bidirectional (spec → test, test → spec) | One-way (feature → test) |
| **Coverage** | Entity-level + statement-level + file-level | Scenario-level only |

**Key difference:** SpecForge separates **specification** (`.spec` files) from **verification** (test files). Cucumber merges them (`.feature` files are both).

### vs. SysML Requirements

| Aspect | SpecForge | SysML |
|--------|-----------|-------|
| **Test linkage** | `tests` field + runtime wrapper | `satisfy` relationship (manual) |
| **Pass/fail tracking** | Automated (via test results) | Manual or tool-specific |
| **Format** | Text DSL | UML/XML (tool-heavy) |

**Key difference:** SysML requires modeling tools (Enterprise Architect, MagicDraw). SpecForge is text-first.

---

## Standard Report Format: `specforge-report.json`

**Version 1.0 schema:**

```typescript
interface SpecForgeReport {
  specforge: "1.0";
  runner: string;  // "@specforge/vitest" | "@specforge/pytest" | "specforge-go"
  timestamp: string;  // ISO 8601
  behaviors: Record<string, BehaviorCoverage>;
  invariants?: Record<string, InvariantCoverage>;
}

interface BehaviorCoverage {
  tests: TestResult[];
  status: "covered" | "missing" | "partial";
}

interface TestResult {
  name: string;
  file: string;
  line?: number;
  status: "pass" | "fail" | "skip";
  duration_ms?: number;
  error?: {
    message: string;
    stack?: string;
  };
}

interface InvariantCoverage {
  violations: TestResult[];  // Tests proving the invariant is enforced
  status: "covered" | "missing";
}
```

**JUnit XML mapping (fallback):**

```xml
<testsuite name="behaviors.create_user">
  <testcase name="valid input creates user" file="tests/user_test.go" line="12" time="0.045">
    <!-- pass -->
  </testcase>
  <testcase name="duplicate email fails" file="tests/user_test.go" line="28" time="0.032">
    <failure message="expected error, got nil">...</failure>
  </testcase>
</testsuite>
```

**Heuristic mapping:**

- `<testsuite name="behaviors.create_user">` → behavior ID = `create_user`
- If test name contains entity ID (e.g., `BEH_MS_001`), extract and link

---

## Implementation Roadmap

| Phase | Deliverable | CLI Commands | Exit Criteria |
|-------|-------------|--------------|---------------|
| **1. Validation** | File existence check | `specforge check` | W018 warns if test file doesn't exist |
| **2. Basic Coverage** | Read `specforge-report.json` | `specforge coverage --test-results X` | Shows pass/fail/missing per behavior |
| **3. Threshold Gating** | Enforce minimum coverage | `specforge coverage --min 95` | Exit code 1 if below threshold |
| **4. Multi-Report Merge** | Merge multiple reports | `specforge coverage --test-results *.json` | Aggregates across runners (ts, py, go) |
| **5. JUnit XML Fallback** | Parse JUnit XML | `specforge coverage --junit junit.xml` | Heuristic linkage when no plugin used |
| **6. Plugin Ecosystem** | `@specforge/vitest` npm package | `npm install @specforge/vitest` | Test runner emits `specforge-report.json` |

**Current status:** Phase 1 ready to implement (W018 diagnostic).

**Phases 2-3:** Core compiler feature, no plugins needed.

**Phases 4-6:** Plugin development (depends on RES-11b schedule).

---

## Final Recommendation

### Primary: **Option B (Consume Test Results)**

```bash
# User workflow
npm test                                      # Emits specforge-report.json (via plugin)
specforge coverage --test-results report.json # Validates coverage

# CI workflow
go test -json ./... | specforge collect go   # Emits report
specforge coverage --min 95                   # Gates on threshold
```

**Why:**

1. **Multi-language native** — works with any test runner that outputs JUnit XML or JSON
2. **CI-friendly** — loose coupling (tests run independently)
3. **Agent-friendly** — agent proves tests pass by showing coverage report
4. **Already designed** — RES-11b defines the format and plugin model
5. **Framework agnostic** — specforge doesn't need to know pytest vs unittest vs go test

### Secondary: **Option D (File Existence)**

Early warning system. If test file declared but doesn't exist → W018.

### Tertiary: **Option C (Annotations)**

Already part of the design via `spec()` wrappers. Not parsed by specforge — parsed by test runner plugins.

### Rejected: **Option A (Run Tests)**

Tight coupling, N × M complexity (runners × config formats), slow, framework-dependent.

---

## Appendices

### Appendix A: Test Result Format Survey

| Runner | Native Format | JUnit XML | JSON | Notes |
|--------|---------------|-----------|------|-------|
| **vitest** | JSON | ✅ (via reporter) | ✅ native | Fast, modern |
| **jest** | JSON | ✅ (via reporter) | ✅ native | Widely used |
| **pytest** | plaintext | ✅ (`--junitxml`) | ⚠️ (via plugin) | XML is standard |
| **go test** | plaintext | ⚠️ (via 3rd party) | ✅ (`-json` flag) | JSON preferred |
| **cucumber** | JSON | ✅ (via formatter) | ✅ native | Gherkin-specific |
| **playwright** | JSON | ✅ (via reporter) | ✅ native | E2E focus |

**Consensus:** JUnit XML is universal. JSON is preferred (richer metadata).

### Appendix B: Validation Code Summary

**New validation codes:**

| Code | Severity | Condition | Message |
|------|----------|-----------|---------|
| **W018** | Warning | Test file in `tests` field doesn't exist | `"test file not found: {path}"` |
| **W019** | Warning | Behavior has `tests` field but no runtime linkage in report | `"test file declared but no coverage data in report"` |
| **I007** | Info | Behavior covered by multiple runners | `"{id} tested by {count} runners: {list}"` |

**Total validation codes:** 35 + 3 = 38 (14 errors, 17 warnings, 7 info).

---

## References

- **RES-11a:** Core compiler architecture (parser, resolver, validator)
- **RES-11b:** Code generation & test plugins (defines `specforge-report.json`)
- **RES-15:** Verify + scenario dual syntax (test declaration model)
- **JUnit XML:** https://github.com/testmoapp/junitxml
- **Vitest reporters:** https://vitest.dev/guide/reporters.html
- **pytest-json-report:** https://pypi.org/project/pytest-json-report/
- **go test -json:** https://pkg.go.dev/cmd/test2json
