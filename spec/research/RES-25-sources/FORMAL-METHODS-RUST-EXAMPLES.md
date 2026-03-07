# Formal Methods: Rust Implementation Examples

> [!CAUTION]
> **PARTIALLY SUPERSEDED** — "code generation" framing outdated; `specforge gen` deprecated. Formal methods syntax integrated into `@specforge/software` (RES-27). The Rust test patterns remain valid as reference for external tools.

**Context**: Concrete code generation examples for SpecForge Rust plugin with formal methods integration
**Source**: EXPERT-7-FORMAL-METHODS-TEST-MODEL.md
**Date**: 2026-03-04

---

## Example 1: DbC Contract → Property-Based Tests

### Spec File
```spec
// File: spec/user-management.spec
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
      "email is queryable"         check get_user_by_email(input.email).is_ok()
    }

    invariants [unique_email_invariant]
  }

  verify contract "preconditions and postconditions"
  verify property "email uniqueness under concurrent creation"

  tests ["tests/users/create_user_test.rs"]
}

invariant unique_email_invariant "Unique Email Addresses" {
  guarantee """No two active users SHALL share the same email address."""
  enforced_by [create_user, update_user_email]
}
```

### Generated Rust Tests

```rust
// File: tests/users/create_user_test.rs
// @specforge-behavior create_user
// @specforge-file spec/user-management.spec
// @specforge-generated

use specforge_test::prelude::*;
use proptest::prelude::*;
use your_app::{create_user, user_exists_by_email, get_user_by_id};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PRECONDITION TESTS (Negative Cases)
// Generated from: contract.requires
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
#[specforge::test("create_user")]
fn create_user__precond_invalid_email_format() {
    // Precondition: "email is valid format"
    // Test: violate this precondition → expect error

    let invalid_emails = vec![
        "not-an-email",
        "missing@domain",
        "@no-local.com",
        "spaces in@email.com",
        "",
    ];

    for email in invalid_emails {
        let result = create_user(email, "Test User");

        // Assert: precondition violation detected
        assert!(
            result.is_err(),
            "Invalid email '{}' should be rejected",
            email
        );
        assert!(matches!(
            result.unwrap_err(),
            CreateUserError::InvalidEmail
        ));
    }
}

#[test]
#[specforge::test("create_user")]
fn create_user__precond_email_already_exists() {
    // Precondition: "email not already in use"
    // Test: violate this precondition → expect error

    let db = TestDatabase::new();

    // Setup: create existing user
    let existing = db.insert_user("alice@example.com", "Alice");
    assert!(existing.is_ok());

    // Act: attempt to create duplicate
    let result = create_user("alice@example.com", "Alice Clone");

    // Assert: precondition violation detected
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CreateUserError::DuplicateEmail
    ));
}

#[test]
#[specforge::test("create_user")]
fn create_user__precond_name_is_empty() {
    // Precondition: "name is non-empty"
    // Test: violate this precondition → expect error

    let result = create_user("test@example.com", "");

    // Assert: precondition violation detected
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CreateUserError::EmptyName
    ));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// POSTCONDITION TESTS (Positive Cases)
// Generated from: contract.ensures
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
#[specforge::test("create_user")]
fn create_user__postcond_user_record_created() {
    // Postcondition: "user record created"
    // Test: after successful creation, user exists in DB

    let db = TestDatabase::new();
    let result = create_user("bob@example.com", "Bob").unwrap();

    // Assert: postcondition holds
    assert!(db.user_exists_by_id(result.user_id));
}

#[test]
#[specforge::test("create_user")]
fn create_user__postcond_returned_id_matches_record() {
    // Postcondition: "returned ID matches record"
    // Test: returned user ID points to correct record

    let db = TestDatabase::new();
    let input_email = "charlie@example.com";
    let result = create_user(input_email, "Charlie").unwrap();

    // Assert: postcondition holds
    let stored_user = db.get_user(result.user_id).unwrap();
    assert_eq!(stored_user.email, input_email);
}

#[test]
#[specforge::test("create_user")]
fn create_user__postcond_email_is_queryable() {
    // Postcondition: "email is queryable"
    // Test: created user can be retrieved by email

    let db = TestDatabase::new();
    let input_email = "dana@example.com";
    let result = create_user(input_email, "Dana").unwrap();

    // Assert: postcondition holds
    let query_result = db.get_user_by_email(input_email);
    assert!(query_result.is_ok());
    assert_eq!(query_result.unwrap().user_id, result.user_id);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// INVARIANT TESTS (Property-Based)
// Generated from: contract.invariants
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

proptest! {
    #[test]
    #[specforge::test("create_user")]
    fn create_user__invariant_unique_email_property(
        emails in prop::collection::vec(
            "[a-z]{3,10}@[a-z]{3,8}\\.com",
            1..50
        )
    ) {
        // Invariant: unique_email_invariant
        // Property: No two users share the same email

        let db = TestDatabase::new();
        let mut created_users = vec![];

        for email in emails.iter() {
            match create_user(email, "Test User") {
                Ok(user) => created_users.push(user),
                Err(CreateUserError::DuplicateEmail) => {
                    // Expected: duplicate rejected
                    continue;
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        // Assert invariant: all emails in DB are unique
        let all_users = db.get_all_users();
        let mut seen_emails = std::collections::HashSet::new();

        for user in all_users {
            prop_assert!(
                seen_emails.insert(&user.email),
                "Duplicate email found: {}",
                user.email
            );
        }
    }
}

proptest! {
    #[test]
    #[specforge::test("create_user")]
    fn create_user__invariant_concurrent_uniqueness(
        emails in prop::collection::vec(
            "[a-z]{5,10}@test\\.com",
            10..50
        )
    ) {
        // Property: Email uniqueness under concurrent creation
        // Simulate concurrent create_user calls with same emails

        use std::sync::{Arc, Mutex};
        use std::thread;

        let db = Arc::new(Mutex::new(TestDatabase::new()));
        let email = "concurrent@test.com";
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let db_clone = Arc::clone(&db);
                let email = email.to_string();
                thread::spawn(move || {
                    create_user(&email, &format!("User {}", i))
                })
            })
            .collect();

        let results: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        // Assert: exactly ONE create_user succeeded
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        prop_assert_eq!(success_count, 1, "Only one concurrent create should succeed");

        // Assert: invariant still holds in DB
        let db = db.lock().unwrap();
        let all_users = db.get_all_users();
        let email_counts: HashMap<_, usize> = all_users
            .iter()
            .map(|u| &u.email)
            .counts();

        for (email, count) in email_counts {
            prop_assert_eq!(count, 1, "Email {} appears {} times", email, count);
        }
    }
}
```

