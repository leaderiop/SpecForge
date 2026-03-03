// Graph types — in-memory graph data shapes

use types/core

type Graph {
  nodes      Node[]
  edges      Edge[]
  fileIndex  FileIndex
}

type Node {
  id         EntityId    @readonly  @unique
  kind       EntityKind
  title      string
  fields     FieldMap
  sourceSpan SourceSpan  @readonly
}

type Edge {
  source     EntityId    @readonly
  target     EntityId    @readonly
  edgeType   EdgeType
}

type EdgeType = references | implements | produces | consumes
             | uses_type | uses_port | enforces | imports | links_to
             | traces_to | bundles | built_from | depends_on | provides
             | defines_port | schedules | protects | constrains
             | mitigates | shaped_by

type FileIndex {
  files      FileEntry[]
}

type FileEntry {
  path       string     @readonly
  entities   EntityId[]
  imports    string[]
}

type Subgraph {
  nodeIds    EntityId[]
  edgeIds    Edge[]
}

type TraceChain {
  root       EntityId
  links      TraceLink[]
}

type TraceLink {
  from       EntityId
  to         EntityId
  edgeType   EdgeType
  depth      integer
}
