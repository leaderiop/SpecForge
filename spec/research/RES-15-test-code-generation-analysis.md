---
id: RES-15
kind: research
title: "Test Code Generation Analysis — verify Statements vs. scenario Blocks"
status: active
date: 2026-03-02
depends_on: [RES-11b, RES-14]
---

# RES-15: Test Code Generation Analysis

## Problem Statement

SpecForge generates test scaffolding for 5 testable entities (behavior, invariant, event, constraint, capability). Two syntaxes exist for declaring test requirements:

1. **`verify` statements** — one-liner test requirement attached to behavior/invariant/event/constraint
2. **`scenario` blocks** — Gherkin-style Given-When-Then e2e test specification (proposed, not yet implemented)

This analysis determines:
- What concrete test code each syntax should generate
- How the generated code differs across languages/frameworks
- Whether `scenario` blocks generate `.feature` files or code directly
- How traceability metadata is embedded in generated tests
- What metadata/comments should appear in generated test files

---

## Current State: `verify` Statements

### Syntax

```spec
behavior create_user {
  contract "..."

  verify unit        "insert user, retrieve by ID, assert equal"
  verify integration "insert user, restart process, retrieve persists"
  verify property    "email uniqueness holds under concurrent inserts"
  verify load        "1000 concurrent creates under 5s p99"
}
```

### Five Test Types

| Type | Meaning | Typical Framework |
|------|---------|-------------------|
| `unit` | Isolated, no external dependencies | Vitest, pytest, cargo test |
| `integration` | Real dependencies (DB, network) | Vitest, pytest, go test |
| `property` | Property-based / generative testing | fast-check, Hypothesis, proptest |
| `load` | Performance / load testing | k6, Locust, Gatling |
| `e2e` | End-to-end through full system | Playwright, Cypress, Selenium |

---

## Generated Code: `verify` Statements

### TypeScript (Vitest)

**Input:**
```spec
behavior create_user {
  contract """
    When a valid CreateUserCommand is received,
    the system MUST create a user record with unique email
    and MUST return Result<User, DuplicateEmailError>.
  """

  verify unit        "valid input creates user and returns ID"
  verify integration "created user is queryable by ID"
  verify property    "email uniqueness holds under concurrent creation"
  verify load        "handles 1000 concurrent requests under 200ms p99"
}
```

**Generated:** `tests/behaviors/create_user.test.ts`
```typescript
/**
 * Test suite for behavior: create_user
 *
 * @specforge-behavior create_user
 * @specforge-file behaviors/user-crud.spec
 * @specforge-generated 2026-03-02T14:30:00Z
 *
 * Contract:
 * When a valid CreateUserCommand is received,
 * the system MUST create a user record with unique email
 * and MUST return Result<User, DuplicateEmailError>.
 *
 * DO NOT EDIT the @specforge annotations above.
 * Implement the test bodies below.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { spec } from '@specforge/vitest';
import type { UserRepository } from '../generated/ports/user-repository';
import type { CreateUserCommand, User, DuplicateEmailError } from '../generated/types/user';

spec('create_user', () => {
  let repo: UserRepository;

  beforeEach(() => {
    // TODO: Initialize test repository
    // repo = createTestUserRepository();
  });

  // @specforge-verify unit
  it('valid input creates user and returns ID', async () => {
    // TODO: Implement test
    // const cmd: CreateUserCommand = {
    //   email: 'alice@example.com',
    //   name: 'Alice',
    //   role: 'admin'
    // };
    // const result = await repo.create(cmd);
    // expect(result.isOk()).toBe(true);
    // const user = result.value;
    // expect(user.email).toBe(cmd.email);
    // expect(user.name).toBe(cmd.name);
  });

  // @specforge-verify integration
  it('created user is queryable by ID', async () => {
    // TODO: Implement integration test with real database
    // 1. Create user via repo.create()
    // 2. Query by ID via repo.findById()
    // 3. Assert returned user matches created user
  });

  // @specforge-verify property
  it('email uniqueness holds under concurrent creation', async () => {
    // TODO: Implement property-based test
    // Use fast-check or similar library
    // Generate multiple CreateUserCommand inputs
    // Ensure concurrent creates with same email result in exactly one success
  });

  // @specforge-verify load
  it('handles 1000 concurrent requests under 200ms p99', async () => {
    // TODO: Implement load test
    // Note: Consider moving to separate k6 or Artillery script
    // This test should verify that under concurrent load:
    // - p99 latency < 200ms
    // - All requests succeed or fail with proper errors
    // - No race conditions or data corruption
  });
});
```