---

## Example 2: Refinement Testing

### Spec File
```spec
// File: spec/authentication.spec

behavior authenticate_abstract "Authenticate User (Abstract)" {
  contract {
    requires {
      "credentials provided" check !credentials.is_empty()
    }
    ensures {
      "session created if valid" check result.is_ok() => session_exists(result.session_id)
      "error if invalid"         check result.is_err() => !session_exists(credentials)
    }
  }

  abstract true  // No tests — abstract spec only
}

behavior authenticate_with_password "Authenticate with Password" {
  refines authenticate_abstract

  contract {
    requires {
      "email provided"       check credentials.email.is_some()
      "password provided"    check credentials.password.is_some()
      "user exists"          check user_exists_by_email(credentials.email)
    }
    ensures {
      "password verified"    check result.is_ok() => verify_password(credentials.password, user.hash)
      "session token valid"  check result.is_ok() => result.session_id.len() == 64
    }
  }

  verify contract "preconditions and postconditions"
  verify refinement "satisfies authenticate_abstract contract"

  tests ["tests/auth/password_test.rs"]
}
```

### Generated Rust Tests

```rust
// File: tests/auth/password_test.rs
// @specforge-behavior authenticate_with_password
// @specforge-refines authenticate_abstract

use specforge_test::prelude::*;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// REFINEMENT TEST
// Ensures concrete behavior satisfies abstract contract
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
#[specforge::test("authenticate_with_password")]
fn authenticate_with_password__refines_abstract() {
    // Refinement PO: Concrete must satisfy abstract contract
    //
    // Abstract requires: credentials provided
    // Concrete requires: email + password + user exists
    // → Concrete preconditions IMPLY abstract (stronger = OK)
    //
    // Abstract ensures: session created if valid
    // Concrete ensures: password verified + session token valid
    // → Concrete postconditions IMPLY abstract (stronger = OK)

    let db = TestDatabase::new();
    let user = db.insert_user("test@example.com", "Test User");
    db.set_password_hash(&user.user_id, "hashed_password_123");

    // Test: valid credentials (satisfies both abstract and concrete)
    let creds = PasswordCredentials {
        email: "test@example.com".to_string(),
        password: "correct_password".to_string(),
    };

    let result = authenticate_with_password(&creds).unwrap();

    // Assert: ABSTRACT postcondition holds
    assert!(db.session_exists(result.session_id));

    // Assert: CONCRETE postconditions hold
    assert_eq!(result.session_id.len(), 64);
    assert!(db.verify_password(&creds.password, &user.password_hash));
}

#[test]
#[specforge::test("authenticate_with_password")]
fn authenticate_with_password__refines_abstract_failure() {
    // Refinement PO: Failure cases also satisfy abstract contract

    let db = TestDatabase::new();
    db.insert_user("test@example.com", "Test User");

    // Test: invalid password
    let creds = PasswordCredentials {
        email: "test@example.com".to_string(),
        password: "wrong_password".to_string(),
    };

    let result = authenticate_with_password(&creds);

    // Assert: ABSTRACT postcondition holds (error if invalid)
    assert!(result.is_err());
    assert!(!db.session_exists_for_email(&creds.email));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// STANDARD CONTRACT TESTS (preconditions/postconditions)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
#[specforge::test("authenticate_with_password")]
fn authenticate_with_password__precond_email_missing() {
    let creds = PasswordCredentials {
        email: String::new(),
        password: "password".to_string(),
    };

    let result = authenticate_with_password(&creds);
    assert!(matches!(result.unwrap_err(), AuthError::MissingEmail));
}

#[test]
#[specforge::test("authenticate_with_password")]
fn authenticate_with_password__precond_user_not_exists() {
    let creds = PasswordCredentials {
        email: "nonexistent@example.com".to_string(),
        password: "password".to_string(),
    };

    let result = authenticate_with_password(&creds);
    assert!(matches!(result.unwrap_err(), AuthError::UserNotFound));
}

#[test]
#[specforge::test("authenticate_with_password")]
fn authenticate_with_password__postcond_session_token_valid() {
    let db = TestDatabase::new();
    db.insert_user_with_password("test@example.com", "Test", "correct_password");

    let creds = PasswordCredentials {
        email: "test@example.com".to_string(),
        password: "correct_password".to_string(),
    };

    let result = authenticate_with_password(&creds).unwrap();

    // Postcondition: session token is 64 chars
    assert_eq!(result.session_id.len(), 64);
    assert!(result.session_id.chars().all(|c| c.is_ascii_alphanumeric()));
}
```

