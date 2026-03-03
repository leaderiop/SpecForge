---
id: RES-11b
kind: research
title: "Spec DSL Code Generation & Test Plugins"
status: active
date: 2026-03-01
split_from: RES-11
depends_on: RES-11a
---

# RES-11b: Spec DSL Code Generation & Test Plugins

## Context & Dependency

This document covers code generation and test coverage plugins — the integration layer that connects `.spec` behavioral contracts to language-specific types, ports, test stubs, and coverage reporting. It builds on the core compiler architecture defined in [RES-11a](./RES-11a-spec-dsl-core-compiler.md).

Build-order dependencies (not phase gates):

- The `.spec` parser, resolver, and validator must be operational (RES-11a steps 1–4)
- The in-memory graph must be available for consumption
- The CLI can `check`, `trace`, and `render` (RES-11a step 5)

---

## Integration Problem Statement

The core compiler (RES-11a) validates that `.spec` files are structurally correct and internally consistent. But two critical questions remain unanswered:

1. **Are the specified behaviors actually tested?** — The compiler knows `BEH-MS-001` exists, but not whether any test exercises it.
2. **Does the code match the spec?** — The compiler knows `UserRepository` is a port, but not whether a TypeScript/Python/Go interface exists for it.

This document addresses both questions:

- **Test coverage plugins** connect test runners to `.spec` behavior IDs, producing structured reports
- **Code generation** produces types, ports, and test stubs from `.spec` definitions, like protobuf generates stubs for gRPC

Together, they close the loop: spec → code → test → report → coverage gate.

---

## Test Coverage: Framework-Native Plugins

Comment scanning (`// @spec BEH-MS-001`) is fragile — it's just a string, it tells you nothing about pass/fail, and it can't validate that the test actually exercises the behavior. Instead, specforge ships **test runner plugins** that integrate natively with each framework.

### The Protocol: `specforge-report.json`

Every plugin emits a standard report file after test execution:

```json
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
        },
        {
          "name": "rejects duplicate email",
          "file": "tests/user-crud.test.ts",
          "line": 28,
          "status": "pass",
          "duration_ms": 32
        }
      ],
      "status": "covered"
    },
    "BEH-MS-002": {
      "tests": [],
      "status": "missing"
    }
  },
  "invariants": {
    "INV-MS-2": {
      "violations": [
        {
          "name": "rejects concurrent duplicate emails",
          "file": "tests/user-crud.test.ts",
          "line": 45,
          "status": "pass",
          "duration_ms": 120
        }
      ],
      "status": "covered"
    }
  }
}
```

The specforge CLI reads this file — not source comments — for coverage analysis.

---

## `@specforge/vitest` — TypeScript / JavaScript

```typescript
// vitest.config.ts
import { specforgeReporter } from "@specforge/vitest";

export default defineConfig({
  test: {
    reporters: [specforgeReporter({ specDir: "./spec" })],
  },
});
```

```typescript
// tests/user-crud.test.ts
import { spec, violation } from "@specforge/vitest";

spec("BEH-MS-001", () => {
  it("creates user with unique email", async () => {
    const repo = createUserRepository(testDb);
    const result = await repo.create({
      email: "a@b.com",
      name: "Alice",
      role: "admin",
    });
    expect(result.isOk()).toBe(true);

    const found = await repo.findById(result.value.id);
    expect(found.value).toEqual(result.value);
  });

  it("rejects duplicate email", async () => {
    const repo = createUserRepository(testDb);
    await repo.create({ email: "a@b.com", name: "Alice", role: "admin" });
    const dup = await repo.create({ email: "a@b.com", name: "Bob", role: "viewer" });
    expect(dup.isErr()).toBe(true);
    expect(dup.error._tag).toBe("DuplicateEmailError");
  });
});

// Invariant violation test — proves the system PREVENTS the violation
violation("INV-MS-2", () => {
  it("rejects concurrent duplicate emails", async () => {
    const repo = createUserRepository(testDb);
    const results = await Promise.all([
      repo.create({ email: "same@b.com", name: "A", role: "admin" }),
      repo.create({ email: "same@b.com", name: "B", role: "admin" }),
    ]);

    const successes = results.filter((r) => r.isOk());
    const failures = results.filter((r) => r.isErr());
    expect(successes).toHaveLength(1);
    expect(failures).toHaveLength(1);
    expect(failures[0].error._tag).toBe("DuplicateEmailError");
  });
});
```