**Key Features:**
- **File header comment** with behavior ID, spec file path, generation timestamp, and full contract text
- **`spec('create_user')` wrapper** from `@specforge/vitest` — registers tests with behavior ID
- **One test per `verify` statement** with description as test name
- **`@specforge-verify {type}` annotations** for tooling to identify test type
- **TODO comments** with guidance on what to implement
- **Generated types imported** from code generator output

---

### Rust (#[test])

**Generated:** `tests/behaviors/create_user.rs`
```rust
//! Test suite for behavior: create_user
//!
//! @specforge-behavior create_user
//! @specforge-file behaviors/user-crud.spec
//! @specforge-generated 2026-03-02T14:30:00Z
//!
//! Contract:
//! When a valid CreateUserCommand is received,
//! the system MUST create a user record with unique email
//! and MUST return Result<User, DuplicateEmailError>.

use crate::ports::UserRepository;
use crate::types::{CreateUserCommand, User, UserRole, DuplicateEmailError};
use specforge_test::behavior;

/// @specforge-verify unit
#[test]
#[behavior("create_user")]
fn valid_input_creates_user_and_returns_id() {
    // TODO: Implement test
    // let repo = create_test_user_repository();
    // let cmd = CreateUserCommand {
    //     email: "alice@example.com".into(),
    //     name: "Alice".into(),
    //     role: UserRole::Admin,
    // };
    // let result = repo.create(cmd);
    // assert!(result.is_ok());
    // let user = result.unwrap();
    // assert_eq!(user.email, "alice@example.com");
}

/// @specforge-verify integration
#[test]
#[behavior("create_user")]
#[ignore = "integration test - requires database"]
fn created_user_is_queryable_by_id() {
    // TODO: Implement integration test with real database
    // 1. Create user via repo.create()
    // 2. Query by ID via repo.find_by_id()
    // 3. Assert returned user matches created user
}

/// @specforge-verify property
#[test]
#[behavior("create_user")]
fn email_uniqueness_holds_under_concurrent_creation() {
    // TODO: Implement property-based test
    // Use proptest or quickcheck
    // Generate CreateUserCommand instances
    // Verify concurrent creates with same email result in exactly one success
}

/// @specforge-verify load
#[test]
#[behavior("create_user")]
#[ignore = "load test - run separately"]
fn handles_1000_concurrent_requests_under_200ms_p99() {
    // TODO: Implement load test
    // Consider using criterion for benchmarking
    // Measure p99 latency under concurrent load
}
```

**Key Features:**
- **`#[behavior("create_user")]` attribute** from `specforge_test` crate — registers with behavior ID
- **`#[ignore]` on integration/load tests** — excluded from default `cargo test` run
- **Doc comments** with contract text for `cargo doc`
- **Module-level doc comment** with full metadata

---

### Python (pytest)

