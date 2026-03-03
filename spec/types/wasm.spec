// Wasm/Extism plugin runtime types

type WasmPluginManifest {
  name              string          @readonly
  version           string          @readonly
  wasmPath          string
  entities          string[]        @optional
  edgeTypes         string[]        @optional
  validations       string[]        @optional
  hostFunctions     string[]        @optional
  peerDependencies  PeerDependency[] @optional
  sandboxPolicy     SandboxPolicy   @optional
  testable          boolean         @optional
}

type PeerDependency {
  plugin            string
  version           string
}

type HostFunctionBinding {
  name              string          @readonly
  inputSchema       string
  outputSchema      string
}

type SandboxPolicy {
  maxMemoryMb       integer         @optional
  maxExecutionMs    integer         @optional
  allowedDomains    string[]        @optional
  allowedPaths      string[]        @optional
  fileSystemAccess  string          @optional
  networkAccess     string          @optional
}

type WasmModuleCache {
  wasmHash          string          @readonly
  aotPath           string
  platform          string
  createdAt         timestamp
}

type PluginLifecycleState = discovered | loading | initialized | validating | generating | unloaded | failed
