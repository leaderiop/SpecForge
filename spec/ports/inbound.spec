// Inbound ports — interfaces the system offers to the outside world

use types/core
use types/graph
use types/output
use types/diagnostics
use types/config
use types/errors
use types/formatting
use types/migration
use types/lsp
use types/wasm

port CompilerApi {
  direction inbound
  category  "api/compiler"

  method check(config: CompilerConfig) -> Result<DiagnosticBag, ParseError>
  method parse(path: string) -> Result<SpecFile, ParseError>
  method resolve(files: SpecFile[]) -> Result<Graph, ResolutionError>
  method validate(graph: Graph) -> Result<DiagnosticBag, ValidationError>
  method trace(graph: Graph, entityId: EntityId @optional) -> Result<TraceChain, ResolutionError>
  method render(graph: Graph, format: string, outDir: string) -> Result<OutputFile[], EmitterError>
  method migrate(targetVersion: string @optional, dryRun: boolean @optional) -> Result<MigrationSummary, EmitterError>
  method export(graph: Graph, format: string, scope: EntityId @optional, maxTokens: integer @optional) -> Result<OutputFile, EmitterError>
  method query(graph: Graph, entityId: EntityId, depth: integer, kindFilter: string[] @optional) -> Result<Graph, ResolutionError>
  method format(paths: string[], config: FormatConfig) -> Result<FormatDiff[], EmitterError>
  method watch(config: CompilerConfig) -> Result<void, EmitterError>
  method init(config: InitConfig) -> Result<ProjectConfig, InitError>
  method add(specifier: ExtensionSpecifier) -> Result<ExtensionManifest, RegistryError>
  method validateGrammar(path: string) -> Result<GrammarContribution, GrammarError>
}

port LspProtocol {
  direction inbound
  category  "api/lsp"

  method initialize(config: CompilerConfig) -> Result<void, ExtensionError>
  method didOpen(path: string, content: string) -> Result<DiagnosticBag, ParseError>
  method didChange(path: string, changes: ContentChangeEvent[]) -> Result<DiagnosticBag, ParseError>
  method didClose(path: string) -> Result<void, never>
  method publishDiagnostics(path: string, diagnostics: Diagnostic[]) -> Result<void, never>
  method gotoDefinition(path: string, line: integer, col: integer) -> Result<SourceSpan, ResolutionError>
  method findReferences(entityId: EntityId) -> Result<SourceSpan[], never>
  method hover(path: string, line: integer, col: integer) -> Result<HoverContent, never>
  method completion(path: string, line: integer, col: integer) -> Result<CompletionItem[], never>
  method rename(entityId: EntityId, newName: string) -> Result<WorkspaceEditResult, ResolutionError>
  method prepareRename(path: string, line: integer, col: integer) -> Result<SourceSpan, never>
  method codeAction(path: string, line: integer, col: integer) -> Result<CodeAction[], never>
  method documentSymbol(path: string) -> Result<DocumentSymbolEntry[], never>
  method workspaceSymbol(query: string) -> Result<WorkspaceSymbolEntry[], never>
  method semanticTokens(path: string) -> Result<SemanticToken[], never>
  method formatting(path: string, content: string) -> Result<TextEdit[], never>
  method rangeFormatting(path: string, content: string, startLine: integer, endLine: integer) -> Result<TextEdit[], never>
}

port McpProtocol {
  direction inbound
  category  "api/mcp"

  method initialize(config: CompilerConfig) -> Result<void, ExtensionError>
  method listResources() -> Result<string[], never>
  method readResource(uri: string) -> Result<string, ExtensionError>
  method listTools() -> Result<string[], never>
  method callTool(name: string, arguments: JsonObject) -> Result<string, ExtensionError>
  method subscribe(uri: string) -> Result<void, never>
  method unsubscribe(uri: string) -> Result<void, never>
  method notify(uri: string, data: JsonObject) -> Result<void, never>
  method listPrompts() -> Result<string[], never>
  method getPrompt(name: string, arguments: JsonObject) -> Result<string, ExtensionError>
  method shutdown() -> Result<void, never>
}