**Generated:** `tests/behaviors/test_create_user.py`
```python
"""
Test suite for behavior: create_user

@specforge-behavior create_user
@specforge-file behaviors/user-crud.spec
@specforge-generated 2026-03-02T14:30:00Z

Contract:
When a valid CreateUserCommand is received,
the system MUST create a user record with unique email
and MUST return Result<User, DuplicateEmailError>.

DO NOT EDIT the @specforge annotations above.
Implement the test bodies below.
"""

import pytest
from hypothesis import given, strategies as st
from specforge.pytest import spec

from src.generated.ports.user_repository import UserRepository
from src.generated.types.user import CreateUserCommand, User, UserRole, DuplicateEmailError


@spec("create_user")
class TestCreateUser:
    """Unit and integration tests for create_user behavior."""

    @pytest.fixture
    def repo(self) -> UserRepository:
        """TODO: Return test repository instance."""
        raise NotImplementedError("Initialize test repository")

    # @specforge-verify unit
    def test_valid_input_creates_user_and_returns_id(self, repo: UserRepository) -> None:
        """Valid input creates user and returns ID."""
        # TODO: Implement test
        # cmd = CreateUserCommand(
        #     email="alice@example.com",
        #     name="Alice",
        #     role=UserRole.ADMIN
        # )
        # result = repo.create(cmd)
        # assert result.is_ok()
        # user = result.ok()
        # assert user.email == cmd.email
        # assert user.name == cmd.name

    # @specforge-verify integration
    @pytest.mark.integration
    def test_created_user_is_queryable_by_id(self, repo: UserRepository) -> None:
        """Created user is queryable by ID."""
        # TODO: Implement integration test with real database
        # 1. Create user via repo.create()
        # 2. Query by ID via repo.find_by_id()
        # 3. Assert returned user matches created user

    # @specforge-verify property
    @pytest.mark.property
    def test_email_uniqueness_holds_under_concurrent_creation(self, repo: UserRepository) -> None:
        """Email uniqueness holds under concurrent creation."""
        # TODO: Implement property-based test using Hypothesis
        # @given(st.emails(), st.text())
        # Test concurrent creates with same email

    # @specforge-verify load
    @pytest.mark.load
    @pytest.mark.skip(reason="load test - run separately")
    def test_handles_1000_concurrent_requests_under_200ms_p99(self, repo: UserRepository) -> None:
        """Handles 1000 concurrent requests under 200ms p99."""
        # TODO: Implement load test
        # Consider using Locust or separate performance test suite
```

**Key Features:**
- **`@spec("create_user")` decorator** from `specforge.pytest` — registers test class with behavior ID
- **Pytest markers** (`@pytest.mark.integration`, `@pytest.mark.load`) for selective test execution
- **Class-based organization** with fixture for shared setup
- **Docstrings** with test descriptions
- **Type hints** for generated types

---

## Generated Code: `scenario` Blocks (Proposed)

### Syntax (Not Yet Implemented)

```spec
use features/user-management

capability create_user_web {
  persona  admin
  surface  [web]
  features [user_management]

  scenario "admin creates user successfully" {
    given "admin is on user management page"
    when  "admin fills name, email, role and clicks Create"
    then  "success toast appears"
    then  "new user appears in user list"
  }

  scenario "duplicate email shows error" {
    given "user alice@example.com already exists"
    given "admin is on user management page"
    when  "admin fills form with email alice@example.com"
    when  "admin clicks Create"
    then  "error message 'Email already in use' appears"
    then  "form remains filled"
    then  "user count stays the same"
  }
}
```

### Decision: Direct Code Generation (Not .feature Files)

**Rationale:**
1. **SpecForge owns the source of truth** — `.spec` files are the canonical specification, not `.feature` files
2. **Simpler toolchain** — no need for Cucumber/Behave/Gherkin parsers
3. **Type-safe step implementations** — generated code uses generated types/ports
4. **Better IDE integration** — tests are native TypeScript/Python/Rust, not string parsing
5. **Easier to debug** — standard test runner, standard debugger, standard stack traces

**Trade-off:** Lose Cucumber's plain-language readability for non-technical stakeholders. But SpecForge's target audience is engineers, and the `.spec` files themselves are the readable specification.

---

### TypeScript (Playwright)