**What `spec()` does at runtime:**
- Wraps `describe()` — all tests inside are tagged with the behavior ID
- The reporter collects per-behavior results: pass, fail, skip, duration
- If a `spec("BEH-MS-099")` references an ID that doesn't exist in `.spec` files, the reporter **fails the test suite** with a clear error
- Emits `specforge-report.json` at the end of the run

**What `violation()` does at runtime:**
- Same as `spec()` but tags the test as an invariant violation test
- These tests prove the system prevents a bad state, not just that the happy path works

---

## `@specforge/pytest` — Python

```python
# conftest.py
pytest_plugins = ["specforge.pytest"]

# pytest.ini or pyproject.toml
[tool.specforge]
spec_dir = "./spec"
```

```python
# tests/test_user_crud.py
from specforge.pytest import spec, violation

@spec("BEH-MS-001")
class TestCreateUser:
    async def test_creates_user(self, user_repo):
        result = await user_repo.create(
            CreateUserCommand(email="a@b.com", name="Alice", role=UserRole.ADMIN)
        )
        assert result.is_ok()

        found = await user_repo.find_by_id(result.ok().id)
        assert found.ok() == result.ok()

    async def test_rejects_duplicate_email(self, user_repo):
        await user_repo.create(
            CreateUserCommand(email="a@b.com", name="Alice", role=UserRole.ADMIN)
        )
        dup = await user_repo.create(
            CreateUserCommand(email="a@b.com", name="Bob", role=UserRole.VIEWER)
        )
        assert dup.is_err()
        assert dup.err()._tag == "DuplicateEmailError"


@violation("INV-MS-2")
class TestEmailUniquenessInvariant:
    async def test_concurrent_duplicate_emails(self, user_repo):
        """Two concurrent creates with same email — exactly one wins."""
        results = await asyncio.gather(
            user_repo.create(CreateUserCommand(email="same@b.com", name="A", role=UserRole.ADMIN)),
            user_repo.create(CreateUserCommand(email="same@b.com", name="B", role=UserRole.ADMIN)),
        )
        successes = [r for r in results if r.is_ok()]
        failures = [r for r in results if r.is_err()]
        assert len(successes) == 1
        assert len(failures) == 1
```

**How the pytest plugin works:**
- `@spec("BEH-MS-001")` is a decorator that marks a test class or function
- The plugin registers a custom pytest reporter that collects results per behavior ID
- At session end, emits `specforge-report.json`
- Validates behavior IDs against `.spec` files at collection time — unknown IDs fail immediately

---

## `@specforge/go` — Go

```go
// user_crud_test.go
package user_test

import (
    "testing"
    "github.com/specforge/go-specforge"
)

func TestCreateUser(t *testing.T) {
    specforge.Spec(t, "BEH-MS-001")

    repo := createUserRepository(testDB)

    t.Run("creates user with unique email", func(t *testing.T) {
        result, err := repo.Create(ctx, CreateUserCommand{
            Email: "a@b.com", Name: "Alice", Role: RoleAdmin,
        })
        require.NoError(t, err)
        require.NotNil(t, result)

        found, err := repo.FindByID(ctx, result.ID)
        require.NoError(t, err)
        require.Equal(t, result, found)
    })

    t.Run("rejects duplicate email", func(t *testing.T) {
        _, _ = repo.Create(ctx, CreateUserCommand{
            Email: "a@b.com", Name: "Alice", Role: RoleAdmin,
        })
        _, err := repo.Create(ctx, CreateUserCommand{
            Email: "a@b.com", Name: "Bob", Role: RoleViewer,
        })
        require.ErrorAs(t, err, &DuplicateEmailError{})
    })
}

func TestEmailUniquenessInvariant(t *testing.T) {
    specforge.Violation(t, "INV-MS-2")

    // ... concurrent insert test
}
```

