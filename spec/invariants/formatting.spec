// Formatting invariants — guarantees the formatter must always uphold

use behaviors/formatting
use behaviors/mcp-operations
use types/formatting

invariant formatting_idempotency "Formatting Idempotency" {
  guarantee """
    Applying the formatter to an already-formatted file MUST produce identical output.
    Formally: format(format(x)) == format(x) for all valid .spec inputs. Any
    violation of this invariant is a P0 bug.
  """
  enforced_by [format_spec_files, apply_format_rules, maintain_format_idempotency, check_formatting, show_formatting_diff, format_from_stdin, lsp_format_document, lsp_format_range, format_with_parse_errors, provide_mcp_format_tool]
  risk high

  verify property "formatting an already-formatted file produces identical output"
  verify property "random valid .spec files satisfy format(format(x)) == format(x)"

}

invariant comment_preservation "Comment Preservation" {
  guarantee """
    The formatter MUST NOT lose, relocate, or alter any comments in a .spec file.
    Every comment present in the input MUST appear in the output at the correct
    attachment point. Comment content MUST NOT be modified.
  """
  enforced_by [format_spec_files, preserve_comments, apply_format_rules, format_from_stdin, check_formatting, show_formatting_diff, format_with_parse_errors, lsp_format_document, lsp_format_range]
  risk high

  verify property "every comment in input appears in formatted output"
  verify unit "trailing comments remain attached to their preceding node"
  verify unit "leading comments remain attached to their following node"

}

invariant formatting_consistency "Formatting Consistency" {
  guarantee """
    Two semantically identical .spec files MUST produce identical formatted output
    regardless of their original whitespace, indentation, or blank line patterns.
    The formatter MUST be a convergent function: all style variations converge
    to the same canonical form. Parse-error regions are excluded from this
    guarantee: format_with_parse_errors preserves unparseable text verbatim,
    so two files with differently-shaped parse errors MAY produce different output.
  """
  enforced_by [format_spec_files, apply_format_rules, check_formatting, show_formatting_diff, format_from_stdin, lsp_format_document, lsp_format_range, format_with_parse_errors]
  risk medium

  verify property "two files differing only in whitespace produce identical formatted output"
  verify unit "tab-indented and space-indented inputs produce the same output"

}

invariant config_defaults_valid "Config Defaults Valid" {
  guarantee """
    The default FormatConfig values (indent_width=2, use_tabs=false, max_width=100)
    MUST themselves form a valid configuration. Falling back to defaults MUST
    always produce a usable formatter configuration, never an error state.
  """
  enforced_by [load_format_config, lsp_respect_editor_config]
  risk low

  verify unit "default FormatConfig passes validation"
  verify unit "fallback from invalid config produces usable FormatConfig"

}

invariant discover_completeness "Discovery Completeness" {
  guarantee """
    When specforge format discovers .spec files under a directory without
    explicit path arguments, it MUST find all .spec files that exist under
    spec_root (as defined in specforge.json) at the time discovery runs.
    Every valid .spec file under spec_root MUST either be formatted or
    explicitly excluded via configuration.
  """

  enforced_by [discover_format_targets]

  risk medium

  verify unit "all .spec files under spec_root are discovered"
  verify unit "no .spec files are silently skipped"

}

invariant format_rule_determinism "Format Rule Determinism" {
  guarantee """
    Given the same input content and the same FormatConfig, the formatter MUST
    produce byte-identical output regardless of execution environment, platform,
    or invocation method (CLI, LSP, stdin). This is stronger than idempotency:
    idempotency guarantees stability across re-application, while determinism
    guarantees stability across independent invocations with identical inputs.
  """
  enforced_by [apply_format_rules, format_spec_files, format_from_stdin, lsp_format_document, lsp_format_range, check_formatting, show_formatting_diff, format_with_parse_errors]
  risk medium

  verify property "same input and config produce identical output across CLI and LSP"
  verify property "same input and config produce identical output across platforms"

}

invariant format_rule_priority "Format Rule Application Order" {
  guarantee """
    When multiple format rules apply to the same whitespace region,
    rules MUST be applied in a deterministic priority order:
    indent > newline > spacing > alignment > wrapping > comment > import > string.
    All compilers MUST produce identical output for the same input and config.
  """
  enforced_by [apply_format_rules]
  risk medium

  verify unit "indent rule takes precedence over spacing rule on same whitespace region"
  verify property "rule priority order is deterministic across invocations"
}
