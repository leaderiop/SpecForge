// Inbound ports — interfaces the system offers to the outside world

use types/core
use types/graph
use types/diagnostics
use types/config
use types/errors
use types/codegen
use types/coverage
use types/formatting

port CompilerApi {
  direction inbound
  category  "api/compiler"

  method check(config: CompilerConfig) -> Result<DiagnosticBag, ParseError>
  method parse(path: string) -> Result<SpecFile, ParseError>
  method resolve(files: SpecFile[]) -> Result<Graph, ResolutionError>
  method validate(graph: Graph) -> Result<DiagnosticBag, ValidationError>
  method trace(graph: Graph, entityId: EntityId) -> Result<TraceChain, ResolutionError>
  method render(graph: Graph, format: string, outDir: string) -> Result<GeneratedFile[], EmitterError>
  method generate(graph: Graph, config: GenConfig) -> Result<GenOutput, EmitterError>
  method coverage(reports: string[]) -> Result<CoverageReport, EmitterError>
  method migrate(fromVersion: string, toVersion: string) -> Result<GeneratedFile[], EmitterError>
  method format(paths: string[], config: FormatConfig) -> Result<FormatDiff[], EmitterError>
}

port LspProtocol {
  direction inbound
  category  "api/lsp"

  method initialize(config: CompilerConfig) -> Result<void, PackageError>
  method didOpen(path: string, content: string) -> Result<DiagnosticBag, ParseError>
  method didChange(path: string, content: string) -> Result<DiagnosticBag, ParseError>
  method didClose(path: string) -> Result<void, never>
  method gotoDefinition(path: string, line: integer, col: integer) -> Result<SourceSpan, ResolutionError>
  method findReferences(entityId: EntityId) -> Result<SourceSpan[], never>
  method hover(path: string, line: integer, col: integer) -> Result<string, never>
  method completion(path: string, line: integer, col: integer) -> Result<string[], never>
  method rename(entityId: EntityId, newName: string) -> Result<GeneratedFile[], ResolutionError>
  method prepareRename(path: string, line: integer, col: integer) -> Result<SourceSpan, never>
  method codeAction(path: string, line: integer, col: integer) -> Result<string[], never>
  method documentSymbol(path: string) -> Result<string[], never>
  method workspaceSymbol(query: string) -> Result<string[], never>
  method semanticTokens(path: string) -> Result<string[], never>
  method formatting(path: string, content: string) -> Result<string[], never>
  method rangeFormatting(path: string, content: string, startLine: integer, endLine: integer) -> Result<string[], never>
}
