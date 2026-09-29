// @specforge/typescript extension events

use "extensions/typescript/types"

event ts_project_scanned "TypeScript Project Scanned" {
  channel "source.ts_project_scanned"
  payload TsScanResult
  verify integration "emits ts_project_scanned with correct item count and file stats"
}