---

## Example 3: CSP Trace-Based Testing

### Spec File
```spec
// File: spec/user-flows.spec

capability create_user_ux "Create User Flow" {
  persona  admin
  surface  [web]
  events   [page_navigated, form_filled, button_clicked, toast_shown, user_listed]

  scenario "admin creates user successfully" {
    given "admin logged in"              event auth_completed(persona: admin)
    given "on user management page"      event page_navigated(path: "/admin/users")

    when  "fills name field"             event form_filled(field: "name", value: "Alice")
    when  "fills email field"            event form_filled(field: "email", value: "alice@test.com")
    when  "clicks Create button"         event button_clicked(id: "create-user-btn")

    then  "success toast appears"        event toast_shown(type: success, message: "User created")
    then  "new user appears in list"     event user_listed(name: "Alice", email: "alice@test.com")
  }

  scenario "duplicate email rejection" {
    given "user alice@test.com exists"   event user_exists(email: "alice@test.com")
    given "on user management page"      event page_navigated(path: "/admin/users")

    when  "fills duplicate email"        event form_filled(field: "email", value: "alice@test.com")
    when  "clicks Create button"         event button_clicked(id: "create-user-btn")

    then  "error toast appears"          event toast_shown(type: error, message: "Email already in use")
    then  "user count unchanged"         event user_count_stable()
  }

  verify trace "all scenarios are valid CSP traces"
  verify deadlock_free "no scenario can deadlock"

  tests ["tests/e2e/create_user_ux_test.rs"]
}
```

