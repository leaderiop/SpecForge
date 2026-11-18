requirement FEATURE-129 "Trace Routes" {
  category "Feature"
  tier     v2
  text     """
    `specforge trace` walks every edge both ways from an entity. On a requirement it lists dozens of rows and says
    nothing of what the requirement's domain means by its thread: its plans, tasks, commits, tests and the verdict
    on what they prove.

    An extension declares routes: from a start kind, ordered steps in the judged rules' selector grammar, each with
    annotations from fields, facts and judgements. The kernel's trace follows the route of the entity's kind and
    prints its thread on one line, or as a tree, on the CLI, MCP and the LSP; a kind with no route keeps the reach.
  """
}