**How the Go plugin works:**
- `specforge.Spec(t, "BEH-MS-001")` registers the current test with that behavior ID
- A `TestMain` hook or `-json` output parser collects results
- `go test -json ./... | specforge collect go` parses the JSON test output and emits `specforge-report.json`

---

## CLI: `specforge coverage`

The CLI reads `specforge-report.json` files (one per test runner) and merges them:

```bash
$ specforge coverage

BEHAVIOR COVERAGE (from 3 reports: vitest, pytest, go)
═══════════════════════════════════════════════════════════════════════
ID          Title                 Tests  Pass  Fail  Skip  Runners
───────────────────────────────────────────────────────────────────────
BEH-MS-001  Create User           5      5     0     0     vitest,pytest,go
BEH-MS-002  Read User by ID       2      2     0     0     vitest,go
BEH-MS-003  Update User Email     3      2     1     0     vitest         ← FAILING
BEH-MS-004  Delete User           2      2     0     0     pytest,go
BEH-MS-005  List Users            0      —     —     —     —              ← MISSING
BEH-MS-006  Search Users          1      1     0     0     go
BEH-MS-007  Change Password       0      —     —     —     —              ← MISSING
BEH-MS-008  Soft Delete User      0      —     —     —     —              ← MISSING
───────────────────────────────────────────────────────────────────────
Total: 5/8 behaviors covered (62.5%)
       15 tests | 12 pass | 1 fail | 0 skip
Threshold: 95%    FAIL

INVARIANT VIOLATION TESTS
═══════════════════════════════════════════════════════════════════════
ID          Title                 Behaviors  Violation Tests   Status
───────────────────────────────────────────────────────────────────────
INV-MS-1    Data Persistence      5          1 pass            ok
INV-MS-2    Email Uniqueness      2          1 pass            ok
INV-MS-3    Audit Integrity       3          0                 MISSING
───────────────────────────────────────────────────────────────────────

FAILING BEHAVIORS
═══════════════════════════════════════════════════════════════════════
BEH-MS-003 "Update User Email"
  FAIL  tests/user-crud.test.ts:52 "concurrent updates — exactly one wins"
        AssertionError: expected 1, got 2 (both succeeded)
        Duration: 230ms

ORPHANS (defined in .spec but not referenced by any feature)
═══════════════════════════════════════════════════════════════════════
BEH-MS-008 "Soft Delete User" — no feature references it
```

---

## Key Differences from Comment Scanning

| Aspect | Comment scanning | Framework plugins |
|---|---|---|
| Knows if test passes | No — just "comment exists" | Yes — per-behavior pass/fail/skip |
| Duration tracking | No | Yes — per test and per behavior |
| Validates IDs | No — typos are invisible | Yes — unknown ID = test failure |
| Multi-runner merge | Grep across files | Merge `specforge-report.json` files |
| Failing behavior detail | No | Yes — shows assertion error + file:line |
| CI integration | Fragile grep | Standard JSON report |
| Invariant violations | Cannot distinguish | First-class `violation()` primitive |
| Runtime overhead | Zero | Minimal (decorator + reporter) |

---

## Project Configuration (Coverage)

```spec
// specforge.spec

spec "my-service" {
  infix   "MS"
  version "1.0"

  coverage {
    threshold     95           // minimum % of behaviors covered
    reports [                  // paths to specforge-report.json files
      "specforge-report.json",                    // default
      "services/auth/specforge-report.json",      // monorepo subproject
    ]
    require_violation_tests  true     // every invariant needs a violation() test
    fail_on_unknown_ids      true     // spec("BEH-XX-999") fails if ID not in .spec
  }
}
```