### Generated CSP Model

```csp
-- File: generated/create_user_ux.csp
-- @specforge-capability create_user_ux
-- @specforge-generated

channel auth_completed: {admin, user}
channel page_navigated: {"/admin/users", "/profile", "/settings"}
channel form_filled: {(field: String, value: String)}
channel button_clicked: {String}
channel toast_shown: {(type: {success, error, warning}, message: String)}
channel user_listed: {(name: String, email: String)}
channel user_exists: {(email: String)}
channel user_count_stable

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- SCENARIO TRACES
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

CREATE_USER_SUCCESS =
    auth_completed.admin ->
    page_navigated."/admin/users" ->
    form_filled.("name", "Alice") ->
    form_filled.("email", "alice@test.com") ->
    button_clicked."create-user-btn" ->
    toast_shown.(success, "User created") ->
    user_listed.("Alice", "alice@test.com") ->
    STOP

CREATE_USER_DUPLICATE =
    user_exists.("alice@test.com") ->
    page_navigated."/admin/users" ->
    form_filled.("email", "alice@test.com") ->
    button_clicked."create-user-btn" ->
    toast_shown.(error, "Email already in use") ->
    user_count_stable ->
    STOP

CREATE_USER_UX = CREATE_USER_SUCCESS [] CREATE_USER_DUPLICATE

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- ASSERTIONS
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

assert CREATE_USER_UX :[deadlock free]
assert CREATE_USER_UX :[divergence free]

-- Traces refinement: implementation accepts only valid traces
assert SPEC [T= IMPL
```

### Generated Rust Integration Tests

