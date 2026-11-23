# @javascript/playwright fixtures

| Fixture | Source |
|---------|--------|
| `playwright-1.65.1.junit.xml` | recorded with Playwright 1.65.1 by the Phase 11 research from standard output: `npx playwright test --reporter=junit` with `testDir: './test'`, so the one file `test/board.spec.js` is written `board.spec.js` (the tests serve a root `package.json` named `board`): `describe("board")` holding `describe("inner")` with a passing and a failing test, and a skipped test, then a top-level test. The research directory in the failure's stack trace is replaced by `/project` |