---

## CI Pipeline

```yaml
# .github/workflows/spec.yml
jobs:
  spec-check:
    steps:
      - run: specforge check --strict

  test-ts:
    steps:
      - run: pnpm vitest run          # emits specforge-report.json via reporter
      - uses: actions/upload-artifact@v4
        with: { name: report-ts, path: specforge-report.json }

  test-py:
    steps:
      - run: pytest                    # emits specforge-report.json via plugin
      - uses: actions/upload-artifact@v4
        with: { name: report-py, path: specforge-report.json }

  test-go:
    steps:
      - run: go test -json ./... | specforge collect go
      - uses: actions/upload-artifact@v4
        with: { name: report-go, path: specforge-report.json }

  coverage:
    needs: [spec-check, test-ts, test-py, test-go]
    steps:
      - uses: actions/download-artifact@v4    # download all reports
      - run: specforge coverage --min=95      # merge + validate
```

---

## Language Integration: Code Generation

Coverage plugins tell you **which behaviors have tests**. Code generation goes further — it produces **types, ports, and test scaffolds** from `.spec` files, like protobuf generates stubs for gRPC.

### New DSL Concept: `type` and `port` Blocks

```spec
// types/user.spec

type User {
  id        string      @readonly
  email     string      @unique
  name      string
  role      UserRole
  createdAt timestamp   @readonly
  updatedAt timestamp   @readonly
}

type UserRole = admin | editor | viewer

type CreateUserCommand {
  email  string
  name   string
  role   UserRole
}

type DuplicateEmailError {
  _tag   "DuplicateEmailError"   @literal
  email  string
  message string
}

type UserNotFoundError {
  _tag    "UserNotFoundError"    @literal
  userId  string
  message string
}
```

```spec
// ports/user-repository.spec

use types/user

port UserRepository {
  direction outbound
  category  "persistence/user"

  method create(cmd: CreateUserCommand) -> Result<User, DuplicateEmailError>
  method findById(id: string) -> Result<User, UserNotFoundError>
  method findByEmail(email: string) -> Result<User?, never>
  method update(id: string, cmd: UpdateUserCommand) -> Result<User, DuplicateEmailError | UserNotFoundError>
  method delete(id: string) -> Result<void, UserNotFoundError>
}
```

### `specforge gen`

```bash
specforge gen typescript ./src/generated/
specforge gen python ./src/generated/
specforge gen go ./internal/generated/
specforge gen json-schema ./schemas/
```

---

## Generated: TypeScript (using hex-di stack)

```typescript
// src/generated/types/user.ts  (auto-generated — do not edit)

export interface User {
  readonly id: string;
  readonly email: string;
  readonly name: string;
  readonly role: UserRole;
  readonly createdAt: Date;
  readonly updatedAt: Date;
}

export type UserRole = "admin" | "editor" | "viewer";

export interface CreateUserCommand {
  readonly email: string;
  readonly name: string;
  readonly role: UserRole;
}

export interface DuplicateEmailError {
  readonly _tag: "DuplicateEmailError";
  readonly email: string;
  readonly message: string;
}

export interface UserNotFoundError {
  readonly _tag: "UserNotFoundError";
  readonly userId: string;
  readonly message: string;
}
```

```typescript
// src/generated/ports/user-repository.ts  (auto-generated — do not edit)

import type { ResultAsync } from "@hex-di/core";
import type {
  User, CreateUserCommand, UpdateUserCommand,
  DuplicateEmailError, UserNotFoundError,
} from "../types/user";

export interface UserRepository {
  create(cmd: CreateUserCommand): ResultAsync<User, DuplicateEmailError>;
  findById(id: string): ResultAsync<User, UserNotFoundError>;
  findByEmail(email: string): ResultAsync<User | null, never>;
  update(id: string, cmd: UpdateUserCommand): ResultAsync<User, DuplicateEmailError | UserNotFoundError>;
  delete(id: string): ResultAsync<void, UserNotFoundError>;
}
```

