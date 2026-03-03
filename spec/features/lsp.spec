// LSP features

use behaviors/lsp

feature go_to_definition_and_references "Go-to-Definition and References" {
  behaviors [go_to_definition, find_all_references, goto_import_definition]

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
  behaviors [hover_information, autocomplete_entity_ids, complete_field_names, complete_keywords]

  problem """
    Users need quick access to entity details without navigating away
    from their current file. Typing entity IDs, field names, and
    keywords manually is slow and error-prone.
  """

  solution """
    LSP hover shows entity title, contract/guarantee text, and reference
    count. Autocomplete suggests matching entity IDs with titles when
    typing in reference lists, valid field names inside entity blocks,
    and entity keywords at the file top level.
  """
}

feature rename_refactoring "Rename Refactoring" {
  behaviors [rename_entity_id]

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

feature live_diagnostics_feature "Live Diagnostics" {
  behaviors [live_diagnostics, shared_incremental_pipeline, provide_semantic_tokens, incremental_document_sync]

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

feature code_actions "Code Actions" {
  behaviors [code_actions_for_missing_tests, code_action_add_missing_import, code_action_create_entity_stub]

  problem """
    Untested behaviors are easy to overlook. Generating test stubs
    manually from verify statements is repetitive boilerplate.
    Unresolved references require manual import addition or entity
    creation.
  """

  solution """
    LSP offers code actions on untested behaviors to generate test
    stubs for the configured language with behavior ID and verify
    descriptions pre-filled. On unresolved references, the LSP offers
    quick-fixes to add the missing import or create an entity stub.
  """
}

feature outline_and_symbol_search "Outline and Symbol Search" {
  behaviors [outline_view, workspace_symbol_search]

  problem """
    Large .spec files need a structural overview. Finding entities
    across a workspace requires scanning multiple files.
  """

  solution """
    LSP outline view shows a tree of all entities in the current file.
    Workspace symbol search finds entities by ID prefix or title
    fragment across all .spec files.
  """
}