**Generated:** `tests/capabilities/create_user_web.spec.ts`
```typescript
/**
 * E2E test suite for capability: create_user_web
 *
 * @specforge-capability create_user_web
 * @specforge-file capabilities/user-management.spec
 * @specforge-generated 2026-03-02T14:30:00Z
 *
 * Persona: admin
 * Surface: web
 * Features: user_management
 */

import { test, expect } from '@playwright/test';
import { scenario } from '@specforge/playwright';

scenario('create_user_web', 'admin creates user successfully', ({ given, when, then }) => {
  test('admin creates user successfully', async ({ page }) => {
    // @specforge-step given "admin is on user management page"
    await given('admin is on user management page', async () => {
      // TODO: Implement navigation
      // await page.goto('/admin/users');
      // await expect(page.locator('h1')).toContainText('User Management');
    });

    // @specforge-step when "admin fills name, email, role and clicks Create"
    await when('admin fills name, email, role and clicks Create', async () => {
      // TODO: Implement form interaction
      // await page.locator('button:text("New User")').click();
      // await page.locator('input[name="name"]').fill('Alice');
      // await page.locator('input[name="email"]').fill('alice@example.com');
      // await page.locator('select[name="role"]').selectOption('admin');
      // await page.locator('button:text("Create")').click();
    });

    // @specforge-step then "success toast appears"
    await then('success toast appears', async () => {
      // TODO: Implement assertion
      // await expect(page.locator('.toast.success')).toBeVisible();
      // await expect(page.locator('.toast.success')).toContainText('User created');
    });

    // @specforge-step then "new user appears in user list"
    await then('new user appears in user list', async () => {
      // TODO: Implement assertion
      // await expect(page.locator('table tbody tr')).toContainText('alice@example.com');
    });
  });
});

scenario('create_user_web', 'duplicate email shows error', ({ given, when, then }) => {
  test('duplicate email shows error', async ({ page }) => {
    // @specforge-step given "user alice@example.com already exists"
    await given('user alice@example.com already exists', async () => {
      // TODO: Set up precondition
      // await createUserViaAPI({ email: 'alice@example.com', name: 'Alice', role: 'admin' });
    });

    // @specforge-step given "admin is on user management page"
    await given('admin is on user management page', async () => {
      // await page.goto('/admin/users');
    });

    // @specforge-step when "admin fills form with email alice@example.com"
    await when('admin fills form with email alice@example.com', async () => {
      // await page.locator('button:text("New User")').click();
      // await page.locator('input[name="email"]').fill('alice@example.com');
    });

    // @specforge-step when "admin clicks Create"
    await when('admin clicks Create', async () => {
      // await page.locator('button:text("Create")').click();
    });

    // @specforge-step then "error message 'Email already in use' appears"
    await then('error message appears', async () => {
      // await expect(page.locator('.error')).toContainText('Email already in use');
    });

    // @specforge-step then "form remains filled"
    await then('form remains filled', async () => {
      // await expect(page.locator('input[name="email"]')).toHaveValue('alice@example.com');
    });

    // @specforge-step then "user count stays the same"
    await then('user count stays the same', async () => {
      // const count = await page.locator('table tbody tr').count();
      // expect(count).toBe(1); // Only the original Alice
    });
  });
});
```

**Key Features:**
- **`scenario()` wrapper** from `@specforge/playwright` — provides `given`/`when`/`then` helpers
- **One test per scenario** — full isolation
- **Inline step implementations** — no separate step definition files
- **Step strings preserved as comments** for traceability
- **TODO comments** guide implementation

---

### TypeScript (Cypress)

**Generated:** `cypress/e2e/capabilities/create_user_web.cy.ts`
```typescript
/**
 * E2E test suite for capability: create_user_web
 *
 * @specforge-capability create_user_web
 * @specforge-file capabilities/user-management.spec
 * @specforge-generated 2026-03-02T14:30:00Z
 */

import { scenario } from '@specforge/cypress';

scenario('create_user_web', () => {
  describe('admin creates user successfully', () => {
    it('completes full flow', () => {
      // @specforge-step given "admin is on user management page"
      cy.visit('/admin/users');
      cy.contains('h1', 'User Management').should('be.visible');

      // @specforge-step when "admin fills name, email, role and clicks Create"
      cy.contains('button', 'New User').click();
      cy.get('input[name="name"]').type('Alice');
      cy.get('input[name="email"]').type('alice@example.com');
      cy.get('select[name="role"]').select('admin');
      cy.contains('button', 'Create').click();

      // @specforge-step then "success toast appears"
      cy.get('.toast.success').should('be.visible');
      cy.get('.toast.success').should('contain', 'User created');

      // @specforge-step then "new user appears in user list"
      cy.get('table tbody tr').should('contain', 'alice@example.com');
    });
  });

  describe('duplicate email shows error', () => {
    beforeEach(() => {
      // @specforge-step given "user alice@example.com already exists"
      cy.request('POST', '/api/v1/users', {
        email: 'alice@example.com',
        name: 'Alice',
        role: 'admin'
      });
    });

    it('shows inline error', () => {
      // @specforge-step given "admin is on user management page"
      cy.visit('/admin/users');

      // @specforge-step when "admin fills form with email alice@example.com"
      cy.contains('button', 'New User').click();
      cy.get('input[name="email"]').type('alice@example.com');

      // @specforge-step when "admin clicks Create"
      cy.contains('button', 'Create').click();

      // @specforge-step then "error message appears"
      cy.get('.error').should('contain', 'Email already in use');

      // @specforge-step then "form remains filled"
      cy.get('input[name="email"]').should('have.value', 'alice@example.com');

      // @specforge-step then "user count stays the same"
      cy.get('table tbody tr').should('have.length', 1);
    });
  });
});
```

