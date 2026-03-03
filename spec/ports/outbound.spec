// Outbound ports — interfaces the system requires from the outside world

use types/core
use types/diagnostics
use types/errors
use types/coverage
use types/rust-codegen

port FileSystem {
  direction outbound
  category  "io/filesystem"

  method readFile(path: string) -> Result<string, EmitterError>
  method writeFile(path: string, content: string) -> Result<void, EmitterError>
  method listFiles(pattern: string) -> Result<string[], EmitterError>
  method watchFiles(patterns: string[]) -> Result<void, EmitterError>
  method exists(path: string) -> Result<boolean, never>
}

port SourceParser {
  direction outbound
  category  "compiler/parser"

  method parseSource(content: string, path: string) -> Result<SpecFile, ParseError>
  method parseIncremental(content: string, path: string, previousTree: string) -> Result<SpecFile, ParseError>
}

port OutputRenderer {
  direction outbound
  category  "io/output"

  method renderMarkdown(templateName: string, data: string) -> Result<string, EmitterError>
  method renderJson(data: string) -> Result<string, EmitterError>
  method renderDot(data: string) -> Result<string, EmitterError>
  method writeOutput(path: string, content: string) -> Result<void, EmitterError>
}

port TestReporter {
  direction outbound
  category  "testing/reporter"

  method collectResults(reportPaths: string[]) -> Result<string, EmitterError>
  method mergeReports(reports: string[]) -> Result<string, EmitterError>
  method formatCoverage(data: string) -> Result<string, EmitterError>
}

port RefValidator {
  direction outbound
  category  "validation/refs"

  method validateScheme(scheme: string) -> Result<boolean, never>
  method validateKind(scheme: string, kind: string) -> Result<boolean, never>
  method validateIdentifier(scheme: string, kind: string, identifier: string) -> Result<boolean, ValidationError>
  method resolveUrl(scheme: string, kind: string, identifier: string) -> Result<string, EmitterError>
}

port RustTestOutputParser {
  direction outbound
  category  "testing/rust"

  method parseJunitXml(path: string) -> Result<TestResultEntry[], EmitterError>
  method parseLibtestJson(input: string) -> Result<TestResultEntry[], EmitterError>
  method parseCargoText(input: string) -> Result<TestResultEntry[], EmitterError>
  method readMappingFiles(dir: string) -> Result<EntityMappingEntry[], EmitterError>
}
