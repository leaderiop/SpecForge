requirement FEATURE-007 "Generic Value Checks" {
  category "Feature"
  tier     v2
  text     """
    A project's values carry dates, versions, URLs, paths and identifiers,
    and nothing says they are well formed or point at something real unless
    each domain writes the same checks again.

    @specforge/checks holds the checkers a cross-industry standard decides,
    in families a setting switches: files, formats, digits, links, git,
    structured, media and registries. A domain pack adds its own on the same kit (ADR 0338).
  """
}