**Key Features:**
- **Cypress-native syntax** — no Given/When/Then wrappers, just Cypress commands
- **`scenario()` wrapper** for registration with SpecForge
- **Step comments** for traceability
- **`beforeEach` for shared setup** — Given steps that repeat across scenarios

---

### Alternative: Generate .feature Files (Cucumber/Behave)

**If** the user explicitly opts into Cucumber-style output:

**Generated:** `features/capabilities/create_user_web.feature`
```gherkin
# @specforge-capability create_user_web
# @specforge-file capabilities/user-management.spec
# @specforge-generated 2026-03-02T14:30:00Z
#
# Persona: admin
# Surface: web
# Features: user_management

Feature: Create User via Web Dashboard

  Scenario: Admin creates user successfully
    Given admin is on user management page
    When admin fills name, email, role and clicks Create
    Then success toast appears
    And new user appears in user list

  Scenario: Duplicate email shows error
    Given user alice@example.com already exists
    And admin is on user management page
    When admin fills form with email alice@example.com
    And admin clicks Create
    Then error message "Email already in use" appears
    And form remains filled
    And user count stays the same
```

**Generated:** `features/step_definitions/create_user_web.ts` (Cucumber.js)
```typescript
/**
 * Step definitions for capability: create_user_web
 *
 * @specforge-capability create_user_web
 * @specforge-generated 2026-03-02T14:30:00Z
 */

import { Given, When, Then } from '@cucumber/cucumber';
import { expect } from '@playwright/test';

Given('admin is on user management page', async function () {
  // TODO: Implement step
  // await this.page.goto('/admin/users');
  // await expect(this.page.locator('h1')).toContainText('User Management');
});

When('admin fills name, email, role and clicks Create', async function () {
  // TODO: Implement step
  // await this.page.locator('button:text("New User")').click();
  // await this.page.locator('input[name="name"]').fill('Alice');
  // await this.page.locator('input[name="email"]').fill('alice@example.com');
  // await this.page.locator('select[name="role"]').selectOption('admin');
  // await this.page.locator('button:text("Create")').click();
});

Then('success toast appears', async function () {
  // TODO: Implement assertion
  // await expect(this.page.locator('.toast.success')).toBeVisible();
});

Then('new user appears in user list', async function () {
  // TODO: Implement assertion
  // await expect(this.page.locator('table tbody tr')).toContainText('alice@example.com');
});

Given('user {string} already exists', async function (email: string) {
  // TODO: Set up precondition
  // await createUserViaAPI({ email, name: 'Test User', role: 'admin' });
});

When('admin fills form with email {string}', async function (email: string) {
  // TODO: Implement step
  // await this.page.locator('input[name="email"]').fill(email);
});

Then('error message {string} appears', async function (message: string) {
  // TODO: Implement assertion
  // await expect(this.page.locator('.error')).toContainText(message);
});

Then('form remains filled', async function () {
  // TODO: Implement assertion
  // await expect(this.page.locator('input[name="email"]')).not.toBeEmpty();
});

Then('user count stays the same', async function () {
  // TODO: Implement assertion
  // const count = await this.page.locator('table tbody tr').count();
  // expect(count).toBe(this.initialUserCount);
});
```

**Trade-offs:**
- ✅ **Pro:** Gherkin syntax is familiar to non-technical stakeholders
- ✅ **Pro:** Step definitions are reusable across scenarios
- ❌ **Con:** Extra layer of indirection — step strings must match exactly
- ❌ **Con:** Harder to debug — step definition failures have poor stack traces
- ❌ **Con:** Separate step definition files complicate navigation
- ❌ **Con:** Step definitions don't benefit from generated types as cleanly

**Recommendation:** Default to direct Playwright/Cypress code generation. Support `.feature` file output as an opt-in configuration flag:

```spec
spec "my-service" {
  gen typescript {
    e2e_style "playwright"    // Default: direct Playwright code
    // e2e_style "cucumber"   // Alternative: .feature files + step definitions
  }
}
```

---

## Structural Differences: `verify` vs. `scenario`

### `verify` Statements