```typescript
// src/generated/tests/user-crud.stubs.ts  (auto-generated — do not edit)
import { spec, violation } from "@specforge/vitest";

spec("BEH-MS-001", () => {
  it.todo("Create User — insert user, retrieve by ID, assert equal");
});

spec("BEH-MS-002", () => {
  it.todo("Read User by ID — insert then get by ID");
});

spec("BEH-MS-003", () => {
  it.todo("Update User Email — update to unique email succeeds");
  it.todo("Update User Email — update to taken email fails with DuplicateEmailError");
});

violation("INV-MS-2", () => {
  it.todo("Email Uniqueness — concurrent inserts with same email, exactly one wins");
});
```

---

## Generated: Python

```python
# src/generated/types/user.py  (auto-generated — do not edit)
from __future__ import annotations
from dataclasses import dataclass
from datetime import datetime
from enum import Enum
from typing import Literal


class UserRole(Enum):
    ADMIN = "admin"
    EDITOR = "editor"
    VIEWER = "viewer"


@dataclass(frozen=True)
class User:
    id: str
    email: str
    name: str
    role: UserRole
    created_at: datetime
    updated_at: datetime


@dataclass(frozen=True)
class DuplicateEmailError:
    _tag: Literal["DuplicateEmailError"] = "DuplicateEmailError"
    email: str = ""
    message: str = ""
```

```python
# src/generated/ports/user_repository.py  (auto-generated — do not edit)
from abc import ABC, abstractmethod
from result import Result
from ..types.user import *


class UserRepository(ABC):
    @abstractmethod
    async def create(self, cmd: CreateUserCommand) -> Result[User, DuplicateEmailError]: ...

    @abstractmethod
    async def find_by_id(self, id: str) -> Result[User, UserNotFoundError]: ...

    @abstractmethod
    async def find_by_email(self, email: str) -> Result[User | None, None]: ...

    @abstractmethod
    async def update(self, id: str, cmd: UpdateUserCommand) -> Result[User, DuplicateEmailError | UserNotFoundError]: ...

    @abstractmethod
    async def delete(self, id: str) -> Result[None, UserNotFoundError]: ...
```

```python
# src/generated/tests/test_user_crud_stubs.py  (auto-generated — do not edit)
import pytest
from specforge.pytest import spec, violation


@spec("BEH-MS-001")
class TestCreateUser:
    @pytest.mark.skip(reason="stub — implement this test")
    async def test_create_user(self): ...


@spec("BEH-MS-002")
class TestReadUser:
    @pytest.mark.skip(reason="stub — implement this test")
    async def test_read_user_by_id(self): ...


@violation("INV-MS-2")
class TestEmailUniquenessInvariant:
    @pytest.mark.skip(reason="stub — implement this test")
    async def test_concurrent_duplicate_emails(self): ...
```

---

## Generated: Go

```go
// internal/generated/types/user.go  (auto-generated — do not edit)
package types

import "time"

type UserRole string

const (
    UserRoleAdmin  UserRole = "admin"
    UserRoleEditor UserRole = "editor"
    UserRoleViewer UserRole = "viewer"
)

type User struct {
    ID        string    `json:"id"`
    Email     string    `json:"email"`
    Name      string    `json:"name"`
    Role      UserRole  `json:"role"`
    CreatedAt time.Time `json:"createdAt"`
    UpdatedAt time.Time `json:"updatedAt"`
}

type DuplicateEmailError struct {
    Tag     string `json:"_tag"`
    Email   string `json:"email"`
    Message string `json:"message"`
}

func (e *DuplicateEmailError) Error() string { return e.Message }
```

