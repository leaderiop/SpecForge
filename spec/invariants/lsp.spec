use "behaviors/lsp"
use "behaviors/zero-entity-lsp"
use "behaviors/incremental"
use "behaviors/formatting"
invariant lsp_response_latency "LSP Response Latency" {
  guarantee """
    Diagnostic updates MUST appear within 100ms of the user stopping
    typing. Completion, hover, go-to-definition, and formatting responses
    MUST return within 200ms for projects under 1000 entities.
  """
  enforced_by [live_diagnostics, hover_information, autocomplete_entity_ids, go_to_definition, find_all_references, prepare_rename, rename_entity_id, outline_view, workspace_symbol_search, provide_semantic_tokens, complete_field_names, complete_keywords, lsp_format_document, lsp_format_range, goto_import_definition, code_action_add_missing_import, code_action_create_entity_stub, code_actions_for_missing_verify, provide_extension_entity_hover, complete_extension_defined_keywords, provide_extension_entity_semantic_tokens, provide_extension_defined_lsp_icons, incremental_document_sync, handle_text_document_change]
  risk medium
}

invariant lsp_extension_reload_consistency "LSP Extension Reload Consistency" {
  guarantee """
    When extensions are added or removed while the LSP server is running,
    KindRegistry, FieldRegistry, and the semantic token legend MUST update
    atomically. No LSP request served between the start and end of the
    update MUST observe a partially-updated registry state. The LSP MUST
    NOT serve stale semantic tokens, completions, or hover information
    for entity kinds that were added or removed.
  """
  enforced_by [lsp_initialize, shared_incremental_pipeline]
  risk medium

  verify unit "adding an extension while LSP is running updates KindRegistry atomically"
  verify unit "removing an extension while LSP is running removes kinds from KindRegistry atomically"
  verify unit "semantic token legend reflects current extensions after reload"

}

invariant rename_atomicity "Rename Atomicity" {
  guarantee """
    A rename operation MUST update all files atomically — either all files
    are updated or none are. Partial updates MUST NOT persist.
  """
  enforced_by [rename_entity_id]
  risk high
}

invariant lsp_text_edit_non_overlapping "LSP TextEdit Non-Overlapping" {
  guarantee """
    TextEdit operations returned in a single LSP response MUST NOT have
    overlapping ranges. The LSP specification requires non-overlapping edits;
    overlapping edits cause undefined client behavior.
  """
  enforced_by [lsp_format_document, lsp_format_range, code_action_add_missing_import, code_action_create_entity_stub, code_actions_for_missing_verify]
  risk high
  verify property "no LSP response contains overlapping TextEdit ranges"
  verify unit "formatting response TextEdits are sorted and non-overlapping"
}
