// @specforge/typescript extension events

use "extensions/typescript/types"

event ts_project_scanned "TypeScript Project Scanned" {
  channel   "source.ts_project_scanned"

  payload   TsScanResult

  verify integration "emits ts_project_scanned with correct item count and file stats"
}

event ts_tests_collected "TypeScript Tests Collected" {
  channel   "coverage.ts_collected"

  payload   TsTestsCollectedPayload

  verify integration "emits ts_tests_collected with correct mapping counts and format"
}