```rust
// File: tests/e2e/create_user_ux_test.rs
// @specforge-capability create_user_ux
// @specforge-traces create_user_ux.csp

use specforge_test::prelude::*;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// VALID TRACE TESTS
// Each scenario is a valid CSP trace
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[tokio::test]
#[specforge::test("create_user_ux")]
async fn trace_create_user_success() {
    // CSP Trace: CREATE_USER_SUCCESS
    let mut ctx = TestContext::new().await;

    // Event 1: auth_completed(admin)
    ctx.login_as("admin").await;

    // Event 2: page_navigated("/admin/users")
    ctx.navigate("/admin/users").await;
    assert_eq!(ctx.current_url(), "/admin/users");

    // Event 3: form_filled(field: "name", value: "Alice")
    ctx.fill_form("name", "Alice").await;

    // Event 4: form_filled(field: "email", value: "alice@test.com")
    ctx.fill_form("email", "alice@test.com").await;

    // Event 5: button_clicked("create-user-btn")
    ctx.click_button("create-user-btn").await;

    // Event 6: toast_shown(success, "User created")
    let toast = ctx.wait_for_toast().await;
    assert_eq!(toast.type_, ToastType::Success);
    assert!(toast.message.contains("User created"));

    // Event 7: user_listed(name: "Alice", email: "alice@test.com")
    let users = ctx.get_user_list().await;
    assert!(users.iter().any(|u| u.name == "Alice" && u.email == "alice@test.com"));
}

#[tokio::test]
#[specforge::test("create_user_ux")]
async fn trace_duplicate_email_rejection() {
    // CSP Trace: CREATE_USER_DUPLICATE
    let mut ctx = TestContext::new().await;

    // Event 1: user_exists("alice@test.com")
    ctx.db().insert_user("alice@test.com", "Existing Alice").await;

    // Event 2: page_navigated("/admin/users")
    ctx.navigate("/admin/users").await;

    // Event 3: form_filled(field: "email", value: "alice@test.com")
    ctx.fill_form("email", "alice@test.com").await;

    // Event 4: button_clicked("create-user-btn")
    ctx.click_button("create-user-btn").await;

    // Event 5: toast_shown(error, "Email already in use")
    let toast = ctx.wait_for_toast().await;
    assert_eq!(toast.type_, ToastType::Error);
    assert!(toast.message.contains("Email already in use"));

    // Event 6: user_count_stable
    let user_count = ctx.db().count_users().await;
    assert_eq!(user_count, 1); // Only the pre-existing user
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// INVALID TRACE TESTS
// These traces violate CSP specification → should fail
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[tokio::test]
#[specforge::test("create_user_ux")]
#[should_panic(expected = "Invalid trace: form_filled before page_navigated")]
async fn invalid_trace_form_before_navigation() {
    // Invalid CSP trace: form_filled BEFORE page_navigated
    // CSP model requires: auth -> navigate -> fill
    // This test violates that ordering

    let mut ctx = TestContext::new().await;
    ctx.login_as("admin").await;

    // Attempt to fill form before navigation (invalid trace)
    ctx.fill_form("name", "Alice").await; // ← Should panic here
}

#[tokio::test]
#[specforge::test("create_user_ux")]
#[should_panic(expected = "Invalid trace: button_clicked without form_filled")]
async fn invalid_trace_button_before_form() {
    // Invalid CSP trace: button_clicked without prior form_filled events

    let mut ctx = TestContext::new().await;
    ctx.login_as("admin").await;
    ctx.navigate("/admin/users").await;

    // Attempt to click button without filling form (invalid trace)
    ctx.click_button("create-user-btn").await; // ← Should panic here
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DEADLOCK FREEDOM TEST
// Generated from: verify deadlock_free
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[tokio::test]
#[specforge::test("create_user_ux")]
async fn deadlock_free_property() {
    // Property: All partial traces can make progress
    // CSP assertion: CREATE_USER_UX :[deadlock free]

    let mut ctx = TestContext::new().await;
    ctx.login_as("admin").await;
    ctx.navigate("/admin/users").await;

    // At every step, the system should be able to proceed
    // Test: fill form partially, ensure UI remains responsive

    ctx.fill_form("name", "Alice").await;
    ctx.assert_responsive().await; // No deadlock after name fill

    ctx.fill_form("email", "alice@test.com").await;
    ctx.assert_responsive().await; // No deadlock after email fill

    // Even if user stops here (doesn't click button), UI should remain responsive
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    ctx.assert_responsive().await; // Still responsive after timeout
}
```

---

## Example 4: Proof Obligation Report

### Command
```bash
$ specforge trace --proof-obligations --test-results specforge-report.json
```

