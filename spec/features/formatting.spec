// Formatting features
//
// Fix #8 — Why formatting lives in core, not an extension (P7: extensions over builtins):
// Formatting uses a hybrid model. Core owns generic CST formatting — indentation,
// spacing, alignment, wrapping, blank lines, comments, imports, and string literals.
// This is purely syntactic (no domain knowledge, no entity awareness) and must
// complete in <50ms for the LSP path (P8: seconds to value); Wasm round-trips
// would violate that budget. Extensions can contribute entity-specific formatting
// rules via ExtensionFormatRule (e.g., field ordering within extension-defined
// blocks). Core delegates to these at format time through the contribution registry.
// Because core never inspects entity kinds or fields, it satisfies P7's real test:
// "does this require a compiler change when a new domain appears?" — it does not.

use behaviors/formatting
use types/core
use types/formatting

feature code_formatting "Code Formatting" {
  behaviors [
    format_spec_files, preserve_comments, check_formatting, show_formatting_diff,
    format_from_stdin, load_format_config, apply_format_rules, maintain_format_idempotency,
    format_with_parse_errors, discover_format_targets
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

feature lsp_formatting "LSP Formatting" {
  behaviors [lsp_format_document, lsp_format_range, lsp_respect_editor_config, format_with_parse_errors, load_format_config, apply_format_rules, maintain_format_idempotency, preserve_comments]

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
    LSP request cancellation ($/cancelRequest) is handled by the LSP protocol
    layer generically for all request types — formatting operations complete
    within the 50ms budget, making cancellation a protocol-level concern rather
    than a formatting-specific behavior.
    Extension-contributed formatting rules (ExtensionFormatRule) are applied
    in the LSP path through the same contribution registry as the CLI path.
  """
}
