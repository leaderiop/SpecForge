// Federation types — cross-project reference and graph merging

use "types/graph"
type ProjectDependency {
  name           string          @readonly
  // Relative to the directory containing specforge.json
  path           string          @optional
  // registry is resolved at CLI level (specforge add/install),
  // not by the compiler core. The compiler only uses local paths.
  registry       string          @optional
  // Semver constraint when from registry, git ref when from git, ignored for local path
  version        string          @optional
}

type FederatedEntityId {
  project        string          @readonly
  entity_id      string          @readonly
}
// Project identifiers: lowercase alphanumeric with hyphens, 2-64 chars
// The qualified form is "project::entity_id"

type FederationConfig {
  dependencies   ProjectDependency[]
}

type FederationMetadata {
  merged_at             timestamp
  source_projects       string[]
  schema_compatible     boolean
}

type FederatedGraph {
  local                 Graph           @readonly
  remote_graphs         Graph[]         @optional
  remote_projects       ProjectDependency[]
  cross_edges           Edge[]
  schema_version        string
  federation_metadata   FederationMetadata
}

type FederatedExportConfig {
  include_projects   string[]        @optional
  exclude_projects   string[]        @optional
  @doc "Default: unlimited (all transitive dependencies)"
  depth              integer         @optional
  format             "context" | "graph" | "brief"  @optional
  max_tokens         integer         @optional
}
