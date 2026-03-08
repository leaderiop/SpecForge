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
  kind        SymbolKind
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

type CodeActionKind = QuickFix | Refactor | Source | SourceOrganizeImports

type CodeAction {
  title           string       @readonly
  kind            CodeActionKind
  diagnostic_code string       @optional
  edits           TextEdit[]
  is_preferred    boolean      @optional
}

type SemanticToken {
  line            integer      @readonly
  start_col       integer      @readonly
  length          integer      @readonly
  token_type      string       @readonly
  modifiers       string[]     @optional
}

type WorkspaceSymbolEntry {
  name            string       @readonly
  kind            SymbolKind
  location        SourceSpan
  container       string       @optional
}

type SymbolKind = file | module | namespace | class | method | property | field | constructor | enum | interface | function | variable | constant | string | number | boolean | array | object | key | enum_member | struct | event | operator | type_parameter