| Aspect | Behavior |
|--------|----------|
| **Purpose** | Declare that a behavior needs a test of a specific type |
| **Attached to** | behavior, invariant, event, constraint |
| **Granularity** | Single test case |
| **Output** | One test function per `verify` statement |
| **Traceability** | Test function → behavior ID → spec file |
| **Implementation** | Developer fills in test body |
| **Framework** | Unit test frameworks (Vitest, pytest, cargo test) |

### `scenario` Blocks

| Aspect | Behavior |
|--------|----------|
| **Purpose** | Define a complete e2e test scenario with preconditions, actions, and assertions |
| **Attached to** | capability (product-level entity) |
| **Granularity** | Multi-step workflow |
| **Output** | One test per scenario, with step-by-step structure |
| **Traceability** | Test → scenario → capability → feature → behavior |
| **Implementation** | Developer fills in step implementations |
| **Framework** | E2E frameworks (Playwright, Cypress, Selenium) |

### When to Use Each

| Use Case | Syntax |
|----------|--------|
| Unit test for a behavior | `verify unit "..."` |
| Integration test with DB | `verify integration "..."` |
| Property-based invariant test | `verify property "..."` |
| Load test for a constraint | `verify load "..."` |
| Full user workflow (web UI) | `scenario { given when then }` |
| API acceptance test | `scenario { given when then }` (or `verify e2e`) |
| CLI smoke test | `scenario { given when then }` |

**Rule of thumb:**
- **`verify`** for code-level testing (unit, integration, property, load)
- **`scenario`** for user-level testing (e2e workflows across personas/surfaces)

---

## Traceability Link Maintenance

### Embedded Metadata

Every generated test file includes structured metadata in comments/annotations:

```typescript
/**
 * @specforge-behavior create_user
 * @specforge-file behaviors/user-crud.spec
 * @specforge-generated 2026-03-02T14:30:00Z
 */
```

### Runtime Registration

Test wrappers register the test with the behavior/capability ID:

```typescript
spec('create_user', () => { /* tests */ });     // Vitest
scenario('create_user_web', () => { /* tests */ }); // Playwright
```

### Coverage Report

The `@specforge/vitest` reporter (and equivalents) parse the test results and emit `specforge-report.json`:

```json
{
  "specforge": "1.0",
  "runner": "@specforge/vitest",
  "timestamp": "2026-03-02T14:30:00Z",
  "behaviors": {
    "create_user": {
      "tests": [
        {
          "name": "valid input creates user and returns ID",
          "file": "tests/behaviors/create_user.test.ts",
          "line": 35,
          "status": "pass",
          "duration_ms": 45,
          "type": "unit"
        },
        {
          "name": "created user is queryable by ID",
          "file": "tests/behaviors/create_user.test.ts",
          "line": 50,
          "status": "pass",
          "duration_ms": 120,
          "type": "integration"
        }
      ],
      "status": "covered"
    }
  },
  "capabilities": {
    "create_user_web": {
      "scenarios": [
        {
          "name": "admin creates user successfully",
          "file": "tests/capabilities/create_user_web.spec.ts",
          "line": 15,
          "status": "pass",
          "duration_ms": 2340,
          "steps": 4
        }
      ],
      "status": "covered"
    }
  }
}
```

### Bi-Directional Trace

The CLI can trace in both directions:

```bash
# Which tests cover this behavior?
specforge trace behavior create_user
# → tests/behaviors/create_user.test.ts:35 (unit)
# → tests/behaviors/create_user.test.ts:50 (integration)

# Which behaviors does this test cover?
specforge trace test tests/behaviors/create_user.test.ts:35
# → behavior: create_user (verify unit)

# Which capabilities test this feature?
specforge trace feature user_management
# → capability: create_user_web (scenario: "admin creates user successfully")
# → capability: create_user_api (scenario: "api creates user")
```

---

## Embedded Metadata and Comments

### File Header

Every generated test file should include:

```typescript
/**
 * Test suite for {entity_kind}: {entity_id}
 *
 * @specforge-{entity_kind} {entity_id}
 * @specforge-file {source_spec_file}
 * @specforge-generated {timestamp}
 *
 * {Contract or Flow text from spec}
 *
 * DO NOT EDIT the @specforge annotations above.
 * Implement the test bodies below.
 */
```

### Test-Level Annotations

Each test function includes:

