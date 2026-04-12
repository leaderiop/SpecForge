invariant lsp_response_latency "LSP Response Latency" {
  guarantee """
    Diagnostic updates MUST appear within 100ms of the user stopping
    typing. Completion, hover, go-to-definition, and formatting responses
    MUST return within 200ms for projects under 1000 entities.
  """
  risk medium

  verify property "LSP Response Latency guarantee holds"
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
  risk high

  verify property "Rename Atomicity guarantee holds"
}

invariant lsp_text_edit_non_overlapping "LSP TextEdit Non-Overlapping" {
  guarantee """
    TextEdit operations returned in a single LSP response MUST NOT have
    overlapping ranges. The LSP specification requires non-overlapping edits;
    overlapping edits cause undefined client behavior.
  """
  risk high
  verify property "no LSP response contains overlapping TextEdit ranges"
  verify unit "formatting response TextEdits are sorted and non-overlapping"
}
