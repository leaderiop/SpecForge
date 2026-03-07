// Formatting behaviors — the code formatter pipeline

use invariants/formatting
use types/config
use types/core
use types/formatting
use ports/outbound
use ports/inbound
use events/compilation

behavior format_spec_files "Format Spec Files" {
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism]
  types      [FormatConfig, FormatDiff]
  ports      [FileSystem, CompilerApi]
  produces   [format_complete]

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
    The formatter MUST attach every comment to the correct CST node using
    the comment attachment algorithm: leading comments attach to the following
    node, trailing comments attach to the preceding node on the same line,
    section header comments attach to the next block group, and standalone
    comment blocks separated by blank lines are preserved as-is.
  """

  verify unit "leading comment attaches to following node"
  verify unit "trailing comment attaches to preceding node on same line"
  verify unit "section header comment attaches to next block group"
  verify unit "standalone comment block between blocks is preserved"
  verify property "no comments are lost after formatting"

}

behavior check_formatting "Check Formatting Without Modifying Files" {
  // Dry-run mode: does not emit format_complete (no files modified)
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism]
  types      [FormatConfig, FormatDiff]
  ports      [FileSystem]

  contract """
    When specforge format --check is invoked, the system MUST compare
    what would be formatted against existing files on disk. If any file
    would change, the command MUST exit with code 1 and print the file
    paths. The system MUST NOT write any files in check mode.
  """

  verify unit "already formatted files exit with code 0"
  verify unit "unformatted files exit with code 1"
  verify unit "check mode writes no files to disk"

}

behavior show_formatting_diff "Show Formatting Diff" {
  // Dry-run mode: does not emit format_complete (no files modified)
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism]
  types      [FormatDiff]
  ports      [FileSystem]

  contract """
    When specforge format --diff is invoked, the system MUST produce
    a unified diff of formatting changes for each file that would be
    modified. The diff MUST use standard unified format with --- and +++
    headers. The system MUST NOT write any files in diff mode.
  """

  verify unit "diff output uses unified format"
  verify unit "diff mode writes no files to disk"
  verify unit "unchanged files produce no diff output"

}

behavior format_from_stdin "Format from Standard Input" {
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism]
  types      [FormatConfig]
  produces   [format_complete]

  contract """
    When specforge format --stdin is invoked, the system MUST read
    .spec content from standard input, format it, and write the
    formatted output to standard output. This enables editor integrations
    that pipe buffer contents through the formatter. The same formatting
    guarantees (idempotency, consistency, comment preservation) apply as
    for file-based formatting.
  """

  verify unit "stdin content is formatted and written to stdout"
  verify unit "stdin mode does not read or write files"
  verify property "stdin formatting is idempotent"
  verify property "stdin formatting converges to canonical form"

}

behavior load_format_config "Load Format Configuration" {
  invariants [config_defaults_valid]
  types      [FormatConfig]
  ports      [FileSystem]

  contract """
    The formatter MUST discover configuration by walking up from the
    formatted file's directory toward the project root (the directory
    containing specforge.json). The walk MUST stop at the project root
    and MUST NOT continue beyond it. Configuration files outside the
    project root MUST NOT be discovered. If .specforgefmt.toml is found,
    it MUST be parsed and validated. Invalid values MUST produce
    diagnostics and fall back to defaults. If no config file is found
    within the project root boundary, defaults MUST be used.
  """

  verify unit "config file in project root is loaded"
  verify unit "config file in parent directory is discovered"
  verify unit "config discovery walks from formatted file directory up to specforge.json parent then stops"
  verify unit "config outside project root is not discovered"
  verify unit "invalid indent_width produces diagnostic and uses default"
  verify unit "missing config file uses defaults"

}

behavior apply_format_rules "Apply Format Rules" {
  // Extension format rules are discovered via the contribution registry at format time
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism, format_rule_priority]
  types      [FormatConfig, FormatRule]

  contract """
    The formatting rule engine MUST walk the CST and emit formatting
    decisions for each whitespace region: keep, replace, insert, or remove.
    Rules cover indentation, spacing, alignment, wrapping, blank lines,
    comments, imports, and string formatting. All rules MUST produce
    deterministic output for the same input and configuration. Rules
    operate on generic keyword blocks and fields — they MUST NOT contain
    logic specific to any extension-defined entity kind.
  """

  verify unit "indentation rules normalize to configured indent style"
  verify unit "spacing rules normalize single spaces between tokens"
  verify unit "alignment rules align field values within blocks"
  verify unit "wrapping rules break long reference lists to multi-line"
  verify unit "import sorting produces alphabetical order"
  verify unit "blank line rules enforce exactly one between blocks"
  verify unit "comment rules normalize spacing around inline comments"
  verify unit "string rules normalize multiline string literal indentation"
  verify property "two files differing only in whitespace produce identical output after formatting"

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
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism]
  types      [FormatConfig, TextEdit]
  ports      [LspProtocol]
  produces   [format_complete]

  contract """
    When the LSP server receives a textDocument/formatting request,
    it MUST format the full document using the formatting engine and
    return a list of TextEdit operations. The result MUST be identical
    to running specforge format on the same file. TextEdit coordinates
    use 0-indexed lines and columns (LSP standard). TextEdit operations
    in a response MUST NOT overlap. When the document contains parse
    errors, the server MUST format well-formed regions and leave error
    regions unchanged, consistent with format_with_parse_errors.
  """

  verify unit "formatting request returns TextEdit list"
  verify unit "TextEdit coordinates are 0-indexed lines and columns"
  verify unit "TextEdit operations in a response do not overlap"
  verify integration "LSP format produces same result as CLI format"
  verify integration "parse errors in document trigger format_with_parse_errors delegation"
  verify performance "formats document within 50ms for files under 1000 lines"

}

behavior lsp_format_range "LSP Format Range" {
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism]
  types      [FormatConfig, TextEdit]
  ports      [LspProtocol]
  produces   [format_complete]

  contract """
    When the LSP server receives a textDocument/rangeFormatting request,
    it MUST expand the range to complete block boundaries, format the
    expanded range, and return TextEdit operations for the affected region.
    TextEdit coordinates use 0-indexed lines and columns (LSP standard).
    TextEdit operations in a response MUST NOT overlap. The formatted
    range MUST produce the same result as full-document formatting for
    the affected blocks. When the selected range contains parse errors,
    error regions within the range MUST be left unchanged.
  """

  verify unit "range is expanded to block boundaries"
  verify unit "range formatting matches full formatting for affected blocks"
  verify integration "parse errors within range are left unchanged per format_with_parse_errors"
  verify performance "formats range within 20ms for ranges under 200 lines"

}

behavior lsp_respect_editor_config "LSP Respect Editor Config" {
  invariants [config_defaults_valid]
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

behavior format_with_parse_errors "Format Files with Parse Errors" {
  // formatting_consistency applies to well-formed regions only; error regions
  // are preserved verbatim and do not participate in consistency checks.
  invariants [comment_preservation, formatting_idempotency, formatting_consistency, format_rule_determinism]
  types      [FormatConfig, FormatDiff]
  ports      [FileSystem, LspProtocol]

  contract """
    When the formatter encounters a .spec file that contains parse errors,
    it MUST NOT crash or produce corrupted output. The formatter MUST use
    tree-sitter error recovery to format well-formed regions of the file
    and leave error regions unchanged. An error region begins at the first
    unparseable token (as identified by a tree-sitter ERROR or MISSING node)
    and extends forward until the next token that begins a parseable
    top-level statement (use directive, entity block, or comment). All
    original whitespace within an error region MUST be preserved byte-for-byte.
    A diagnostic MUST be emitted listing each file that could not be fully
    formatted due to parse errors, including the line range of each error region.
  """

  verify unit "file with syntax error is partially formatted without crash"
  verify unit "well-formed blocks in a file with errors are still formatted"
  verify unit "error regions are preserved verbatim in output"
  verify unit "error region starts at first unparseable token"
  verify unit "error region ends before next parseable top-level statement"
  verify unit "whitespace within error regions is preserved byte-for-byte"
  verify unit "diagnostic lists files with parse errors and error line ranges"

}

behavior discover_format_targets "Discover Format Targets" {
  invariants [discover_completeness]
  types      [FormatConfig, CompilerConfig]
  ports      [FileSystem]

  contract """
    When specforge format is invoked without explicit file paths, the
    system MUST discover all .spec files under the spec_root defined in
    specforge.json. When explicit paths are provided, only those paths
    MUST be formatted. Directories provided as arguments MUST be
    recursively searched for .spec files. Non-.spec files MUST be
    skipped without error. If specforge.json defines a format.exclude
    glob list, matching files MUST be excluded from discovery.
  """

  verify unit "no arguments formats all .spec files under spec_root"
  verify unit "files matching format.exclude globs are excluded"
  verify unit "explicit file paths format only those files"
  verify unit "directory argument recursively discovers .spec files"
  verify unit "non-.spec files are skipped with no error"

}
