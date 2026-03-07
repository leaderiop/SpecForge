# Expert 7: Formal Methods Integration into SpecForge Test Model

> [!CAUTION]
> **PARTIALLY SUPERSEDED by RES-27** — Formal methods syntax integrated into `@specforge/software`, not a separate plugin. `specforge gen` deprecated. The test model analysis and verify kind recommendations remain valid.

**Expert Role**: Testing & Verification Specialist
**Focus Areas**: Property-based testing, formal verification, model checking, test adequacy
**Date**: 2026-03-04
**Research Corpus**: Design by Contract (DbC), B-Method, CSP

---

## Executive Summary

SpecForge's current test model (RES-14, RES-15, RES-17) provides a solid three-layer traceability foundation (intent → linkage → proof). However, the formal methods research reveals **untapped verification power** that could transform SpecForge from "test coverage tracking" to "proof obligation management."

**Key Recommendation**: Evolve SpecForge's test model from **has-passing-test coverage** to **proof-obligation discharge** by integrating:

1. **DbC contracts** → automatic property-based test generation
2. **B-Method refinement** → test obligation matrices per abstraction level
3. **CSP trace semantics** → trace-based scenario generation

This shifts SpecForge's value proposition from "did you write tests?" to "can you prove correctness?"

---

## 1. Design by Contract → Property-Based Test Generation

### 1.1 Current State: Underspecified Contracts

**Current `behavior` contract field:**
```spec
behavior create_user "Create User" {
  contract """The system MUST create a user record with unique email."""
  verify unit "valid input creates user and returns ID"
}
```

**Problem**: The `contract` field is a prose description. Agents must **interpret** it. No machine-checkable specification.

### 1.2 DbC Integration: Structured Pre/Post/Invariant

**Proposed: Add contract block with DbC sections:**

```spec
behavior create_user "Create User" {
  contract {
    requires {
      "email is valid format"      check email_is_valid_format(input.email)
      "email not already in use"   check !user_exists_by_email(input.email)
      "name is non-empty"          check !input.name.is_empty()
    }

    ensures {
      "user record created"        check user_exists_by_id(result.user_id)
      "returned ID matches record" check db.get_user(result.user_id).email == input.email
      "user is queryable"          check get_user_by_id(result.user_id).is_ok()
    }

    invariants [unique_email, unique_user_ids]
  }

  // These are NOW GENERATED from the contract
  verify property "email uniqueness under concurrent creation"  // from invariants
  verify unit     "valid input creates user"                    // from ensures
  verify unit     "duplicate email rejected"                    // from requires negation
}
```

### 1.3 Automatic Test Generation Rules

| Contract Section | Generates Test Type | Test Strategy |
|-----------------|-------------------|---------------|
| **requires** | Unit tests (negative cases) | For each precondition, generate test that violates it + expects failure |
| **ensures** | Unit tests (positive cases) | For each postcondition, generate test that checks it holds |
| **invariants** | Property-based tests | Generate QuickCheck/Proptest generators from invariant predicate |
| **requires + ensures** | Integration tests | Full end-to-end path validation |

**Code Generation Target (Rust):**

```rust
// Generated from: requires "email not already in use"
#[test]
#[specforge::test("create_user")]
fn create_user__duplicate_email_rejected() {
    // Setup: create existing user
    let existing = create_user_unchecked("alice@example.com");

    // Act: attempt duplicate
    let result = create_user("alice@example.com", "Alice");

    // Assert: precondition violation detected
    assert!(matches!(result, Err(CreateUserError::DuplicateEmail)));
}

// Generated from: ensures "user record created"
#[test]
#[specforge::test("create_user")]
fn create_user__record_created() {
    let result = create_user("bob@example.com", "Bob").unwrap();

    // Postcondition: user exists
    assert!(user_exists_by_id(result.user_id));
}

// Generated from: invariants [unique_email]
#[proptest]
#[specforge::test("create_user")]
fn create_user__email_uniqueness_property(
    #[strategy(vec(arbitrary_email(), 1..100))] emails: Vec<String>
) {
    let mut created_ids = vec![];

    for email in emails {
        if let Ok(user) = create_user(&email, "Test") {
            created_ids.push(user.user_id);

            // Invariant check: no duplicate emails in DB
            let all_users = db.get_all_users();
            let email_counts: HashMap<_, usize> = all_users
                .iter()
                .map(|u| &u.email)
                .counts();

            prop_assert!(email_counts.values().all(|&count| count == 1));
        }
    }
}
```

