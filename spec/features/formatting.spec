// Formatting features

use behaviors/formatting

feature code_formatting "Code Formatting" {
  behaviors [
    format_spec_files, preserve_comments, check_formatting, show_formatting_diff,
    format_from_stdin, load_format_config, apply_format_rules, maintain_format_idempotency
  ]

  problem """
    .spec files accumulate inconsistent formatting over time — varying indentation,
    trailing whitespace, ragged alignment, and unsorted imports. This creates noisy
    diffs, wastes AI agent tokens on style variations, and slows code review. There
    is no way to enforce a canonical style across a project or in CI.
  """

  solution """
    A CST-based formatter that walks the tree-sitter concrete syntax tree, applies
    deterministic formatting rules, and writes canonically formatted output. The
    formatter operates purely on syntax (no semantic analysis required), supports
    check mode for CI, diff mode for review, and stdin mode for editor integration.
    Configuration is minimal (3 options) to maximize consistency across projects.
  """
}

feature lsp_format_on_save "LSP Format on Save" {
  behaviors [lsp_format_document, lsp_format_range, lsp_respect_editor_config]

  problem """
    Developers must manually format .spec files or rely on CLI commands after editing.
    Without textDocument/formatting support in the LSP server, format-on-save is not
    available and developers produce inconsistently formatted files.
  """

  solution """
    The LSP server implements textDocument/formatting and textDocument/rangeFormatting
    using the same formatting engine as the CLI. Format-on-save triggers full-document
    formatting within 50ms. Range formatting expands to block boundaries and produces
    results consistent with full-document formatting.
  """
}