```go
// internal/generated/ports/user_repository.go  (auto-generated — do not edit)
package ports

import (
    "context"
    "myservice/internal/generated/types"
)

type UserRepository interface {
    Create(ctx context.Context, cmd types.CreateUserCommand) (types.User, error)
    FindByID(ctx context.Context, id string) (types.User, error)
    FindByEmail(ctx context.Context, email string) (*types.User, error)
    Update(ctx context.Context, id string, cmd types.UpdateUserCommand) (types.User, error)
    Delete(ctx context.Context, id string) error
}
```

---

## Project Configuration (Code Generation)

```spec
// specforge.spec

spec "my-service" {
  infix   "MS"
  version "1.0"

  coverage {
    threshold                 95
    reports                   ["specforge-report.json"]
    require_violation_tests   true
    fail_on_unknown_ids       true
  }

  gen typescript {
    out       "src/generated/"
    result    "hex-di"              // ResultAsync from @hex-di/core
    readonly  true
    naming    "camelCase"
    tests     "@specforge/vitest"   // use spec() / violation() from vitest plugin
  }

  gen python {
    out       "src/generated/"
    result    "result"
    frozen    true
    naming    "snake_case"
    tests     "@specforge/pytest"
  }

  gen go {
    out       "internal/generated/"
    module    "myservice"
    naming    "PascalCase"
    tests     "@specforge/go"
  }

  gen json_schema {
    out       "schemas/"
    draft     "2020-12"
  }
}
```

---

## Drift Detection

When `.spec` files change, generated code becomes stale:

```bash
$ specforge gen typescript --check   # exits 1 if output would differ from current files

error: generated code is stale
  types/user.spec changed at 2026-03-01 14:30
  src/generated/types/user.ts last generated at 2026-02-28 10:00

  Run `specforge gen typescript` to regenerate.
```

---

## Adapter Verification

The compiler can verify that hand-written adapters implement generated ports:

```bash
$ specforge verify typescript

PORT IMPLEMENTATION CHECK
═══════════════════════════════════════════════════════════════════════
Port                 Adapter                         Status
───────────────────────────────────────────────────────────────────────
UserRepository       src/adapters/pg-user-repo.ts    ok
EmailService         src/adapters/sendgrid.ts        ok
PaymentGateway       —                               MISSING
───────────────────────────────────────────────────────────────────────
```

---

## Full Chain: Spec → Code → Test → Report

```
types/user.spec            --gen-->  src/generated/types/user.ts
ports/user-repo.spec       --gen-->  src/generated/ports/user-repository.ts
                                         ^ implements
                                     src/adapters/pg-user-repo.ts        (hand-written)
                                         ^ tested by
behaviors/user-crud.spec   --gen-->  src/generated/tests/user-crud.stubs.ts
                                         ^ developer fills in
                                     tests/user-crud.test.ts             (hand-written)
                                         uses spec("BEH-MS-001") from @specforge/vitest
                                         ^ reporter emits
                                     specforge-report.json               (auto-generated)
                                         ^ read by
                                     specforge coverage --min=95         (CI gate)
```

---

## Language Plugin Architecture

Each generator and test plugin is a standalone package:

| Package | Role | Language |
|---|---|---|
| `@specforge/vitest` | Test runner plugin (reporter + `spec()` / `violation()`) | TypeScript |
| `@specforge/jest` | Test runner plugin for Jest | TypeScript |
| `@specforge/pytest` | Pytest plugin (decorator + reporter) | Python |
| `@specforge/go` | Test helper + JSON output collector | Go |
| `@specforge/gen-typescript` | Code generator (built-in) | TypeScript |
| `@specforge/gen-python` | Code generator (built-in) | Python |
| `@specforge/gen-go` | Code generator (built-in) | Go |
| `@specforge/gen-rust` | Code generator (community) | Rust |
| `@specforge/gen-kotlin` | Code generator (community) | Kotlin |

Custom plugins read the in-memory graph as JSON from stdin and write files to stdout. Write a plugin in any language.

### Integration Depth by Language