### 1.4 Coverage Model Evolution: Proof Obligation Discharge

**Current coverage** (RES-15):
```
Entity          | Intent              | Test File                       | Status
────────────────|─────────────────────|─────────────────────────────────|────────
create_user     | 3 verify (u/u/i)    | tests/users/create-user.test.ts | ✅ 3/3 PASS
```

**Proposed coverage** (proof obligations):
```
Entity          | Preconditions | Postconditions | Invariants | Discharge Rate | Status
────────────────|---------------|----------------|------------|----------------|────────
create_user     | 3/3 ✅        | 3/3 ✅         | 2/2 ✅     | 100%           | PROVED
authenticate    | 2/2 ✅        | 1/3 ⚠️         | 1/1 ✅     | 83%            | PARTIAL
delete_user     | 0/2 ❌        | 0/3 ❌         | 0/1 ❌     | 0%             | UNPROVED
```

**Proof obligation discharge rate** = % of contract clauses with passing tests.

**New validation code:**
- **E017**: `behavior` has `contract` block but missing test for precondition clause
- **E018**: `behavior` has `contract` block but missing test for postcondition clause
- **W023**: `invariant` referenced in contract but no property-based test exists

---

## 2. B-Method Refinement → Test Obligation Matrices

### 2.1 Current State: Flat Entity Model

SpecForge entities are flat — a `behavior` is a `behavior`. No notion of **abstraction levels** or **refinement steps**.

### 2.2 B-Method Insight: Refinement = Testability Layers

B-Method's core workflow:
1. **Abstract machine** — high-level specification (what, not how)
2. **First refinement** — decompose into suboperations
3. **Second refinement** — introduce data structures
4. **Implementation** — concrete code

Each refinement step generates **proof obligations** (POG = Proof Obligation Generator):
- Invariant preservation
- Precondition strengthening
- Operation refinement correctness

### 2.3 Proposed: `refines` Field for Hierarchical Behaviors

```spec
behavior authenticate_abstract "Authenticate User (Abstract)" {
  contract {
    requires {
      "credentials provided" check !credentials.is_empty()
    }
    ensures {
      "session created if valid" check result.is_ok() => session_exists(result.session_id)
    }
  }

  // Abstract — no verify/tests, only contract
  abstract true
}

behavior authenticate_with_password "Authenticate with Password" {
  refines authenticate_abstract

  contract {
    requires {
      "email and password provided" check credentials.email.is_some() && credentials.password.is_some()
      "user exists"                  check user_exists_by_email(credentials.email)
    }
    ensures {
      "password hash checked"        check verify_password_hash(credentials.password, user.password_hash)
      "session token generated"      check result.session_id.len() == 64
    }
  }

  verify unit "valid credentials create session"
  verify unit "invalid password rejected"

  tests ["tests/auth/password.test.ts"]
}

behavior authenticate_with_oauth "Authenticate with OAuth" {
  refines authenticate_abstract

  contract {
    requires {
      "oauth token provided"  check credentials.oauth_token.is_some()
      "provider is supported" check SUPPORTED_PROVIDERS.contains(credentials.provider)
    }
    ensures {
      "token validated with provider" check oauth_client.verify(credentials.oauth_token)
      "session linked to provider"    check result.provider == credentials.provider
    }
  }

  verify integration "oauth provider validates token"
  verify unit        "unsupported provider rejected"

  tests ["tests/auth/oauth.test.ts"]
}
```

### 2.4 Test Obligation Matrix

