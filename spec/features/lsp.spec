// LSP features

use "behaviors/lsp"
use "behaviors/zero-entity-lsp"
feature lsp_lifecycle "LSP Lifecycle" {

  problem """
    The LSP server must properly initialize with capabilities that
    reflect installed extensions, manage document lifecycle (open/close),
    and clean up resources on shutdown. Without explicit lifecycle
    management, the server may leak resources, advertise stale
    capabilities, or fail to track open documents for incremental
    compilation.
  """

  solution """
    LSP initialization derives all capabilities from loaded extensions
    via KindRegistry and FieldRegistry — the semantic token legend,
    completion triggers, and code action kinds are all extension-driven.
    Document open/close tracks which files participate in incremental
    compilation. Shutdown releases the in-memory graph, Wasm engines,
    and open document buffers.
  """
}

feature go_to_definition_and_references "Go-to-Definition and References" {

  problem """
    Navigating between entity declarations and their references across
    multiple .spec files is tedious without IDE support. Users must
    manually search for entity IDs and import paths.
  """

  solution """
    LSP provides Ctrl+click go-to-definition that jumps to entity
    declarations, import target files, and find-all-references that
    shows every usage site across all .spec files.
  """
}

feature hover_and_autocomplete "Hover and Autocomplete" {
  // Owned here (behaviors/lsp.spec):
  //   hover_information, autocomplete_entity_ids, complete_keywords
  // Bridge from extension_driven_lsp (features/zero-entity-core.spec):
  //   complete_field_names — queries FieldRegistry for extension-defined field names
  // Bridge references from zero-entity-core (behaviors/zero-entity-lsp.spec):
  //   provide_extension_entity_hover, complete_extension_defined_keywords

  problem """
    Users need quick access to entity details without navigating away
    from their current file. Typing entity IDs, field names, and
    keywords manually is slow and error-prone.
  """

  solution """
    LSP hover shows entity title, first string field summary, and
    reference count. Autocomplete suggests matching entity IDs with titles when
    typing in reference lists, valid field names inside entity blocks,
    and entity keywords at the file top level.
  """
}

feature rename_refactoring "Rename Refactoring" {

  problem """
    Renaming an entity ID requires updating every file that references
    it. Manual find-and-replace is risky — missed references become
    broken.
  """

  solution """
    LSP rename atomically updates the entity declaration and all
    references across all .spec files in a single operation.
  """
}

feature live_diagnostics "Live Diagnostics" {

  problem """
    Users need immediate feedback as they type, not after saving.
    Waiting for a manual specforge check breaks the flow. Full document
    sync wastes bandwidth on every keystroke.
  """

  solution """
    LSP provides real-time diagnostics using the shared incremental
    compilation pipeline with incremental document sync. Error squiggles
    appear within 100ms of the user stopping typing.
  """
}

feature semantic_tokens "Semantic Tokens" {
  // Owned here (behaviors/lsp.spec):
  //   provide_semantic_tokens
  // Bridge references from zero-entity-core (behaviors/zero-entity-lsp.spec):
  //   provide_extension_entity_semantic_tokens
  // Bridge from behaviors/lsp.spec: load_extension_grammars_for_highlighting
  //   provides grammar-based highlighting data for extension-defined grammars

  problem """
    Without semantic understanding, editors can only provide basic
    syntax highlighting via TextMate grammars. Entity keywords defined
    by extensions are indistinguishable from plain identifiers, and
    field names lack contextual coloring.
  """

  solution """
    LSP provides semantic tokens that classify entity keywords using
    extension-defined token types from the KindRegistry. Structural
    keywords, entity IDs, triple-quoted strings, and extension-defined
    fields each receive distinct token classifications for rich
    highlighting.
  """
}

feature code_actions "Code Actions" {
  // Owned: code_action_add_missing_import
  // Bridge: code_actions_for_missing_verify, code_action_create_entity_stub
  //   (owned by extension_driven_code_actions in features/zero-entity-core.spec)

  problem """
    Untested entities are easy to overlook. Adding verify declarations
    manually is repetitive. Unresolved references require manual import
    addition or entity creation.
  """

  solution """
    LSP offers three code actions: (1) on unresolved use references, add
    the missing import statement; (2) on untested testable entities (any
    entity kind with testable=true in its extension manifest), add verify
    stub declarations to the .spec file; (3) on unresolved entity
    references, create an entity stub in the appropriate file.
  """
}

feature outline_and_symbol_search "Outline and Symbol Search" {
  // Owned here (behaviors/lsp.spec):
  //   outline_view, workspace_symbol_search
  // Bridge references from zero-entity-core (behaviors/zero-entity-lsp.spec):
  //   provide_extension_defined_lsp_icons

  problem """
    Large .spec files need a structural overview. Finding entities
    across a workspace requires scanning multiple files.
  """

  solution """
    LSP outline view shows a tree of all entities in the current file
    with extension-driven SymbolKind icons from the KindRegistry.
    Workspace symbol search finds entities by ID prefix or title
    fragment across all .spec files.
  """
}

// LSP formatting features (format on save, range format) are in features/formatting.spec
