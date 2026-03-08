// Formatting behaviors — the code formatter pipeline

use invariants/formatting
use invariants/core
use types/config
use types/core
use types/formatting
use ports/outbound
use ports/inbound
use events/compilation

behavior format_spec_files "Format Spec Files" {
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism, formatting_semantic_preservation]
  types      [FormatConfig, FormatDiff]
  ports      [FileSystem, CompilerApi]
  produces   [format_complete]

  requires {
    spec_files_available "target .spec files exist on disk and are readable"
    format_config_loaded "FormatConfig has been resolved (from .specforgefmt.toml or defaults)"
  }

  ensures {
    formatted_output_written "formatted output is written back to disk for files that changed"
    unchanged_files_preserved "files already in canonical format are not rewritten"
    format_complete_emitted "format_complete event is produced after successful formatting"
    summary_printed "names of changed files and a summary count are printed"
  }

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
  verify contract "requires/ensures consistency for spec file formatting"

}

behavior preserve_comments "Preserve Comments During Formatting" {
  invariants [comment_preservation]
  types      [FormatConfig]

  requires {
    cst_available "the .spec file has been parsed into a CST with comment nodes present"
  }

  ensures {
    all_comments_attached "every comment is attached to the correct CST node per the attachment algorithm"
    no_comments_lost "no comments are dropped or reordered during formatting"
  }

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
  verify contract "requires/ensures consistency for comment preservation"

}

behavior check_formatting "Check Formatting Without Modifying Files" {
  // Dry-run mode: does not emit format_complete (no files modified)
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism, dry_run_side_effect_freedom, formatting_semantic_preservation]
  types      [FormatConfig, FormatDiff]
  ports      [FileSystem]

  requires {
    spec_files_available "target .spec files exist on disk and are readable"
    format_config_loaded "FormatConfig has been resolved (from .specforgefmt.toml or defaults)"
  }

  ensures {
    no_files_written "no files are written to disk in check mode"
    exit_code_correct "exit code is 0 when all files are formatted, 1 when any would change"
    unformatted_paths_printed "file paths of unformatted files are printed to stdout"
  }

  contract """
    When specforge format --check is invoked, the system MUST compare
    what would be formatted against existing files on disk. If any file
    would change, the command MUST exit with code 1 and print the file
    paths. The system MUST NOT write any files in check mode.
  """

  verify unit "already formatted files exit with code 0"
  verify unit "unformatted files exit with code 1"
  verify unit "check mode writes no files to disk"
  verify contract "requires/ensures consistency for formatting check"

}

behavior show_formatting_diff "Show Formatting Diff" {
  // Dry-run mode: does not emit format_complete (no files modified)
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism, dry_run_side_effect_freedom, formatting_semantic_preservation]
  types      [FormatDiff]
  ports      [FileSystem]

  requires {
    spec_files_available "target .spec files exist on disk and are readable"
    format_config_loaded "FormatConfig has been resolved (from .specforgefmt.toml or defaults)"
  }

  ensures {
    no_files_written "no files are written to disk in diff mode"
    unified_diff_produced "a unified diff with --- and +++ headers is produced for each file that would change"
  }

  contract """
    When specforge format --diff is invoked, the system MUST produce
    a unified diff of formatting changes for each file that would be
    modified. The diff MUST use standard unified format with --- and +++
    headers. The system MUST NOT write any files in diff mode.
  """

  verify unit "diff output uses unified format"
  verify unit "diff mode writes no files to disk"
  verify unit "unchanged files produce no diff output"
  verify contract "requires/ensures consistency for formatting diff"

}

behavior format_from_stdin "Format from Standard Input" {
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism, formatting_semantic_preservation]
  types      [FormatConfig]
  produces   [format_complete]

  requires {
    stdin_available "standard input contains valid .spec content"
    format_config_loaded "FormatConfig has been resolved (from .specforgefmt.toml or defaults)"
  }

  ensures {
    stdout_produced "formatted output is written to standard output"
    no_files_touched "no files are read from or written to disk"
    format_complete_emitted "format_complete event is produced after successful formatting"
  }

  contract """
    When specforge format --stdin is invoked, the system MUST read
    .spec content from standard input, format it, and write the
    formatted output to standard output. This enables editor integrations
    that pipe buffer contents through the formatter. The same formatting
    guarantees (idempotency, consistency, comment preservation) apply as
    for file-based formatting. In stdin mode, the format_complete event
    MUST set filesChecked=1 and filesChanged to 0 (input already canonical)
    or 1 (formatting applied).
  """

  verify unit "stdin content is formatted and written to stdout"
  verify unit "stdin mode does not read or write files"
  verify property "stdin formatting is idempotent"
  verify property "stdin formatting converges to canonical form"
  verify contract "requires/ensures consistency for stdin formatting"

}

behavior load_format_config "Load Format Configuration" {
  invariants [config_defaults_valid]
  types      [FormatConfig]
  ports      [FileSystem]

  requires {
    project_root_available "project root directory (containing specforge.json) is identifiable"
    filesystem_accessible "FileSystem port is available for reading configuration files"
  }

  ensures {
    config_resolved "FormatConfig is resolved from .specforgefmt.toml or defaults"
    walk_bounded "configuration discovery does not continue beyond the project root"
    invalid_values_diagnosed "invalid configuration values produce diagnostics and fall back to defaults"
  }

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
  verify contract "requires/ensures consistency for format config loading"

}