| Behavior | Abstraction Level | Preconditions | Postconditions | Refines POs | Status |
|----------|------------------|---------------|----------------|-------------|--------|
| `authenticate_abstract` | L0 (abstract) | 1 | 1 | — | Abstract (no tests) |
| `authenticate_with_password` | L1 (concrete) | 2 | 2 | 4 (L0→L1) | 100% |
| `authenticate_with_oauth` | L1 (concrete) | 2 | 2 | 4 (L0→L1) | 100% |

**Refinement proof obligations** (auto-generated):
1. **Precondition weakening**: concrete preconditions MUST imply abstract preconditions
2. **Postcondition strengthening**: concrete postconditions MUST imply abstract postconditions
3. **Coverage**: ALL abstract behaviors MUST have at least one concrete refinement
4. **Consistency**: Concrete `verify` tests MUST cover abstract contract

**New verify kind:**
```spec
verify refinement "password authentication refines abstract contract"
```

This generates a test that:
- Runs the concrete implementation (`authenticate_with_password`)
- Asserts that the abstract contract (`authenticate_abstract`) is satisfied

### 2.5 Validation Rules for Refinement

| Code | Condition | Message |
|------|-----------|---------|
| **E019** | Concrete behavior refines abstract but violates its precondition | `"'{concrete_id}' refines '{abstract_id}' but precondition '{clause}' not implied"` |
| **E020** | Concrete behavior refines abstract but violates its postcondition | `"'{concrete_id}' refines '{abstract_id}' but postcondition '{clause}' not satisfied"` |
| **W024** | Abstract behavior has no concrete refinements | `"abstract behavior '{id}' has no refinements"` |
| **I007** | Behavior marked abstract but has verify statements | `"abstract behavior '{id}' has verify statements (should only have contract)"` |

---

## 3. CSP Trace Semantics → Trace-Based Test Generation

### 3.1 Current State: Scenarios Are Prose

**Current scenario block:**
```spec
scenario "admin creates user successfully" {
  given "admin is on user management page"
  when  "admin fills name, email, role and clicks Create"
  then  "success toast appears"
  then  "new user appears in user list"
}
```

**Problem**: Steps are English strings. No formal trace semantics. Cannot generate CSP models for model checking.

### 3.2 CSP Insight: Traces = Valid Event Sequences

CSP models concurrency as **processes** that communicate via **events**. A **trace** is a sequence of events a process can engage in.

**CSP notation:**
```csp
CREATE_USER = navigate?admin?user_page
            -> fill?name -> fill?email -> fill?role
            -> click?create_button
            -> (toast!success -> user_list!shows_user -> STOP
                []
                toast!error -> STOP)
```

CSP tools (FDR4, PAT) can:
- **Traces refinement check**: Does implementation accept only valid traces?
- **Failures refinement check**: Does implementation refuse invalid events?
- **Deadlock freedom check**: Can the system always make progress?

### 3.3 Proposed: Structured Scenario Steps with Event Tags

**Enhanced scenario syntax:**

```spec
capability create_user_ux "Create User Flow" {
  events [page_navigated, form_filled, button_clicked, toast_shown, user_listed]

  scenario "admin creates user successfully" {
    given "admin logged in"                event auth_completed(persona: admin)
    given "on user management page"        event page_navigated(path: "/admin/users")

    when  "fills name field"               event form_filled(field: "name", value: "Alice")
    when  "fills email field"              event form_filled(field: "email", value: "alice@example.com")
    when  "clicks Create button"           event button_clicked(id: "create-user")

    then  "success toast appears"          event toast_shown(type: success, message: "User created")
    then  "new user in list"               event user_listed(name: "Alice")
  }

  scenario "duplicate email rejection" {
    given "user alice@example.com exists" event user_exists(email: "alice@example.com")
    given "on user management page"       event page_navigated(path: "/admin/users")

    when  "fills email alice@example.com" event form_filled(field: "email", value: "alice@example.com")
    when  "clicks Create button"          event button_clicked(id: "create-user")

    then  "error toast appears"           event toast_shown(type: error, message: "Email already in use")
    then  "user count unchanged"          event user_count_stable()
  }

  // CSP trace refinement test auto-generated
  verify trace "all scenarios are valid CSP traces"

  // Deadlock freedom auto-generated
  verify deadlock_free "no scenario can deadlock"
}
```

