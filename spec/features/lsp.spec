requirement FEATURE-062 "LSP Lifecycle" {
  category "Feature"
  tier     v1
  text     """
    The LSP server must properly initialize with capabilities that
    reflect installed extensions, manage document lifecycle (open/close),
    and clean up resources on shutdown. Without explicit lifecycle
    management, the server may leak resources, advertise stale
    capabilities, or fail to track open documents for incremental
    compilation.

    LSP initialization derives all capabilities from loaded extensions
    via KindRegistry and FieldRegistry — the semantic token legend,
    completion triggers, and code action kinds are all extension-driven.
    Document open/close tracks which files participate in incremental
    compilation. Shutdown releases the in-memory graph, Wasm engines,
    and open document buffers.
  """
}

requirement FEATURE-046 "Go-to-Definition and References" {
  category "Feature"
  tier     v1
  text     """
    Navigating between entity declarations and their references across
    multiple .spec files is tedious without IDE support. Users must
    manually search for entity IDs and import paths.

    LSP provides Ctrl+click go-to-definition that jumps to entity
    declarations, import target files, and find-all-references that
    shows every usage site across all .spec files.
  """
}

requirement FEATURE-049 "Hover and Autocomplete" {
  category "Feature"
  tier     v1
  text     """
    Users need quick access to entity details without navigating away
    from their current file. Typing entity IDs, field names, and
    keywords manually is slow and error-prone.

    LSP hover shows entity title, headline summary, coverage, reference
    counts and the diagnostics about the entity. Autocomplete suggests matching entity IDs with titles when
    typing in reference lists, valid field names inside entity blocks,
    and entity keywords at the file top level.
  """
}

requirement FEATURE-111 "Rename Refactoring" {
  category "Feature"
  tier     v1
  text     """
    Renaming an entity ID requires updating every file that references
    it. Manual find-and-replace is risky — missed references become
    broken.

    LSP rename atomically updates the entity declaration and all
    references across all .spec files in a single operation.
  """
}

requirement FEATURE-059 "Live Diagnostics" {
  category "Feature"
  tier     v1
  text     """
    Users need immediate feedback as they type, not after saving.
    Waiting for a manual specforge check breaks the flow. Full document
    sync wastes bandwidth on every keystroke.

    LSP provides real-time diagnostics using the shared incremental
    compilation pipeline with incremental document sync. Error squiggles
    appear within 100ms of the user stopping typing.
  """
}

requirement FEATURE-121 "Semantic Tokens" {
  category "Feature"
  tier     v1
  text     """
    Without semantic understanding, editors can only provide basic
    syntax highlighting via TextMate grammars. Entity keywords defined
    by extensions are indistinguishable from plain identifiers, and
    field names lack contextual coloring.

    LSP provides semantic tokens that classify entity keywords using
    extension-defined token types from the KindRegistry. Structural
    keywords, entity IDs, triple-quoted strings, and extension-defined
    fields each receive distinct token classifications for rich
    highlighting.
  """
}

requirement FEATURE-008 "Code Actions" {
  category "Feature"
  tier     v1
  text     """
    Untested entities are easy to overlook. Adding verify declarations
    manually is repetitive. Unresolved references require manual entity
    creation.

    LSP offers code actions: on untested testable entities (any entity
    kind with testable=true in its extension manifest), add verify stub
    declarations to the .spec file; on unresolved entity references,
    replace a misspelled id with the close match or create an entity stub
    in the current file. References resolve across the project without
    `use`, so no import action is needed.
  """
}

requirement FEATURE-078 "Outline and Symbol Search" {
  category "Feature"
  tier     v1
  text     """
    Large .spec files need a structural overview. Finding entities
    across a workspace requires scanning multiple files.

    LSP outline view shows a tree of all entities in the current file
    with extension-driven SymbolKind icons from the KindRegistry.
    Workspace symbol search finds entities by ID prefix or title
    fragment across all .spec files.
  """
}