```typescript
// @specforge-verify {type}
it('test description from verify statement', () => { /* ... */ });

// @specforge-step {given|when|then} "step text"
await given('step text', async () => { /* ... */ });
```

### Why Annotations Matter

1. **Tooling can extract metadata** without parsing the full test file
2. **IDEs can provide jump-to-spec navigation** via annotations
3. **Coverage reporters can parse annotations** to map tests to specs
4. **Regeneration can preserve hand-written test bodies** by matching annotations

### Regeneration Strategy

When a spec changes and tests are regenerated:

1. **Parse existing test file** — extract test bodies by `@specforge-verify` annotation
2. **Generate new skeleton** — with updated contract text, new verify statements
3. **Merge preserved test bodies** — match by verify type + description
4. **Flag orphaned tests** — tests that no longer have a matching `verify` statement

This allows iterative development: generate → implement → regenerate → merge.

---

## Configuration: Per-Language Test Generation

### specforge.spec

```spec
spec "my-service" {
  gen typescript {
    out              "src/generated/"
    result           "hex-di"
    tests            "@specforge/vitest"
    test_out         "tests/"
    test_framework   "vitest"           // vitest | jest | node:test
    e2e_framework    "playwright"       // playwright | cypress | cucumber
    e2e_style        "playwright"       // "playwright" (direct code) | "cucumber" (.feature files)
  }

  gen python {
    out              "src/generated/"
    result           "result"
    tests            "@specforge/pytest"
    test_out         "tests/"
    test_framework   "pytest"
    e2e_framework    "playwright-python"
  }

  gen rust {
    out              "src/generated/"
    test_out         "tests/"
    test_framework   "cargo-test"
    property_framework "proptest"        // proptest | quickcheck
  }
}
```

---

## Step Definition Stubs

### Scenario Block → Step Stubs

For direct Playwright/Cypress generation (default), steps are inlined:

```typescript
await given('admin is on user management page', async () => {
  // TODO: Implement step
});
```

For Cucumber generation (opt-in), separate step definition files are created:

```typescript
Given('admin is on user management page', async function () {
  // TODO: Implement step
});
```

### Step Reusability

**Direct generation approach:**
- Steps are **not reusable** across scenarios
- Each scenario is self-contained
- Trade-off: More duplication, but easier to understand and debug

**Cucumber approach:**
- Steps are **reusable** across scenarios and features
- Step definition matching is based on string patterns
- Trade-off: Less duplication, but harder to navigate and maintain

**Recommendation:** Default to direct generation. Most e2e tests are integration tests for specific workflows, not shared building blocks. If step reusability becomes critical, users can refactor to Cucumber-style or extract helper functions manually.

---

## Answers to Key Questions

### 1. Does `scenario` generate `.feature` files or code directly?

**Answer:** Direct code generation (Playwright/Cypress) by default. `.feature` file output is opt-in via `e2e_style "cucumber"` configuration.

**Rationale:** Simpler toolchain, better IDE integration, type-safe step implementations, easier debugging.

---

### 2. How do `verify` and `scenario` outputs differ structurally?

| Aspect | `verify` | `scenario` |
|--------|----------|-----------|
| **File location** | `tests/behaviors/{id}.test.ts` | `tests/capabilities/{id}.spec.ts` |
| **Test wrapper** | `spec('behavior_id', () => {})` | `scenario('capability_id', () => {})` |
| **Structure** | Flat list of tests, one per `verify` | Nested: one test per scenario, steps within |
| **Imports** | Generated types/ports | Generated types/ports + page objects |
| **Fixtures/Setup** | `beforeEach` for shared repository | `beforeEach` for preconditions (Given steps) |
| **Assertions** | Standard test assertions | Page/DOM assertions (Playwright/Cypress) |
| **Test type markers** | `@specforge-verify {type}` | `@specforge-step {given|when|then}` |

---

### 3. Does `scenario` generate step definition stubs?

**Answer:**

- **Direct generation (default):** Steps are inlined as anonymous functions with TODO comments
- **Cucumber generation (opt-in):** Separate step definition files are created with function stubs

**Example (direct):**
```typescript
await given('admin is on page', async () => {
  // TODO: Implement navigation
});
```

**Example (Cucumber):**
```typescript
Given('admin is on page', async function () {
  // TODO: Implement navigation
});
```

---

### 4. How is the traceability link maintained?