### 3.4 Auto-Generated CSP Models

**From the above scenarios, generate:**

```csp
-- Process: CREATE_USER_UX
CREATE_USER_SUCCESS =
    auth_completed.admin ->
    page_navigated./admin/users ->
    form_filled.name.Alice ->
    form_filled.email.alice@example.com ->
    button_clicked.create-user ->
    toast_shown.success ->
    user_listed.Alice -> STOP

CREATE_USER_DUPLICATE =
    user_exists.alice@example.com ->
    page_navigated./admin/users ->
    form_filled.email.alice@example.com ->
    button_clicked.create-user ->
    toast_shown.error ->
    user_count_stable -> STOP

CREATE_USER_UX = CREATE_USER_SUCCESS [] CREATE_USER_DUPLICATE

-- Assertions
assert CREATE_USER_UX :[deadlock free]
assert CREATE_USER_UX :[divergence free]
```

**Code generation target:**

```typescript
// Generated trace-based test
test.describe('create_user_ux CSP traces', () => {
  test('trace: admin creates user successfully', async ({ page }) => {
    // Trace step 1: auth_completed(persona: admin)
    await login(page, 'admin');

    // Trace step 2: page_navigated(path: "/admin/users")
    await page.goto('/admin/users');
    expect(page.url()).toContain('/admin/users');

    // Trace step 3: form_filled(field: "name", value: "Alice")
    await page.fill('[name="name"]', 'Alice');

    // Trace step 4: form_filled(field: "email", value: "alice@example.com")
    await page.fill('[name="email"]', 'alice@example.com');

    // Trace step 5: button_clicked(id: "create-user")
    await page.click('#create-user');

    // Trace step 6: toast_shown(type: success, message: "User created")
    await expect(page.locator('.toast.success')).toContainText('User created');

    // Trace step 7: user_listed(name: "Alice")
    await expect(page.locator('table')).toContainText('Alice');
  });

  test('invalid trace: form_filled before page_navigated (should fail)', async ({ page }) => {
    // This test asserts that INVALID traces are rejected
    await login(page, 'admin');

    // Attempt to fill form before navigating (violates CSP trace)
    await expect(async () => {
      await page.fill('[name="name"]', 'Alice');
    }).rejects.toThrow(); // Element not found — trace violation detected
  });
});
```

### 3.5 New Verify Kinds for Trace-Based Testing

| Verify Kind | Purpose | Generated Test |
|------------|---------|----------------|
| `verify trace` | Valid trace refinement | Run all scenarios as CSP traces, assert each completes |
| `verify deadlock_free` | Deadlock freedom | Generate negative tests for all partial traces that should NOT deadlock |
| `verify liveness` | Progress property | Assert that certain events eventually occur (e.g., "toast MUST appear within 5s") |

**Example:**
```spec
verify trace "all scenarios are valid CSP traces"
verify deadlock_free "form submission never deadlocks"
verify liveness "toast appears within 5 seconds"
```

---

## 4. Mutation Testing Integration: Contracts Define Kill Criteria

### 4.1 Current State: No Mutation Testing

SpecForge tracks "tests exist and pass" but not **test quality**. A test suite can have 100% coverage yet fail to catch bugs.

### 4.2 Formal Methods Insight: Contracts = Expected Mutations

**DbC contracts tell you what mutations SHOULD be caught.**

Example behavior:
```spec
behavior withdraw_funds "Withdraw Funds" {
  contract {
    requires {
      "account exists"            check account_exists(account_id)
      "sufficient balance"        check balance(account_id) >= amount
      "amount is positive"        check amount > 0
    }
    ensures {
      "balance decreased"         check balance(account_id) == old(balance) - amount
      "transaction recorded"      check transaction_log.contains(tx_id)
    }
  }
}
```

### 4.3 Auto-Generated Mutation Kill Matrix

