use behaviors/lsp
use behaviors/zero-entity-lsp
use behaviors/incremental
use behaviors/formatting

invariant lsp_response_latency "LSP Response Latency" {
  guarantee """
    Diagnostic updates MUST appear within 100ms of the user stopping
    typing. Completion, hover, go-to-definition, and formatting responses
    MUST return within 200ms for projects under 1000 entities.
  """
  enforced_by [live_diagnostics, hover_information, autocomplete_entity_ids, go_to_definition, find_all_references, prepare_rename, rename_entity_id, outline_view, workspace_symbol_search, provide_semantic_tokens, complete_field_names, complete_keywords, lsp_format_document, lsp_format_range, goto_import_definition, code_action_add_missing_import, code_action_create_entity_stub, code_actions_for_missing_tests, provide_extension_entity_hover, complete_extension_defined_keywords, provide_extension_entity_semantic_tokens, provide_extension_defined_lsp_icons]
  risk medium
}

invariant rename_atomicity "Rename Atomicity" {
  guarantee """
    A rename operation MUST update all files atomically — either all files
    are updated or none are. Partial updates MUST NOT persist.
  """
  enforced_by [rename_entity_id]
  risk high
}
