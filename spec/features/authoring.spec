requirement FEATURE-131 "Typed Edits" {
  category "Feature"
  tier     v2
  text     """
    Every change to a .spec file is a hand edit: SpecForge has no operation that
    creates an entity, sets or removes a field, adds or removes a reference, or
    deletes an entity. Only rename writes entity text, and the CLI has no rename.
    A person or an agent guesses the field order, the quoting and the alignment,
    and learns of a mistake from specforge check afterwards.

    One operation plans a list of typed edits (create, set, unset, retitle, link,
    unlink, add or remove a verify obligation, delete) and writes them. Each
    value is written in the form its field's declaration decides and each
    touched block is laid out by the formatter. Nothing is written unless the
    edited project compiles with no error it did not already have (E080); the
    files are then written as one project change. The CLI gets new, set,
    unset, link, unlink and delete; MCP gets specforge.edit.
  """
}