| Mutation | Expected Killing Test | Contract Clause | Status |
|----------|----------------------|-----------------|--------|
| Remove `balance >= amount` check | Negative test: overdraft attempt | requires "sufficient balance" | ✅ KILLED |
| Change `-` to `+` in balance update | Positive test: balance decreased | ensures "balance decreased" | ✅ KILLED |
| Remove `amount > 0` check | Negative test: zero amount | requires "amount is positive" | ✅ KILLED |
| Remove transaction logging | Integration test: tx in log | ensures "transaction recorded" | ❌ SURVIVED |

**Mutation coverage** = % of contract-derived mutants killed by test suite.

### 4.4 Integration with Rust Mutation Testing Tools

**Tools**: `cargo-mutants`, `mutagen`

**Proposed workflow:**
1. ~~`specforge gen rust --contract-mutations`~~ [DEPRECATED — `specforge gen` removed; mutation targets would come from extension contributions] → generates mutation targets from contracts
2. `cargo mutants --file mutations.json` → runs mutations
3. `specforge trace --mutation-results mutants-out.json` → reports which contract clauses lack killing tests

**New coverage metric:**
```
Entity          | Proof Obligations | Discharge Rate | Mutation Coverage | Status
────────────────|-------------------|----------------|-------------------|────────
withdraw_funds  | 5/5 ✅            | 100%           | 3/4 (75%)         | ⚠️ WEAK TESTS
```

---

## 5. New Verify Kinds: Summary

| Verify Kind | What It Tests | Generated By | Example |
|------------|---------------|--------------|---------|
| `verify contract` | Preconditions + postconditions | DbC contract block | Property tests for each clause |
| `verify refinement` | Abstract → concrete correctness | `refines` field | Concrete impl satisfies abstract contract |
| `verify trace` | Valid event sequences | CSP-enhanced scenarios | All scenario paths are valid traces |
| `verify deadlock_free` | Progress guarantee | CSP model checking | No partial trace can deadlock |
| `verify liveness` | Eventual outcomes | CSP temporal properties | "Success event occurs within N seconds" |
| `verify mutation` | Test suite quality | Contract-derived mutations | Mutants for each contract clause are killed |

**Backward compatibility**: Existing `verify unit/integration/property/load/e2e` remain. New kinds are opt-in.

---

## 6. Coverage Model Evolution: From "Has Test" to "Proved Correct"

### 6.1 Current Coverage Levels (RES-15)

| Level | Condition | Signal |
|-------|-----------|--------|
| Declared | Has `verify`/`scenario` | Entity has test intent |
| Linked | Has `tests` field | Connected to real tests |
| Executed | Test result in report | Tests have been run |
| Passing | All tests pass | Full traceability achieved |

**Problem**: "Passing" means "tests don't crash." Does NOT mean "spec is implemented correctly."

### 6.2 Proposed: Proof Obligation Coverage

| Level | Condition | Signal |
|-------|-----------|--------|
| **Specified** | Has `contract` block | Formal spec exists |
| **Obligations Defined** | Proof obligations extracted | Preconditions, postconditions, invariants counted |
| **Obligations Discharged** | Each PO has passing test | All clauses tested |
| **Refinement Checked** | If `refines` used, refinement tests pass | Abstraction layers consistent |
| **Mutation Resistant** | Contract-derived mutants killed | Tests are strong |
| **Model Checked** | CSP traces validated (if applicable) | Concurrency correct |

**Final verdict:**
- **UNPROVED**: No contract block
- **PARTIAL**: Some POs discharged (50-99%)
- **PROVED**: All POs discharged (100%)
- **STRONG**: PROVED + mutation coverage ≥ 80%
- **VERIFIED**: STRONG + model checking pass

### 6.3 New `specforge trace` Output

```bash
$ specforge trace --proof-obligations
```

```
Entity              | POs   | Discharge | Mutation | Model Check | Verdict
────────────────────|-------|-----------|----------|-------------|──────────
create_user         | 8/8   | 100%      | 7/8 88%  | —           | STRONG
authenticate        | 6/6   | 100%      | 6/6 100% | ✅ PASS     | VERIFIED
withdraw_funds      | 5/5   | 100%      | 3/5 60%  | —           | PROVED (weak tests)
delete_user         | 4/7   | 57%       | —        | —           | PARTIAL
import_csv          | 0/0   | —         | —        | —           | UNPROVED (no contract)

Overall: 23/26 proof obligations discharged (88%)
```

