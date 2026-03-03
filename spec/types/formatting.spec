// Formatting types — data shapes for the code formatter

type FormatConfig {
  indent_width  integer
  use_tabs      boolean
  max_width     integer
}

type FormatRule = IndentRule | SpacingRule | AlignmentRule | WrappingRule | NewlineRule | CommentRule | ImportRule | StringRule

type FormatDiff {
  path       string
  before     string
  after      string
  changed    boolean
}
