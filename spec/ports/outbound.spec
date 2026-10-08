// Outbound ports — interfaces the system requires from the outside world

use "types/config"
use "types/core"
use "types/diagnostics"
use "types/errors"
use "types/wasm"
use "types/zero-entity-core"

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
  // The transport to a package registry (ADR 0044): it chooses no registry
  // and checks no reply. Its adapters are the HTTP client (production) and
  // the in-memory client (tests); both keep one contract suite.
  method versions(name: PackageName, registry: RegistryConfig) -> Result<string[], RegistryError>
  method metadata(name: PackageName, version: string, registry: RegistryConfig) -> Result<PackageMetadata, RegistryError>
  method download(wasmUrl: string) -> Result<u8[], RegistryError>
  method search(query: string, registry: RegistryConfig) -> Result<SearchHit[], RegistryError>
  method publish(wasm: u8[], declaration: ExtensionDeclaration, manifest: string, signature: string, registry: RegistryConfig, credential: RegistryCredential) -> Result<string, RegistryError>
  method authenticate(registry: RegistryConfig, credential: RegistryCredential) -> Result<string, RegistryError>
  verify integration "RegistryClient contract is satisfied"
}