---

## 7. Implementation Roadmap

### Phase 1: DbC Contracts (3-4 weeks)
- [ ] Add `contract { requires/ensures/invariants }` block to grammar
- [ ] Implement proof obligation extraction
- [ ] Generate unit tests from preconditions (negative cases)
- [ ] Generate unit tests from postconditions (positive cases)
- [ ] Add validation: E017 (missing precondition test), E018 (missing postcondition test)
- [ ] Update `specforge trace` to show PO discharge rate

### Phase 2: Property-Based Test Generation (2-3 weeks)
- [ ] Integrate with Proptest (Rust), fast-check (TypeScript), Hypothesis (Python)
- [ ] Generate property test from invariant references in contract block
- [ ] Add `verify contract` kind
- [ ] Update Rust plugin to recognize property tests

### Phase 3: Refinement Testing (3-4 weeks)
- [ ] Add `refines` field to behavior grammar
- [ ] Add `abstract` bool field
- [ ] Implement refinement PO generation (precondition weakening, postcondition strengthening)
- [ ] Add `verify refinement` kind
- [ ] Validation: E019/E020 (refinement violations), W024 (abstract with no refinements)

### Phase 4: CSP Trace Generation (4-5 weeks)
- [ ] Extend scenario steps with `event` tags
- [ ] Add `events` field to capability/behavior
- [ ] Generate CSP models from scenarios
- [ ] Integrate with FDR4 or PAT for model checking
- [ ] Add `verify trace`, `verify deadlock_free`, `verify liveness` kinds
- [ ] Generate trace-based tests (TypeScript/Playwright, Rust/integration tests)

### Phase 5: Mutation Testing (2-3 weeks)
- [ ] Generate mutation targets from contract blocks
- [ ] Integrate with `cargo-mutants` (Rust), Stryker (TypeScript)
- [ ] Add mutation coverage to `specforge trace`
- [ ] Add `verify mutation` kind

### Phase 6: Model Checking Integration (3-4 weeks)
- [ ] Optional CSP model checking in CI via `--model-check` flag
- [ ] FDR4/PAT integration for deadlock/livelock/trace refinement
- [ ] Generate model checking report in `specforge-report.json`
- [ ] Update coverage verdicts: STRONG → VERIFIED

**Total: 18-23 weeks (4.5-5.5 months)**

---

## 8. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **Complexity creep** | High | High | Make all formal methods features **opt-in**. Default behavior unchanged. |
| **Steep learning curve** | High | Medium | Provide templates, examples, agent guidance. Formal specs are for advanced users. |
| **Tool dependency** | Medium | Medium | CSP/FDR4 model checking is optional. Core features work without it. |
| **False positives** | Medium | High | Proof obligation extraction must be conservative. Better to under-extract than over-extract. |
| **Performance** | Low | Medium | Proof obligation checking runs post-compile. Does not slow down watch mode. |

---

## 9. Competitive Differentiation

### SpecForge After Formal Methods Integration

| Dimension | SpecForge 1.0 (current) | SpecForge 2.0 (formal methods) | Dafny | TLA+ | Alloy |
|-----------|-------------------------|--------------------------------|-------|------|-------|
| **Primary use case** | AI agent specs | Proof-carrying specs | Verified programs | Distributed systems | Model finding |
| **Test generation** | Scaffolds only | Full property-based + trace | N/A (verification, not testing) | N/A | N/A |
| **Proof obligations** | Manual | Auto-extracted from contracts | Auto-extracted | Manual | Manual |
| **Refinement tracking** | ❌ | ✅ B-Method inspired | ✅ (via refinement types) | ✅ (via TLA+ hierarchies) | ❌ |
| **Mutation testing** | ❌ | ✅ Contract-derived | ❌ | ❌ | ❌ |
| **Model checking** | ❌ | ✅ CSP trace validation | ❌ | ✅ TLC | ✅ SAT solver |
| **Learning curve** | Low | Medium | Very high | Very high | High |
| **Integration** | Any language | Any language | Dafny only | Separate spec | Separate spec |
| **Agent-first** | ✅ | ✅ | ❌ | ❌ | ❌ |