**Answer:** Three-layer approach:

1. **Metadata comments** — `@specforge-behavior`, `@specforge-file`, `@specforge-generated` in file header
2. **Runtime registration** — `spec('behavior_id')` wrapper registers tests with behavior ID
3. **Coverage reporting** — `@specforge/vitest` reporter emits `specforge-report.json` mapping test results to behavior IDs

**Bi-directional trace:**
- Spec → Test: `specforge trace behavior create_user` → lists test files
- Test → Spec: `specforge trace test tests/behaviors/create_user.test.ts:35` → lists behaviors

---

### 5. What metadata/comments should be embedded?

**File header:**
- Entity kind + ID (`@specforge-behavior create_user`)
- Source spec file (`@specforge-file behaviors/user-crud.spec`)
- Generation timestamp (`@specforge-generated 2026-03-02T14:30:00Z`)
- Contract/flow text (for reference)

**Test-level:**
- Verify type (`@specforge-verify unit`)
- Step type (`@specforge-step given`)
- TODO comments with implementation guidance

**Why:** Enables tooling (IDEs, coverage reporters, regeneration) to extract metadata without full parsing.

---

## Implementation Checklist

### Phase 1: `verify` Statement Generation
- [ ] TypeScript (Vitest) generator — `@specforge/gen-typescript`
- [ ] Python (pytest) generator — `@specforge/gen-python`
- [ ] Rust (cargo test) generator — `@specforge/gen-rust`
- [ ] File header metadata template
- [ ] Test-level `@specforge-verify` annotations
- [ ] TODO comments with guidance

### Phase 2: Test Runner Integration
- [ ] `@specforge/vitest` — `spec()` wrapper + reporter
- [ ] `@specforge/pytest` — `@spec` decorator + plugin
- [ ] `@specforge/rust` — `#[behavior]` attribute + test harness
- [ ] `specforge-report.json` schema
- [ ] Coverage merge logic in `specforge coverage`

### Phase 3: `scenario` Block Syntax (DSL)
- [ ] Grammar extension for `scenario` blocks
- [ ] Parser support for `given`/`when`/`then` statements
- [ ] AST representation
- [ ] Validation rules (scenario must be inside capability)

### Phase 4: `scenario` Direct Code Generation
- [ ] TypeScript (Playwright) generator
- [ ] TypeScript (Cypress) generator
- [ ] Python (Playwright) generator
- [ ] Inline step implementation stubs

### Phase 5: `scenario` Cucumber Generation (Opt-In)
- [ ] `.feature` file generator
- [ ] TypeScript Cucumber.js step definitions
- [ ] Python Behave step definitions
- [ ] Configuration flag `e2e_style "cucumber"`

### Phase 6: Traceability
- [ ] `specforge trace behavior <id>` — lists covering tests
- [ ] `specforge trace test <file:line>` — lists covered behaviors
- [ ] `specforge trace capability <id>` — lists scenarios
- [ ] IDE plugin with jump-to-spec navigation

---

## Recommendations

1. **Start with `verify` statements** — implement Phases 1-2 before tackling `scenario` blocks
2. **Default to direct code generation** — Playwright/Cypress output, not Cucumber
3. **Support Cucumber as opt-in** — for teams with existing Gherkin workflows
4. **Inline steps, not separate definitions** — simpler navigation, better type safety
5. **Embed rich metadata** — `@specforge-*` annotations enable tooling
6. **Support regeneration with merge** — preserve hand-written test bodies across regenerations

---

## Conclusion

**For `verify` statements:**
- Generate one test function per `verify` statement
- Wrap tests in `spec('behavior_id')` for traceability
- Include `@specforge-verify {type}` annotations
- Provide TODO comments with guidance
- Support unit, integration, property, load test types

**For `scenario` blocks:**
- Default to direct Playwright/Cypress code generation
- One test per scenario, steps inlined as anonymous functions
- Wrap tests in `scenario('capability_id')` for traceability
- Include `@specforge-step {given|when|then}` annotations
- Support `.feature` file output as opt-in via `e2e_style "cucumber"`

**Traceability:**
- Metadata comments in file headers
- Runtime registration wrappers
- Coverage reporting via `specforge-report.json`
- Bi-directional trace commands

This analysis provides the foundation for implementing test code generation across all supported languages and frameworks.