### Output
```
╭─────────────────────────────────────────────────────────────────────────────────╮
│                         SpecForge Proof Obligation Report                        │
│                                                                                   │
│  Project: example-app                                                            │
│  Generated: 2026-03-04 14:55:32                                                  │
│  Test Results: specforge-report.json                                             │
╰─────────────────────────────────────────────────────────────────────────────────╯

╭────────────────────────┬──────┬───────────┬──────────┬─────────────┬──────────╮
│ Entity                 │ POs  │ Discharge │ Mutation │ Model Check │ Verdict  │
├────────────────────────┼──────┼───────────┼──────────┼─────────────┼──────────┤
│ create_user            │ 8/8  │   100%    │  7/8 88% │      —      │ STRONG   │
│ authenticate_password  │ 6/6  │   100%    │  6/6 100%│      —      │ STRONG   │
│ create_user_ux         │ 2/2  │   100%    │    —     │  ✅ PASS    │ VERIFIED │
│ withdraw_funds         │ 5/5  │   100%    │  3/5 60% │      —      │ PROVED   │
│ delete_user            │ 4/7  │    57%    │    —     │      —      │ PARTIAL  │
│ import_csv             │ 0/0  │     —     │    —     │      —      │ UNPROVED │
╰────────────────────────┴──────┴───────────┴──────────┴─────────────┴──────────╯

Overall Coverage: 25/28 proof obligations discharged (89%)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Detailed Proof Obligations: create_user

  ✅ Precondition: "email is valid format"
     → test: create_user__precond_invalid_email_format (PASS, 12ms)

  ✅ Precondition: "email not already in use"
     → test: create_user__precond_email_already_exists (PASS, 23ms)

  ✅ Precondition: "name is non-empty"
     → test: create_user__precond_name_is_empty (PASS, 8ms)

  ✅ Postcondition: "user record created"
     → test: create_user__postcond_user_record_created (PASS, 15ms)

  ✅ Postcondition: "returned ID matches record"
     → test: create_user__postcond_returned_id_matches_record (PASS, 18ms)

  ✅ Postcondition: "email is queryable"
     → test: create_user__postcond_email_is_queryable (PASS, 14ms)

  ✅ Invariant: unique_email_invariant
     → test: create_user__invariant_unique_email_property (PASS, 245ms, 100 cases)

  ✅ Invariant: concurrent uniqueness
     → test: create_user__invariant_concurrent_uniqueness (PASS, 180ms, 50 cases)

Mutation Coverage: 7/8 mutants killed (88%)

  ✅ KILLED: Remove email format validation (killed by: precond_invalid_email_format)
  ✅ KILLED: Remove duplicate email check (killed by: precond_email_already_exists)
  ✅ KILLED: Remove empty name check (killed by: precond_name_is_empty)
  ✅ KILLED: Skip database insert (killed by: postcond_user_record_created)
  ✅ KILLED: Return wrong user ID (killed by: postcond_returned_id_matches_record)
  ❌ SURVIVED: Skip email queryability check
  ✅ KILLED: Allow duplicate emails (killed by: invariant_unique_email_property)
  ✅ KILLED: Race condition on concurrent inserts (killed by: invariant_concurrent_uniqueness)

⚠️  Weak test detected: postcond_email_is_queryable does not assert strongly enough

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Verdict Summary:

  • VERIFIED  (1): All POs discharged + mutations killed + model checked
  • STRONG    (2): All POs discharged + mutation coverage ≥80%
  • PROVED    (1): All POs discharged
  • PARTIAL   (1): 50-99% POs discharged
  • UNPROVED  (1): No contract block

Next Steps:
  - Fix weak test in create_user (survived mutant)
  - Complete delete_user proof obligations (3 missing tests)
  - Add contract block to import_csv
```

---

## Summary

These examples demonstrate:

1. **DbC Contracts** → automatic test generation (preconditions, postconditions, invariants)
2. **Refinement** → abstract/concrete behavior tracking with refinement POs
3. **CSP Traces** → scenario steps become formal event sequences with deadlock checking
4. **Proof Obligations** → coverage measured by clause discharge, not just "tests exist"
5. **Mutation Testing** → contracts define expected mutations to kill

**Result**: SpecForge 2.0 doesn't just track "tests exist" — it **proves correctness** via proof obligation discharge.