**Unique position**: SpecForge becomes the **only agent-first specification language with formal verification integration**.

---

## 10. Conclusion

SpecForge's test model is currently **coverage-focused** ("Did you write tests?"). Formal methods research reveals a path to **correctness-focused** testing ("Can you prove it works?").

**Key transformations:**

1. **Contracts become executable** — `contract { requires/ensures }` generates tests, not just documentation
2. **Coverage becomes proof** — "100% coverage" means "all proof obligations discharged"
3. **Refinement becomes first-class** — abstract behaviors + concrete refinements track design intent
4. **Scenarios become traces** — CSP semantics enable model checking + trace-based testing
5. **Mutation testing closes the loop** — contracts define what mutants SHOULD be killed

**Recommended priority:**
1. **Phase 1 (DbC contracts)** — highest ROI, lowest risk, immediate value
2. **Phase 2 (property tests)** — natural extension of Phase 1
3. **Phase 3 (refinement)** — architectural value for complex systems
4. **Phase 4-6 (CSP/mutation)** — advanced features, opt-in

**Vision**: SpecForge 2.0 positions itself as **"the proof-carrying specification language for AI agents"** — bridging the gap between lightweight specs (OpenAPI, GraphQL schemas) and heavyweight verification (Dafny, TLA+).

---

## Appendix A: Example Before/After

### Before (SpecForge 1.0)

```spec
behavior create_user "Create User" {
  contract """The system MUST create a user record with unique email."""

  verify unit "valid input creates user"
  verify unit "duplicate email rejected"

  tests ["tests/users/create_user.test.rs"]
}
```

**Coverage report:**
```
Entity      | Intent     | Tests                          | Status
────────────|────────────|────────────────────────────────|────────
create_user | 2 verify   | tests/users/create_user.test.rs| ✅ 2/2 PASS
```

### After (SpecForge 2.0 with Formal Methods)

```spec
behavior create_user "Create User" {
  contract {
    requires {
      "email is valid format"      check email_is_valid_format(input.email)
      "email not already in use"   check !user_exists_by_email(input.email)
      "name is non-empty"          check !input.name.is_empty()
    }

    ensures {
      "user record created"        check user_exists_by_id(result.user_id)
      "returned ID matches"        check db.get_user(result.user_id).email == input.email
      "email is queryable"         check get_user_by_email(input.email).is_ok()
    }

    invariants [unique_email_invariant]
  }

  // Auto-generated from contract:
  verify contract "preconditions and postconditions"
  verify property "email uniqueness under concurrent creation"
  verify mutation "contract-derived mutants killed"

  tests ["tests/users/create_user.test.rs"]
}
```

**Coverage report:**
```
Entity      | POs   | Discharge | Mutation | Verdict
────────────|-------|-----------|----------|──────────
create_user | 7/7   | 100%      | 6/7 86%  | STRONG

Proof Obligations:
  ✅ Precondition: email is valid format          (test: invalid_email_rejected)
  ✅ Precondition: email not already in use       (test: duplicate_email_rejected)
  ✅ Precondition: name is non-empty              (test: empty_name_rejected)
  ✅ Postcondition: user record created           (test: user_created)
  ✅ Postcondition: returned ID matches           (test: returned_id_correct)
  ✅ Postcondition: email is queryable            (test: user_queryable)
  ✅ Invariant: unique_email_invariant            (test: email_uniqueness_property)

Mutation Coverage:
  ✅ KILLED: Remove email format check
  ✅ KILLED: Remove duplicate email check
  ✅ KILLED: Remove name empty check
  ✅ KILLED: Skip user record creation
  ✅ KILLED: Return wrong user ID
  ❌ SURVIVED: Skip queryability assertion          ← weak test detected
  ✅ KILLED: Violate email uniqueness
```

**The difference**: SpecForge 2.0 doesn't just track "tests exist" — it **proves correctness** via proof obligation discharge and mutation resistance.
