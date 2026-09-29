# Tracing vitest tests to spec entities

This guide links a TypeScript or JavaScript project's vitest tests to the
entities they prove, so `specforge analyze coverage` can tell which `verify`
obligations are backed by a passing test, which by a failing one, and which
by nothing.

It takes two extensions:

- **`@specforge/testing`** gives kinds like `behavior` and `invariant` their
  `verify` obligations.
- **`@specforge/vitest`** is the runner extension. `specforge collect` runs
  vitest for it and maps the JSON report onto the spec
  ([ADR 0002](../adr/0002-test-runner-extensions.md)).

Tests link themselves through vitest's own test metadata, so there's
nothing to install on the JavaScript side. It needs a vitest version that
accepts `meta` in test options; it was checked against vitest 5.0.

## 1. Enable the extensions

`specforge init` enables both when you ask for `@specforge/software` in a
project with a `vitest.config.*` or `vitest.workspace.*` file, or with vitest
in `package.json`:

```bash
specforge init --extensions @specforge/software
```

In an existing project:

```bash
specforge add @specforge/vitest   # also enables @specforge/testing
```

## 2. Declare the obligations

```spec
behavior create_user "Create User" {
  contract "..."
  verify unit "rejects a duplicate email"
  verify unit "stores a hashed password"
}
```

## 3. Link the tests

Put a `specforge` entry in the test's `meta`: the entity's kind and ID, and
optionally the obligation it proves.

```ts
import { describe, test, expect } from 'vitest'

test('rejects a duplicate email',
  { meta: { specforge: { behavior: 'create_user', verify: 'rejects a duplicate email' } } },
  () => {
    // ...
  })
```

The same link can be set in other places:

| Where | Example |
|---|---|
| Test options | `test(name, { meta: { specforge: { behavior: 'create_user' } } }, fn)` |
| At run time | `test(name, ({ task }) => { task.meta.specforge = { invariant: 'unique_ids' } })` |
| A `describe` block (its tests inherit it) | `describe(name, { meta: { specforge: { behavior: 'delete_user' } } }, () => { ... })` |
| Several entities | `{ specforge: [{ behavior: 'create_user' }, { invariant: 'unique_ids' }] }` |

Tests without a `specforge` entry prove nothing and are left out.

**TypeScript:** vitest's `TaskMeta` type doesn't know the `specforge` key, so
`tsc` rejects it. Declare it once, for example in `specforge.d.ts`:

```ts
import 'vitest'

type SpecforgeLink = { verify?: string } & Record<string, string>

declare module 'vitest' {
  interface TaskMeta {
    specforge?: SpecforgeLink | SpecforgeLink[]
  }
}
```

## 4. Collect and analyze

```bash
specforge collect            # runs vitest, records results
specforge analyze coverage   # reads specforge-report.json
```

`collect` runs the following, with the usual terminal output plus a JSON
report in `.specforge/reports/vitest.json`:

```
npx --no vitest run --reporter=default --reporter=json --outputFile.json=<report>
```

- **No downloads:** `--no` means it only runs the project's own vitest; it
  never downloads one.
- **Approval:** the first run asks you to approve the command, and the answer
  is remembered per project and command in `~/.specforge/collector-consent.json`.
- **Failing tests:** they don't make `collect` fail. They're recorded, and
  `analyze` reports them (A014).
- **Skipped tests:** skipped and todo tests are counted but prove nothing.
- **Generated files:** `specforge-report.json` and `.specforge/` are
  generated; add them to `.gitignore`.
- **CI:** pass `--yes`. Or run vitest yourself with the JSON reporter
  writing to `.specforge/reports/vitest.json`, then `specforge collect --no-run`.
- **Mixed Rust and TypeScript:** in a project with both, `collect` runs vitest
  and `cargo test` (see [Rust test tracing](rust-test-tracing.md)); pick one
  with `--runner vitest`.

## Troubleshooting

- **`W115 ... reported tests for unknown entity 'x'`:** a test's `specforge`
  entry names an entity no spec declares. Fix the entry.
- **`E058 no test runner detected`:** several runners are enabled and none
  was detected, for example with vitest configured inside `vite.config.ts`.
  Use `--runner vitest`.
- **`E045 ... produced no report`:** vitest didn't start. Check that it's
  installed in the project (`npm install -D vitest`); the runner's output
  above the error says what went wrong.
