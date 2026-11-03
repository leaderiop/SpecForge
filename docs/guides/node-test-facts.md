# Recording node --test tests as test facts

This guide makes a JavaScript project's `node --test` tests into facts the
spec can cite: each test becomes a `test` entity with its last status, and an
entity names the tests that prove it by their ids. It takes two extensions:

- **`@code/core`** declares the test facts (`test` and `test_run`) and checks
  every reference a spec writes to them
  ([ADR 0329](../adr/0329-code-core-holds-test-facts-and-their-ids.md)).
- **`@javascript/node-test`** is the runner pack. `specforge collect` runs
  your project's own `node --test` for it, and it turns the JUnit report into
  facts ([ADR 0327](../adr/0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md)).

Nothing is installed on the JavaScript side: the pack runs the `node` already
on your `PATH`. It needs node 20.10 or later, for the JUnit reporter.

## 1. Enable the extensions

```bash
specforge add @javascript/node-test   # also enables @code/core
```

`specforge init` does not enable it for you.

## 2. Collect

```bash
specforge collect --runner node-test
```

A `package.json` at the project root selects the pack, so a plain
`specforge collect` runs it beside every other runner it detects. It runs:

```
node --test --test-reporter=spec --test-reporter-destination=stdout --test-reporter=junit --test-reporter-destination=<report>
```

- **Output:** the `spec` reporter prints node's usual output; the `junit`
  reporter writes `.specforge/reports/node-test.xml`, which the pack reads.
- **Approval:** the first run asks you to approve the command, and the answer
  is remembered per project and command. In CI, pass `--yes`.
- **Failing tests:** they don't make `collect` fail. Node exits 1, `collect`
  says so, and the failing tests are recorded as `failed`.
- **Loaders:** the pack runs `node` itself, not your `test` script, so its
  flags don't apply. A project that needs a loader sets it in `NODE_OPTIONS`
  (`NODE_OPTIONS="--import tsx" specforge collect`), which reaches node with
  the rest of the environment.
- **Your own run:** run node with the JUnit reporter writing to
  `.specforge/reports/node-test.xml` yourself, then
  `specforge collect --runner node-test --no-run`.
- **Where the facts go:** `.specforge/facts/<key>/node-test.json`, generated
  and not committed; `specforge-harness` records their digest.

## 3. The test ids

A test's id is `js:<package>/<file>#<suite > … > name>`:

- `<package>` is the `name` of the nearest `package.json` at or above the
  test's file, `@scope/` kept. A workspace's tests are named after their own
  package.
- `<file>` is the test file's path under that package's directory, with `/`.
- `<suite > … > name>` is the `describe` path, outermost first, then the test's
  name; a top-level test is `#<name>`.

```js
// package.json: { "name": "taskboard" }   test/tasks.test.js:
test('a task is added with the next id', () => { /* ... */ })
// js:taskboard/test/tasks.test.js#a task is added with the next id
```

## 4. Cite a test

Cite a test by its id, quoted, in a field that names test facts. A GSD truth
names the tests that prove it in its `proof`:

```spec
truth TR1 "A task is added with the next id" {
  proof ["js:taskboard/test/tasks.test.js#a task is added with the next id"]
}
```

`@code/core` checks each citation: one no collected test holds is
`code/E101`, one cited before any collect is `code/W102`, and a cited test
whose last run failed is `code/W401`. While the citing entity is still pending
(its plan not complete), a missing or red test is not reported: that is TDD.

## Troubleshooting

| What you see | Why, and the fix |
|---|---|
| `E045` node-test produced no report at … | node is older than 20.10 and refused the JUnit reporter, or it stopped before writing. Run the command above yourself to see node's message. |
| `<file>: no package.json with a name at or above it` | The test's package has no `name`. Give it one: the id is named after it, so a later name would change every id. |
| `node's JUnit reporter wrote no file for '<test>'` | node is older than 20.10. Upgrade node. |
| `E045` the command cannot start | `node` is not on the `PATH` `specforge` runs with. |
