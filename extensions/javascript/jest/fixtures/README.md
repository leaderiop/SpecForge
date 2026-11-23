# @javascript/jest fixtures

| Fixture | Source |
|---------|--------|
| `jest-30.5.2-jest-junit-17.0.0.junit.xml` | recorded with jest 30.5.2 and jest-junit 17.0.0 by the Phase 11 research through the pack's env file: `node --env-file=jest-junit.env.toml node_modules/jest/bin/jest.js --reporters=default --reporters=jest-junit`, the file holding the pack's six `JEST_JUNIT_*` lines. One file, `test/board.test.js` (the tests serve a root `package.json` named `board`): `describe("board")` holding `describe("inner")` with a passing and a failing test, and a skipped test, then a top-level test. The research directory in the failure's stack trace is replaced by `/project` |