behavior apply_format_rules "Apply Format Rules" {
  // Extension format rules are discovered via the contribution registry at format time
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism, format_rule_priority, formatting_semantic_preservation]
  types      [FormatConfig, FormatRule]
  ports      [WasmRuntime]

  requires {
    cst_available "the .spec file has been parsed into a CST for rule engine traversal"
    format_config_loaded "FormatConfig has been resolved with indent style and width settings"
    contribution_registry_available "in-memory contribution registry is populated with extension format rules"
  }

  ensures {
    deterministic_output "all rules produce deterministic output for the same input and configuration"
    no_domain_logic "rules operate on generic keyword blocks and fields with no extension-specific entity kind logic"
    extension_rules_applied "extension-contributed format rules are executed at priority level 9 via WasmRuntime"
  }

  contract """
    The formatting rule engine MUST walk the CST and emit formatting
    decisions for each whitespace region: keep, replace, insert, or remove.
    Rules cover indentation, spacing, alignment, wrapping, blank lines,
    comments, imports, and string formatting. All rules MUST produce
    deterministic output for the same input and configuration. Rules
    operate on generic keyword blocks and fields — they MUST NOT contain
    logic specific to any extension-defined entity kind.

    Extension-contributed format rules MUST be discovered from the
    in-memory contribution registry (populated during extension loading)
    and executed via the WasmRuntime port. Extension rules run at priority
    level 9 (after all 8 core rules). When multiple extensions contribute
    format rules, they MUST be applied in extension load order.
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
  verify contract "requires/ensures consistency for format rule application"

}

behavior maintain_format_idempotency "Maintain Format Idempotency" {
  invariants [formatting_idempotency, formatting_semantic_preservation]
  types      [FormatConfig]

  requires {
    format_rules_available "all format rules (core and extension) are loaded and ready"
  }

  ensures {
    idempotency_holds "format(format(x)) == format(x) for all valid inputs"
    no_oscillation "alignment and wrapping decisions are stable across consecutive runs"
  }

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
  verify contract "requires/ensures consistency for format idempotency"

}

behavior lsp_format_document "LSP Format Document" {
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism, formatting_semantic_preservation]
  types      [FormatConfig, TextEdit]
  refs       [format_with_parse_errors]
  ports      [LspProtocol]
  produces   [format_complete]

  requires {
    document_open "the document is registered in the LSP open document set"
    format_config_loaded "FormatConfig has been resolved for the document's project"
  }

  ensures {
    textedit_list_returned "a list of non-overlapping TextEdit operations is returned"
    cli_parity_enforced "the result is identical to running specforge format on the same file"
    format_complete_emitted "format_complete event is produced after successful formatting"
  }

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
  verify contract "requires/ensures consistency for LSP document formatting"

}

behavior lsp_format_range "LSP Format Range" {
  invariants [formatting_idempotency, formatting_consistency, comment_preservation, format_rule_determinism, formatting_semantic_preservation]
  types      [FormatConfig, TextEdit]
  ports      [LspProtocol]
  produces   [format_complete]

  requires {
    document_open "the document is registered in the LSP open document set"
    format_config_loaded "FormatConfig has been resolved for the document's project"
  }

  ensures {
    range_expanded "the range is expanded to complete block boundaries before formatting"
    textedit_list_returned "a list of non-overlapping TextEdit operations for the affected region is returned"
    full_format_parity "the formatted range produces the same result as full-document formatting for affected blocks"
    format_complete_emitted "format_complete event is produced after successful formatting"
  }

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
  verify contract "requires/ensures consistency for LSP range formatting"

}

behavior lsp_respect_editor_config "LSP Respect Editor Config" {
  invariants [config_defaults_valid, format_rule_determinism]
  types      [FormatConfig]
  ports      [LspProtocol]

  requires {
    lsp_initialized_fired "LSP server has been initialized and editor settings are available"
  }

  ensures {
    config_precedence_enforced ".specforgefmt.toml takes precedence over editor settings when it exists"
    editor_fallback_applied "editor-level tab size and insert-spaces are used when no config file exists"
  }

  contract """
    When no .specforgefmt.toml exists, the LSP formatting MUST respect
    editor-level settings for tab size and insert-spaces. When a
    .specforgefmt.toml exists, it MUST take precedence over editor settings.
  """

  verify unit "editor tab size used when no config file exists"
  verify unit "config file takes precedence over editor settings"
  verify contract "requires/ensures consistency for editor config respect"

}

behavior format_with_parse_errors "Format Files with Parse Errors" {
  // formatting_consistency applies to well-formed regions only; error regions
  // are preserved verbatim and do not participate in consistency checks.
  invariants [comment_preservation, formatting_idempotency, formatting_consistency, format_rule_determinism, formatting_semantic_preservation]
  types      [FormatConfig, FormatDiff]
  ports      [FileSystem, LspProtocol]

  requires {
    cst_with_errors "the .spec file has been parsed into a CST that contains tree-sitter ERROR or MISSING nodes"
  }

  ensures {
    no_crash "the formatter does not crash or produce corrupted output"
    well_formed_regions_formatted "well-formed regions of the file are formatted normally"
    error_regions_preserved "error regions are preserved verbatim with original whitespace byte-for-byte"
    parse_error_diagnosed "a diagnostic is emitted listing each file with parse errors and error line ranges"
  }

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
  verify contract "requires/ensures consistency for formatting with parse errors"

}

behavior discover_format_targets "Discover Format Targets" {
  invariants [discover_completeness]
  types      [FormatConfig, CompilerConfig]
  ports      [FileSystem]

  requires {
    project_root_available "project root directory (containing specforge.json) is identifiable for spec_root resolution"
    filesystem_accessible "FileSystem port is available for directory traversal"
  }

  ensures {
    all_spec_files_discovered "all .spec files under spec_root are discovered when no explicit paths are given"
    exclusions_applied "files matching format.exclude globs are excluded from the target set"
    non_spec_skipped "non-.spec files are skipped without error"
  }

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
  verify contract "requires/ensures consistency for format target discovery"

}
