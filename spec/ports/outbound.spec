// Outbound ports — interfaces the system requires from the outside world

use "types/config"
use "types/core"
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
  // The byte-level port under call_extension_exports (ADR 0013): its two
  // adapters are the component runtime (production) and the in-process
  // runtime (tests).
  method loadModule(extensionId: string, wasmPath: string) -> Result<void, ExtensionError>
  method callExport(extensionId: string, exportName: string, input: u8[]) -> Result<u8[], WasmTrapInfo>
  method setExecutionDeadline(extensionId: string, maxExecutionMs: integer) -> Result<void, never>
  method loadFailure(extensionId: string) -> Result<Diagnostic, never>
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

port Editor {
  direction outbound
  category  "api/lsp"
  // What the LSP's reaction to a change tells the editor (ADR 0043): its
  // two adapters are the tower-lsp client (production) and a recorder
  // (the LSP's tests). Each call returns once the editor has the message;
  // watch returns once the editor answered, and a refusal falls back to
  // the static watchers.
  method publish(path: string, diagnostics: Diagnostic[], version: integer @optional) -> Result<void, never>
  method watch(globs: string[]) -> Result<void, string>
  method log(level: string, message: string) -> Result<void, never>
  method progress(done: boolean, title: string @optional) -> Result<void, never>
  method refreshTokens() -> Result<void, never>
  verify integration "Editor contract is satisfied"
}
