// @specforge/coverage extension ports

use "types/errors"
use "extensions/coverage/types"
port TestReporter {
  direction outbound
  category  "testing/reporter"

  method collectResults(reportPaths: string[]) -> Result<CoverageReport, EmitterError>
  method mergeReports(reports: CoverageReport[]) -> Result<CoverageReport, EmitterError>
  method formatCoverage(data: CoverageReport) -> Result<string, EmitterError>
}
