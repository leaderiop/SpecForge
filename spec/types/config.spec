// Configuration types — compiler and plugin configuration

type CompilerConfig {
  name         string     @readonly
  infix        string     @readonly
  version      string     @readonly
  plugins      string[]
  providers    ProviderConfig[]  @optional
  personas     PersonaConfig[]   @optional
  surfaces     SurfaceConfig[]   @optional
  testDirs     string[]          @optional
  coverage     CoverageConfig    @optional
  genTargets   GenConfig[]       @optional
}

type ProviderConfig {
  alias      string    @unique
  package    string
  config     ProviderSettings
}

type ProviderSettings {
  entries    ConfigEntry[]
}

type ConfigEntry {
  key        string
  value      string
}

type PersonaConfig {
  id          string  @unique
  displayName string
  description string  @optional
}

type SurfaceConfig {
  id          string  @unique
  displayName string
  surfaceType string  @optional
}

type CoverageConfig {
  threshold              integer
  reports                string[]      @optional
  requireViolationTests  boolean       @optional
  failOnUnknownIds       boolean       @optional
}

type GenConfig {
  language     string
  outDir       string
  resultStyle  string       @optional
  readonly     boolean      @optional
  naming       string       @optional
  testPlugin   string       @optional
  wasmPath     string       @optional
}

type PluginManifest {
  name              string              @readonly
  version           string              @readonly
  wasmPath          string
  entities          string[]
  prefixes          string[]
  edgeTypes         string[]
  validations       string[]
  hostFunctions     string[]            @optional
  peerDependencies  string[]            @optional
  sandboxPolicy     string              @optional
  testable          boolean             @optional
}
