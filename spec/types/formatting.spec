// Formatting types — data shapes for the code formatter

type FormatConfig {
  indent_width    integer
  use_tabs        boolean
  max_width       integer
  extension_rules JsonValue   @optional
}

type FormatRule = IndentRule | SpacingRule | AlignmentRule | WrappingRule | NewlineRule | CommentRule | ImportRule | StringRule | ExtensionFormatRule

type IndentRule {
  scope          string
  style          string
}

type SpacingRule {
  context        string
  spaces         integer
}

type AlignmentRule {
  target         string
  column         integer   @optional
}

type WrappingRule {
  max_width      integer
  wrap_style     string
}

type NewlineRule {
  context        string
  count          integer
}

type CommentRule {
  style          string
  preserve_inline boolean
}

type ImportRule {
  sort_order     string
  grouping       string    @optional
}

type StringRule {
  quote_style       string
  multiline_string  boolean   @optional
}

// Extensions can register custom formatting rules via their manifest.
// The core formatter delegates to extension-provided rules for entity-specific
// formatting concerns (e.g., field ordering within extension-defined blocks).
type ExtensionFormatRule {
  extension_id   string
  rule_name      string
  config         JsonValue   @optional
}

type FormatDiff "Per-file formatting diff with change statistics" {
  file_path    string    @readonly
  before       string
  after        string
  insertions   integer
  deletions    integer
}
