// Wasm/Extism package runtime types

type PackageManifest {
  name              string          @readonly
  version           string          @readonly
  wasmPath          string
  contributes       PackageContributions @optional
  entities          string[]        @optional
  edgeTypes         string[]        @optional
  validations       string[]        @optional
  hostFunctions     string[]        @optional
  peerDependencies  PeerDependency[] @optional
  sandboxPolicy     SandboxPolicy   @optional
  testable          boolean         @optional
}

type PackageContributions {
  entities          boolean         @optional
  validators        boolean         @optional
  generators        boolean         @optional
  providers         boolean         @optional
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

// ── Entity Enhancement Types ─────────────────────────────────

type FieldEnhancement {
  targetEntity      string          @readonly
  fieldName         string          @readonly
  fieldType         EnhancedFieldType
  required          boolean         @optional
  description       string          @optional
}

type EnhancedFieldType = string_type | integer_type | bool_type | enum_type | string_list_type | reference_type | reference_list_type

type EnumFieldType {
  values            string[]
}

type ReferenceFieldType {
  edgeLabel         string
  targetKind        string          @optional
}

type DynamicEdgeType {
  label             string          @readonly
  sourcePlugin      string          @readonly
  soft              boolean         @optional
}

type EnhancementConflict {
  entityKind        string          @readonly
  fieldName         string          @readonly
  firstPlugin       string          @readonly
  secondPlugin      string          @readonly
  resolution        ConflictResolution
}

type ConflictResolution = unresolved | explicit_override | load_order | namespaced

type EnhancementPolicy = error | priority | namespace

// ── Query Extension Types ───────────────────────────────────

type QueryExtension {
  kind              QueryFileKind   @readonly
  patterns          string
}

type QueryFileKind = highlights | folds | indents | injections

// ── Package Lifecycle Types ─────────────────────────────────

type PackageInstallResult {
  packageName       string          @readonly
  version           string          @readonly
  source            PackageSource
  wasmSize          integer
  aotCompiled       boolean
  installedPath     string
}

type PackageSource = registry | local | git

type WasmTrapInfo {
  kind              string          @readonly
  message           string          @readonly
  exportName        string          @optional
  memoryAddress     string          @optional
  packageName       string
}

// ── Lock File Types ──────────────────────────────────────────

type LockFileEntry {
  packageName       string          @readonly
  version           string          @readonly
  source            PackageSource
  wasmHash          string          @readonly
  resolvedAt        timestamp
}
