// Logical Data Model features

use "behaviors/model"

feature logical_data_model "Logical Data Model Visualization" {

  problem """
    Users and AI agents have no way to visualize the schema-level structure
    of a SpecForge project — which entity kinds exist, what fields they have,
    and how they relate to each other via edge types. The existing specforge
    schema command outputs raw JSON Schema for programmatic consumption, not
    a human- or agent-readable data model. The existing specforge export
    --format=dot shows instance-level graphs, not schema structure.
  """

  solution """
    specforge model renders the schema-level meta-model as a logical data
    model in five formats: Markdown (default, LLM-friendly), Mermaid
    erDiagram (visual, renders in GitHub/IDEs), DOT (Graphviz with
    HTML-like labels), JSON (ERD-oriented, machine-readable), and DBML
    (compatible with dbdiagram.io). The command reads the GraphProtocolSchema
    assembled from extension registries and transforms it into an
    ERD-oriented intermediate representation. Cardinality is inferred
    from field types (reference -> N:1, reference_list -> N:M). Three
    field detail tiers (none, keys, all) control verbosity. Entities
    can be grouped by extension or listed flat. Filters by extension,
    entity kind, or kind-level depth scope the output.
  """
}

feature model_multi_format "Multi-Format Model Output" {

  problem """
    Different consumers need different output formats: terminals need
    readable text, GitHub needs Mermaid, Graphviz users need DOT, ERD
    tools need JSON or DBML, and LLMs need structured prose.
  """

  solution """
    Five format renderers share a single ModelIntermediate IR. Markdown
    is the default for terminal and LLM use. Mermaid erDiagram renders
    natively in GitHub markdown and IDE previews. DOT produces Graphviz
    output with HTML-like labels for rich visual rendering. JSON provides
    an ERD-oriented schema distinct from specforge schema. DBML is
    compatible with dbdiagram.io for database diagram tooling.
  """
}

feature model_filtering "Model Filtering and Scoping" {

  problem """
    With 23+ entity kinds across 4 extensions, the full model is noisy.
    Users need to focus on specific extensions, entity kinds, or
    neighborhoods of the kind graph.
  """

  solution """
    Three filtering dimensions: --extension narrows to a single
    extension's entity kinds. --kinds filters to specific entity kind
    names. --root with --depth builds a kind-level adjacency graph and
    includes only kinds within N hops of the root kind. Filters compose
    as intersection. Relationships where either endpoint is filtered out
    are automatically excluded.
  """
}

feature model_agent_access "Model Access for AI Agents" {

  problem """
    AI agents consuming spec graphs need to understand the schema structure
    to make correct first-attempt modifications. The raw GraphProtocolSchema
    JSON contains compiler metadata (testable, singleton, incremental) that
    is noise for agents trying to understand entity relationships.
  """

  solution """
    The specforge.model MCP tool exposes the logical data model in all
    five formats with the same filtering options as the CLI. The default
    Markdown format includes a framing preamble that explains entity kinds,
    fields, and relationships so an LLM encountering the model for the
    first time has context. The ERD JSON format strips compiler metadata
    and focuses on entities-as-tables, fields-as-columns, and
    relationships-with-cardinality.
  """
}
