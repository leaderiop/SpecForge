// @specforge/markdown-renderer extension features
// Markdown documentation is a renderer contribution, not a core feature.
// Vision: "SpecForge does not produce code, configuration, documentation,
// or any output artifact." Markdown rendering is an extension responsibility.

use "extensions/markdown-renderer/behaviors"
feature markdown_documentation_generation "Markdown Documentation Generation" {
  behaviors [render_markdown_documentation, render_index_files, selective_render_by_entity_type]

  problem """
    Stakeholders who don't work with .spec files need readable
    documentation. Generated docs must stay in sync with the spec
    automatically without manual maintenance.
  """

  solution """
    The @specforge/markdown-renderer extension contributes a renderer
    that traverses the graph and produces .md files grouped by entity
    kind, with cross-reference links, index files, and support for
    entity kind filtering. Invoked via specforge render markdown.
  """
}
