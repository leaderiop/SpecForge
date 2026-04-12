// Outbound ports — interfaces the system requires from the outside world

use "types/core"
use "types/config"
use "types/diagnostics"
use "types/errors"
use "types/wasm"
port FileSystem {
  direction outbound
  category  "io/filesystem"

  method readFile(path: string) -> Result<string, EmitterError>
  method writeFile(path: string, content: string) -> Result<void, EmitterError>
  method listFiles(pattern: string) -> Result<string[], EmitterError>
  method watchFiles(patterns: string[]) -> Result<void, EmitterError>
  method exists(path: string) -> Result<boolean, never>
  method atomicWrite(path: string, content: string) -> Result<void, EmitterError>
  method rename(from: string, to: string) -> Result<void, EmitterError>
  verify integration "FileSystem contract is satisfied"
}

port SourceParser {
  direction outbound
  category  "compiler/parser"

  method parseSource(content: string, path: string) -> Result<SpecFile, ParseError>
  method parseIncremental(content: string, path: string, previousTree: string) -> Result<SpecFile, ParseError>
  verify integration "SourceParser contract is satisfied"
}

port GraphSerializer {
  direction outbound
  category  "io/output"

  method serializeJson(data: JsonValue) -> Result<string, EmitterError>
  method serializeDot(data: JsonValue) -> Result<string, EmitterError>
  method writeOutput(path: string, content: string) -> Result<void, EmitterError>
  verify integration "GraphSerializer contract is satisfied"
}

port RefValidator {
  direction outbound
  category  "validation/refs"

  method validateScheme(scheme: string) -> Result<boolean, never>
  method validateKind(scheme: string, kind: string) -> Result<boolean, never>
  method validateIdentifier(scheme: string, kind: string, identifier: string) -> Result<boolean, ValidationError>
  method resolveUrl(scheme: string, kind: string, identifier: string) -> Result<string, EmitterError>
  verify integration "RefValidator contract is satisfied"
}

port WasmRuntime {
  direction outbound
  category  "runtime/wasm"

  method loadModule(wasmPath: string) -> Result<string, ExtensionError>
  method callExport(extensionId: string, exportName: string, input: JsonValue) -> Result<JsonValue, ExtensionError>
  method registerHostFunction(name: string, handler: string) -> Result<void, ExtensionError>
  method aotCompile(wasmPath: string, cachePath: string) -> Result<string, ExtensionError>
  method unloadModule(extensionId: string) -> Result<void, ExtensionError>
  method getMemoryUsage(extensionId: string) -> Result<integer, never>
  method discoverExtensions(source: string, extensionSpec: string) -> Result<string[], ExtensionError>
  method getCacheStatus(extensionId: string) -> Result<string, never>
  method loadGrammar(contribution: GrammarContribution) -> Result<void, GrammarError>
  method callBodyParser(export_name: string, raw_body: string) -> Result<FieldMap, BodyParserError>
  method getGrammarCacheStatus(hash: string) -> Result<GrammarCacheEntry, GrammarError>
  verify integration "WasmRuntime contract is satisfied"
}

port RegistryClient {
  direction outbound
  category  "io/registry"

  method fetchExtension(registryUrl: string, name: string) -> Result<RegistryResponse, RegistryError>
  method fetchVersion(registryUrl: string, name: string, version: string) -> Result<RegistryResponse, RegistryError>
  method downloadWasm(registryUrl: string, name: string, version: string) -> Result<string, RegistryError>
  method search(registryUrl: string, query: string) -> Result<RegistrySearchResult, RegistryError>
  method publish(registryUrl: string, name: string, wasmPath: string, manifest: string) -> Result<void, RegistryError>
  method authenticate(registryUrl: string, credential: RegistryCredential) -> Result<string, RegistryError>
  method validateCredential(credential: RegistryCredential) -> Result<boolean, RegistryError>
  verify integration "RegistryClient contract is satisfied"
}

