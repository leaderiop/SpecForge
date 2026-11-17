requirement FEATURE-009 "Code Formatting" {
  category "Feature"
  tier     v1
  text     """
    .spec files accumulate inconsistent formatting over time — varying indentation,
    trailing whitespace, ragged alignment, and unsorted imports. This creates noisy
    diffs, wastes AI agent tokens on style variations, and slows code review. There
    is no way to enforce a canonical style across a project or in CI.

    A CST-based formatter that walks the tree-sitter concrete syntax tree, applies
    deterministic formatting rules, and writes canonically formatted output. The
    formatter operates purely on syntax (no semantic analysis required), supports
    check mode for CI, diff mode for review, and stdin mode for editor integration.
    Configuration is minimal (3 options) to maximize consistency across projects.
  """
}

requirement FEATURE-061 "LSP Formatting" {
  category "Feature"
  tier     v1
  text     """
    Developers must manually format .spec files or rely on CLI commands after editing.
    Without textDocument/formatting support in the LSP server, format-on-save is not
    available and developers produce inconsistently formatted files.

    The LSP server implements textDocument/formatting and textDocument/rangeFormatting
    using the same formatting engine as the CLI. Format-on-save triggers full-document
    formatting within 50ms. Range formatting expands to block boundaries and produces
    results consistent with full-document formatting.
    LSP request cancellation ($/cancelRequest) is handled by the LSP protocol
    layer generically for all request types — formatting operations complete
    within the 50ms budget, making cancellation a protocol-level concern rather
    than a formatting-specific behavior.
    Extension-contributed formatting rules (ExtensionFormatRule) are applied
    in the LSP path through the same contribution registry as the CLI path.
  """
}
