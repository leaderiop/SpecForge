// Formatting behaviors — the code formatter pipeline

use invariants/formatting
use types/core
use types/formatting
use types/errors
use ports/outbound
use ports/inbound

behavior format_spec_files "Format Spec Files" {
  invariants [formatting_idempotency, formatting_consistency]
  types      [FormatConfig, FormatDiff]
  ports      [FileSystem, CompilerApi]

  contract """
    When specforge format is invoked with file paths or a project directory,
    the system MUST parse each .spec file into a CST using tree-sitter,
    apply formatting rules, and write the formatted output back to disk.
    Files that are already correctly formatted MUST NOT be rewritten.
    The command MUST print the names of changed files and a summary count.
  """

  verify unit "files matching the canonical format are not rewritten"
  verify unit "changed files are printed to stdout"
  verify unit "summary count reflects actual changes"
  verify integration "formatting all files in spec/ directory succeeds"
}

behavior preserve_comments "Preserve Comments During Formatting" {
  invariants [comment_preservation]
  types      [FormatConfig]

  contract """
    The formatter MUST attach every comment to the correct AST node using
    the comment attachment algorithm: leading comments attach to the following
    node, trailing comments attach to the preceding node on the same line,
    section header comments attach to the next block group, and standalone
    comment blocks separated by blank lines are preserved as-is.
  """

  verify unit "leading comment attaches to following node"
  verify unit "trailing comment attaches to preceding node on same line"
  verify unit "section header comment attaches to next block group"
  verify unit "standalone comment block between entities is preserved"
  verify property "no comments are lost after formatting"
}

behavior check_formatting "Check Formatting Without Modifying Files" {
  types      [FormatConfig, FormatDiff]
  ports      [FileSystem]

  contract """
    When specforge format --check is invoked, the system MUST compare
    what would be formatted against existing files on disk. If any file
    would change, the command MUST exit with code 1 and print the file
    paths. No files MUST be written in check mode.
  """

  verify unit "already formatted files exit with code 0"
  verify unit "unformatted files exit with code 1"
  verify unit "check mode writes no files to disk"
}

behavior show_formatting_diff "Show Formatting Diff" {
  types      [FormatDiff]
  ports      [FileSystem]

  contract """
    When specforge format --diff is invoked, the system MUST produce
    a unified diff of formatting changes for each file that would be
    modified. The diff MUST use standard unified format with --- and +++
    headers. No files MUST be written in diff mode.
  """

  verify unit "diff output uses unified format"
  verify unit "diff mode writes no files to disk"
  verify unit "unchanged files produce no diff output"
}

behavior format_from_stdin "Format from Standard Input" {
  types      [FormatConfig]

  contract """
    When specforge format --stdin is invoked, the system MUST read
    .spec content from standard input, format it, and write the
    formatted output to standard output. This enables editor integrations
    that pipe buffer contents through the formatter.
  """

  verify unit "stdin content is formatted and written to stdout"
  verify unit "stdin mode does not read or write files"
}

behavior load_format_config "Load Format Configuration" {
  types      [FormatConfig]
  ports      [FileSystem]

  contract """
    The formatter MUST discover configuration by walking up from the
    formatted file to the project root (directory containing specforge.spec).
    If .specforgefmt.toml is found, it MUST be parsed and validated.
    Invalid values MUST produce diagnostics and fall back to defaults.
    If no config file is found, defaults MUST be used.
  """

  verify unit "config file in project root is loaded"
  verify unit "config file in parent directory is discovered"
  verify unit "invalid indent_width produces diagnostic and uses default"
  verify unit "missing config file uses defaults"
}

behavior apply_format_rules "Apply Format Rules" {
  invariants [formatting_idempotency, formatting_consistency]
  types      [FormatConfig, FormatRule]

  contract """
    The formatting rule engine MUST walk the CST and emit formatting
    decisions for each whitespace region: keep, replace, insert, or remove.
    Rules cover indentation, spacing, alignment, wrapping, blank lines,
    comments, imports, and string formatting. All rules MUST produce
    deterministic output for the same input and configuration.
  """

  verify unit "indentation rules normalize to configured indent style"
  verify unit "spacing rules normalize single spaces between tokens"
  verify unit "alignment rules align field values within blocks"
  verify unit "wrapping rules break long reference lists to multi-line"
  verify unit "import sorting produces alphabetical order"
  verify unit "blank line rules enforce exactly one between blocks"
}

behavior maintain_format_idempotency "Maintain Format Idempotency" {
  invariants [formatting_idempotency]
  types      [FormatConfig]

  contract """
    The formatter MUST satisfy the idempotency property: applying the
    formatter twice produces the same output as applying it once.
    This MUST be verified by property-based tests that generate random
    valid .spec files and check format(format(x)) == format(x).
    Any idempotency violation is treated as a P0 bug.
  """

  verify property "format(format(x)) == format(x) for random valid inputs"
  verify unit "alignment rules do not oscillate between runs"
  verify unit "wrapping decisions are stable across runs"
}

behavior lsp_format_document "LSP Format Document" {
  invariants [formatting_idempotency]
  types      [FormatConfig]
  ports      [LspProtocol]

  contract """
    When the LSP server receives a textDocument/formatting request,
    it MUST format the full document using the formatting engine and
    return a list of TextEdit operations. The response MUST be delivered
    within 50ms for a typical .spec file. The result MUST be identical
    to running specforge format on the same file.
  """

  verify unit "formatting request returns TextEdit list"
  verify unit "response time is under 50ms for typical files"
  verify integration "LSP format produces same result as CLI format"
}

behavior lsp_format_range "LSP Format Range" {
  invariants [formatting_idempotency]
  types      [FormatConfig]
  ports      [LspProtocol]

  contract """
    When the LSP server receives a textDocument/rangeFormatting request,
    it MUST expand the range to complete block boundaries, format the
    expanded range, and return TextEdit operations for the affected region.
    The formatted range MUST produce the same result as full-document
    formatting for the affected blocks.
  """

  verify unit "range is expanded to block boundaries"
  verify unit "range formatting matches full formatting for affected blocks"
}

behavior lsp_respect_editor_config "LSP Respect Editor Config" {
  types      [FormatConfig]
  ports      [LspProtocol]

  contract """
    When no .specforgefmt.toml exists, the LSP formatting MUST respect
    editor-level settings for tab size and insert-spaces. When a
    .specforgefmt.toml exists, it MUST take precedence over editor settings.
  """

  verify unit "editor tab size used when no config file exists"
  verify unit "config file takes precedence over editor settings"
}