| Level | What | TypeScript | Python | Go |
|---|---|---|---|---|
| **0** | `specforge check` only | yes | yes | yes |
| **1** | Type generation | interfaces (readonly) | frozen dataclasses | structs |
| **2** | Port generation | interface + ResultAsync | ABC + Result | interface |
| **3** | Test stub generation | `spec()` / `violation()` via `@specforge/vitest` | decorators via `@specforge/pytest` | helpers via `@specforge/go` |
| **4** | Runtime coverage reporting | vitest reporter -> `specforge-report.json` | pytest plugin -> `specforge-report.json` | `go test -json` collector |
| **5** | Adapter verification | tsc type-check | mypy check | go vet |
| **6** | Drift detection | `specforge gen --check` | `specforge gen --check` | `specforge gen --check` |

Every project gets Level 0 for free. Deeper integration is opt-in via `gen` and `coverage` blocks in `specforge.spec`.

---

## Plugin Architecture

### Common Plugin Interface

All specforge plugins — generators, test reporters, and validators — share a common interface based on subprocess I/O:

```
┌──────────────┐     JSON graph      ┌──────────────┐
│   specforge  │ ──── stdin ────────→ │    Plugin    │
│   CLI        │                      │  (any lang)  │
│              │ ←── stdout ───────── │              │
│              │     file list        │              │
│              │ ←── stderr ───────── │              │
│              │     diagnostics      │              │
└──────────────┘                      └──────────────┘
```

**stdin (JSON):** The full graph or a filtered subgraph. Schema:

```json
{
  "specforge": "1.0",
  "graph": {
    "nodes": [
      { "id": "INV-MS-1", "type": "invariant", "title": "Data Persistence", "properties": { ... } }
    ],
    "edges": [
      { "from": "BEH-MS-001", "to": "INV-MS-1", "type": "references" }
    ]
  },
  "config": {
    "gen": { "out": "src/generated/", "result": "hex-di", ... }
  }
}
```

**stdout (file list):** One or more files to write:

```json
{
  "files": [
    { "path": "src/generated/types/user.ts", "content": "..." },
    { "path": "src/generated/ports/user-repository.ts", "content": "..." }
  ]
}
```

**stderr (diagnostics):** Structured diagnostic messages:

```json
{
  "diagnostics": [
    { "level": "warning", "message": "Type 'UpdateUserCommand' referenced but not defined", "node": "UserRepository" }
  ]
}
```

### Language-Specific Plugin Patterns

| Pattern | Description | Example |
|---|---|---|
| **Generator** | Reads graph → emits source files | `@specforge/gen-typescript` |
| **Reporter** | Runs inside test framework → emits `specforge-report.json` | `@specforge/vitest` |
| **Collector** | Parses test runner output → emits `specforge-report.json` | `specforge collect go` |
| **Verifier** | Reads generated + hand-written code → emits diagnostics | `specforge verify typescript` |

### Community Plugin Development

To create a custom plugin:

1. Accept JSON graph on stdin
2. Emit files on stdout (for generators) or `specforge-report.json` (for reporters)
3. Emit diagnostics on stderr
4. Register in `specforge.spec` under a `plugin` block:

```spec
plugin "my-custom-generator" {
  command  "my-gen-binary"
  type     generator
  triggers [type, port]     // which node types this plugin cares about
}
```

---

## Language Integration Strategy

### Decision Tree: How Deep to Integrate

When adding support for a new language, follow this decision tree:

```
Does the language have a popular test framework?
  ├─ YES → Build a test runner plugin (Level 3-4)
  │         Does the framework support custom reporters?
  │         ├─ YES → Native reporter plugin (like @specforge/vitest)
  │         └─ NO  → Output collector (like specforge collect go)
  │
  └─ NO  → Comment scanning fallback only (Level 0)

Does the language benefit from generated types?
  ├─ YES (statically typed) → Build a code generator (Level 1-2)
  │         Does the language have a type checker?
  │         ├─ YES → Add adapter verification (Level 5)
  │         └─ NO  → Stop at generation
  │
  └─ NO (dynamically typed) → Skip codegen, focus on test plugins
```

