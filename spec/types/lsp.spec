// LSP-specific types for hover, completion, and document symbols

use types/core

type ContentChangeEvent {
  start_line   integer
  start_col    integer
  end_line     integer
  end_col      integer
  text         string
}

type CompletionItem {
  label       string       @readonly
  detail      string       @optional
  kind        string
  insert_text string       @optional
}

type HoverContent {
  entity_kind      string
  title            string
  source_extension string
  testable         boolean        @optional
  summary          string         @optional
  incoming_reference_count  integer
  outgoing_reference_count  integer
}

type DocumentSymbolEntry {
  name        string       @readonly
  kind        string
  range       SourceSpan
  children    DocumentSymbolEntry[] @optional
}

type SemanticTokenLegendEntry {
  token_type       string       @readonly
  token_modifiers  string[]     @optional
  source_extension string       @optional
}

type WorkspaceEditResult {
  edits            TextEdit[]
  document_count   integer
}
