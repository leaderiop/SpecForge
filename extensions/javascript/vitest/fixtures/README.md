# @javascript/vitest fixtures

| Fixture | Source |
|---------|--------|
| `vitest-5.1.1.junit.xml` | recorded with vitest 5.1.1 by the Phase 11 research: `npx vitest run --reporter=default --reporter=junit --outputFile.junit=<file>` over one file, `test/board.test.js` (the tests serve a root `package.json` named `board`): `describe("board")` holding `describe("inner")` with a passing and a failing test, and a skipped test, then a top-level test. The machine's hostname is replaced by `host`; the report held no absolute path |