### When to Use Framework Plugins vs. Codegen

| Use Case | Approach |
|---|---|
| "Which behaviors have tests?" | Test runner plugin (Level 3-4) |
| "Do my types match the spec?" | Code generation + type checker (Level 1-2, 5) |
| "Is my generated code up to date?" | Drift detection (Level 6) |
| "I just want spec validation" | Core compiler only (Level 0) |
| "I want everything" | All levels — gen + test + verify + drift |

---

## Multi-Language Support

### Monorepo Considerations

In a monorepo with multiple languages, each service may produce its own `specforge-report.json`:

```
monorepo/
  spec/                        # shared .spec files
  services/
    auth-ts/
      specforge-report.json    # from @specforge/vitest
    billing-py/
      specforge-report.json    # from @specforge/pytest
    gateway-go/
      specforge-report.json    # from specforge collect go
```

### Report Merge Strategy

`specforge coverage` merges multiple reports:

1. **Union of behaviors:** If `BEH-MS-001` appears in reports from vitest and pytest, both test sets are merged.
2. **Status priority:** `fail` > `pass` > `skip` > `missing`. If any runner reports a failure, the behavior is marked as failing.
3. **Deduplication:** Tests with identical `file:line` across reports are deduplicated (shouldn't happen in practice, but handled gracefully).
4. **Runner attribution:** Each test retains its runner tag so `specforge coverage` can show which runners cover which behaviors.

### Per-Language Configuration

```spec
// specforge.spec

spec "monorepo" {
  infix   "MS"
  version "1.0"

  coverage {
    threshold  95
    reports [
      "services/auth-ts/specforge-report.json",
      "services/billing-py/specforge-report.json",
      "services/gateway-go/specforge-report.json",
    ]
  }

  gen typescript {
    out    "services/auth-ts/src/generated/"
    result "hex-di"
    tests  "@specforge/vitest"
  }

  gen python {
    out    "services/billing-py/src/generated/"
    result "result"
    tests  "@specforge/pytest"
  }

  gen go {
    out    "services/gateway-go/internal/generated/"
    module "gateway"
    tests  "@specforge/go"
  }
}
```

---

## Implementation Plan

### Build Order

These steps continue from the core compiler build order (RES-11a steps 1–7).

| Step | Deliverable | Depends On |
|---|---|---|
| **8** | `@specforge/vitest` — TypeScript test runner plugin | Step 5 (CLI) |
| **9** | `@specforge/pytest` — Python test runner plugin | Step 5 (CLI) |
| **10** | `@specforge/go` — Go test collector | Step 5 (CLI) |
| **11** | `specforge coverage` CLI command — report merging + threshold gating | Steps 8–10 |
| **12** | `@specforge/gen-typescript` — TypeScript code generator | Step 3 (resolver) |
| **13** | `@specforge/gen-python` — Python code generator | Step 3 (resolver) |
| **14** | `@specforge/gen-go` — Go code generator | Step 3 (resolver) |
| **15** | Drift detection — `specforge gen --check` | Steps 12–14 |
| **16** | Adapter verification — `specforge verify` | Steps 12–14 |
| **17** | Plugin API — stdin/stdout interface for community plugins | Steps 11, 15 |

### Parallelization

- Steps 8–10 can proceed in parallel (independent language plugins)
- Steps 12–14 can proceed in parallel (independent language generators)
- Step 11 requires at least one test plugin (8, 9, or 10) to be testable
- Step 17 should be last — the plugin API should be informed by building the built-in plugins first

### Success Criteria

- `specforge coverage --min=95` gates CI with real test results
- `specforge gen typescript --check` catches stale generated code
- `specforge verify typescript` confirms adapters implement ports
- Community can build custom plugins using the stdin/stdout JSON interface
