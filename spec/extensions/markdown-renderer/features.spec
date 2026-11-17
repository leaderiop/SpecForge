requirement FEATURE-063 "Markdown Documentation Generation" {
  category "Feature"
  tier     v2
  text     """
    Stakeholders who don't work with .spec files need readable
    documentation. Generated docs must stay in sync with the spec
    automatically without manual maintenance.

    The @specforge/markdown-renderer extension contributes a renderer
    that traverses the graph and produces .md files grouped by entity
    kind, with cross-reference links, index files, and support for
    entity kind filtering. Invoked via specforge render markdown.
  """
}
