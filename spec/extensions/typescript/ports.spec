// @specforge/typescript extension ports

use "types/errors"
use "extensions/coverage/types"
use "extensions/typescript/types"

port TsSourceScanner {
  direction outbound
  category  "source/typescript"

  method scanFile(path: string) -> Result<TsSourceItem[], EmitterError>
  method scanDirectory(root: string, config: TsExtensionConfig) -> Result<TsScanResult, EmitterError>
  method resolveBarrelExport(barrel_path: string, symbol_name: string) -> Result<TsSourceAnchor, EmitterError>
  method findDefinition(entity_id: string, project_root: string) -> Result<TsSourceAnchor, EmitterError>
  verify integration "TsSourceScanner contract is satisfied"
}

port TsTestOutputParser {
  direction outbound
  category  "testing/typescript"

  method parseJestJson(input: string) -> Result<TestResultEntry[], EmitterError>
  method parseVitestJson(input: string) -> Result<TestResultEntry[], EmitterError>
  method parsePlaywrightJson(input: string) -> Result<TestResultEntry[], EmitterError>
  method parseCypressJunit(path: string) -> Result<TestResultEntry[], EmitterError>
  method parseMochaJson(input: string) -> Result<TestResultEntry[], EmitterError>
  method parseTap(input: string) -> Result<TestResultEntry[], EmitterError>
  verify integration "TsTestOutputParser contract is satisfied"
}

port TsProjectDetector {
  direction outbound
  category  "source/typescript"

  method detectMonorepo(root: string) -> Result<TsMonorepoInfo, EmitterError>
  method detectFrameworks(root: string) -> Result<TsFrameworkDetection[], EmitterError>
  method readPackageJson(path: string) -> Result<FieldMap, EmitterError>
  method readTsConfig(path: string) -> Result<FieldMap, EmitterError>
  verify integration "TsProjectDetector contract is satisfied"
}
