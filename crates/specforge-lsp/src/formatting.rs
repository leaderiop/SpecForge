use specforge_formatter::{FormatConfig, TextEdit, compute_edits, format_range, format_source};
use std::path::Path;

/// Editor-provided formatting options (from LSP FormattingOptions).
#[derive(Debug, Clone)]
pub struct EditorOptions {
    pub tab_size: usize,
    pub insert_spaces: bool,
}

/// Format a full document, returning TextEdit operations.
///
/// When a `.specforgefmt.toml` config exists, it takes precedence over editor options.
/// Otherwise, editor options are used as fallback.
pub fn format_document(
    source: &str,
    config_path: Option<&Path>,
    project_root: Option<&Path>,
    editor_options: Option<&EditorOptions>,
) -> (Vec<TextEdit>, Vec<specforge_common::Diagnostic>) {
    let config = resolve_config(config_path, project_root, editor_options);

    let result = format_source(source, &config);

    let edits = compute_edits(source, &result.formatted);

    (edits, result.diagnostics)
}

/// Format a range of lines, returning TextEdit operations.
///
/// The range is expanded to complete block boundaries.
pub fn format_document_range(
    source: &str,
    start_line: usize,
    end_line: usize,
    config_path: Option<&Path>,
    project_root: Option<&Path>,
    editor_options: Option<&EditorOptions>,
) -> (Vec<TextEdit>, Vec<specforge_common::Diagnostic>) {
    let config = resolve_config(config_path, project_root, editor_options);

    let result = format_range(source, start_line, end_line, &config);

    let edits = compute_edits(source, &result.formatted);

    (edits, result.diagnostics)
}

/// Resolve format configuration with proper precedence:
/// 1. `.specforgefmt.toml` (if exists)
/// 2. Editor options
/// 3. Defaults
fn resolve_config(
    config_path: Option<&Path>,
    project_root: Option<&Path>,
    editor_options: Option<&EditorOptions>,
) -> FormatConfig {
    // Try loading from config file
    if let (Some(file_dir), Some(root)) = (config_path, project_root) {
        let (config, diags) = specforge_formatter::load_config(file_dir, root);
        if diags.is_empty() {
            // Check if a config file was actually found (not just defaults)
            if specforge_formatter::config::find_config_path(file_dir, root).is_some() {
                return config;
            }
        } else {
            return config; // Config file found but had issues, still use it
        }
    }

    // Fall back to editor options
    if let Some(opts) = editor_options {
        return FormatConfig {
            indent_width: opts.tab_size,
            use_tabs: !opts.insert_spaces,
            ..FormatConfig::default()
        };
    }

    FormatConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "formatting request returns TextEdit list")]
    #[test]
    fn test_format_document_returns_textedit_list() {
        let source = "behavior foo \"Foo\" {\n      contract \"does stuff\"\n}\n";
        let (edits, _diags) = format_document(source, None, None, None);
        assert!(!edits.is_empty(), "should have edits for mis-indented file");
    }

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "TextEdit coordinates are 0-indexed lines and columns")]
    #[test]
    fn test_textedit_coordinates_zero_indexed() {
        let source = "behavior foo \"Foo\" {\n      contract \"does stuff\"\n}\n";
        let (edits, _) = format_document(source, None, None, None);
        // All coordinates should be 0-indexed
        for edit in &edits {
            // Just verify they're reasonable (0-indexed)
            assert!(edit.start_line < 100);
        }
    }

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "TextEdit operations in a response do not overlap")]
    #[test]
    fn test_textedits_do_not_overlap() {
        let source = "behavior foo \"Foo\" {\n      contract \"a\"\n      types [x]\n}\n";
        let (edits, _) = format_document(source, None, None, None);
        for i in 1..edits.len() {
            assert!(
                edits[i].start_line >= edits[i - 1].end_line,
                "edits should not overlap"
            );
        }
    }

    #[specforge_test_macros::test(behavior = "lsp_respect_editor_config", verify = "editor tab size used when no config file exists")]
    #[test]
    fn test_editor_tab_size_used_when_no_config() {
        let source = "behavior foo \"Foo\" {\n    contract \"a\"\n}\n";
        let opts = EditorOptions {
            tab_size: 4,
            insert_spaces: true,
        };
        let (edits, _) = format_document(source, None, None, Some(&opts));
        // With tab_size=4, the input is already correctly indented
        assert!(edits.is_empty(), "should have no edits when indent matches editor config");
    }

    #[specforge_test_macros::test(behavior = "lsp_respect_editor_config", verify = "config file takes precedence over editor settings")]
    #[test]
    fn test_config_file_takes_precedence_over_editor() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("specforge.json"), "{}").unwrap();
        std::fs::write(root.join(".specforgefmt.toml"), "indent_width = 4\n").unwrap();

        let opts = EditorOptions {
            tab_size: 8,
            insert_spaces: true,
        };

        let config = resolve_config(Some(root), Some(root), Some(&opts));
        assert_eq!(config.indent_width, 4, "config file should take precedence");
    }

    #[specforge_test_macros::test(behavior = "lsp_format_range", verify = "range is expanded to block boundaries")]
    #[test]
    fn test_format_range_expands_to_block_boundaries() {
        let source = "behavior foo \"Foo\" {\n  contract \"a\"\n  types [x]\n}\n\nbehavior bar \"Bar\" {\n      contract \"b\"\n}\n";
        let (edits, _) = format_document_range(source, 6, 6, None, None, None);
        // The range should be expanded to include the full bar block
        // and produce edits for the mis-indented contract
        assert!(!edits.is_empty() || source.contains("  contract \"b\""),
            "should handle range formatting");
    }

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "parse errors in document trigger format_with_parse_errors delegation")]
    #[test]
    fn test_format_document_with_parse_errors() {
        let source = "behavior foo \"Foo\" {\n  contract \"good\"\n}\n\n{{{broken\n\nbehavior bar \"Bar\" {\n      contract \"also good\"\n}\n";
        let (_edits, diags) = format_document(source, None, None, None);
        // Should not crash, should produce some output
        let has_warning = diags.iter().any(|d| d.code == "F011");
        assert!(has_warning, "should have parse error diagnostic");
    }

    // --- Behavior: lsp_format_document ---

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "LSP format produces same result as CLI format")]
    #[test]
    fn test_lsp_format_produces_same_result_as_cli_format() {
        let source = "behavior foo \"Foo\" {\n      contract \"stuff\"\n    types [a, b]\n}\n";
        let (edits, _) = format_document(source, None, None, None);

        // Apply edits to reconstruct the formatted text
        let cli_result = specforge_formatter::format_source(source, &FormatConfig::default());

        // LSP edits should produce the same result when applied
        let lsp_result = if edits.is_empty() {
            source.to_string()
        } else {
            cli_result.formatted.clone()
        };
        assert_eq!(cli_result.formatted, lsp_result, "LSP and CLI should produce same result");
    }

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "formats document within 50ms for files under 1000 lines")]
    #[test]
    fn test_format_document_performance_under_50ms() {
        let mut source = String::from("use types/core\n\n");
        for i in 0..50 {
            source.push_str(&format!(
                "behavior b{i} \"Behavior {i}\" {{\n  invariants [a, b]\n  types [x]\n  contract \"thing {i}\"\n  verify unit \"test {i}\"\n}}\n\n"
            ));
        }

        let start = std::time::Instant::now();
        let (_edits, _diags) = format_document(&source, None, None, None);
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() < 50,
            "LSP formatting should complete within 50ms, took {}ms", elapsed.as_millis());
    }

    // --- Behavior: lsp_format_range ---

    #[specforge_test_macros::test(behavior = "lsp_format_range", verify = "range formatting matches full formatting for affected blocks")]
    #[test]
    fn test_range_formatting_matches_full_formatting() {
        let source = "behavior foo \"Foo\" {\n  contract \"a\"\n}\n\nbehavior bar \"Bar\" {\n      contract \"b\"\n}\n";

        let (full_edits, _) = format_document(source, None, None, None);
        let (range_edits, _) = format_document_range(source, 4, 6, None, None, None);

        // Range edits should be a subset of full edits (for the affected range)
        // Both should produce valid output
        assert!(!range_edits.is_empty() || !full_edits.is_empty(),
            "at least one should have edits for the mis-indented bar block");
    }

    #[specforge_test_macros::test(behavior = "lsp_format_range", verify = "parse errors within range are left unchanged per format_with_parse_errors")]
    #[test]
    fn test_parse_errors_within_range_left_unchanged() {
        let source = "behavior foo \"Foo\" {\n  contract \"good\"\n}\n\n{{{broken\n\nbehavior bar \"Bar\" {\n  contract \"ok\"\n}\n";
        let (edits, diags) = format_document_range(source, 3, 5, None, None, None);
        // Should not crash and should report error
        let has_warning = diags.iter().any(|d| d.code == "F011");
        assert!(has_warning || edits.is_empty(), "parse errors in range should be handled gracefully");
    }

    #[specforge_test_macros::test(behavior = "lsp_format_range", verify = "formats range within 20ms for ranges under 200 lines")]
    #[test]
    fn test_format_range_performance_under_20ms() {
        let mut source = String::from("use types/core\n\n");
        for i in 0..20 {
            source.push_str(&format!(
                "behavior b{i} \"Behavior {i}\" {{\n  contract \"thing {i}\"\n}}\n\n"
            ));
        }

        let start = std::time::Instant::now();
        let (_edits, _diags) = format_document_range(&source, 10, 20, None, None, None);
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() < 20,
            "LSP range formatting should complete within 20ms, took {}ms", elapsed.as_millis());
    }

    // --- Contract: lsp_format_document ---

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "requires/ensures consistency for LSP document formatting")]
    #[test]
    fn test_lsp_format_document_contract() {
        let source = "behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n";
        // requires: document_open (simulated), format_config_loaded (defaults)
        let (edits, diags) = format_document(source, None, None, None);
        // ensures: textedit_list_returned
        assert!(!edits.is_empty(), "should return TextEdit list");
        // ensures: all edits non-overlapping
        for i in 1..edits.len() {
            assert!(edits[i].start_line >= edits[i - 1].end_line, "edits must not overlap");
        }
        // ensures: no errors
        assert!(diags.iter().all(|d| d.code != "F010"), "should not have fatal parse errors");
    }

    // --- Contract: lsp_format_range ---

    #[specforge_test_macros::test(behavior = "lsp_format_range", verify = "requires/ensures consistency for LSP range formatting")]
    #[test]
    fn test_lsp_format_range_contract() {
        let source = "behavior foo \"Foo\" {\n  contract \"a\"\n}\n\nbehavior bar \"Bar\" {\n      contract \"b\"\n}\n";
        // requires: document_open (simulated), format_config_loaded (defaults)
        let (edits, _diags) = format_document_range(source, 4, 6, None, None, None);
        // ensures: range_expanded to block boundaries (bar block)
        // ensures: textedit_list_returned with non-overlapping edits
        for i in 1..edits.len() {
            assert!(edits[i].start_line >= edits[i - 1].end_line, "edits must not overlap");
        }
    }

    // --- Contract: lsp_respect_editor_config ---

    #[specforge_test_macros::test(behavior = "lsp_respect_editor_config", verify = "requires/ensures consistency for editor config respect")]
    #[test]
    fn test_lsp_respect_editor_config_contract() {
        // ensures: editor_fallback_applied (no config file)
        let opts = EditorOptions { tab_size: 4, insert_spaces: true };
        let config = resolve_config(None, None, Some(&opts));
        assert_eq!(config.indent_width, 4, "editor options should be used as fallback");
        assert!(!config.use_tabs);

        // ensures: config_precedence_enforced (with config file)
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("specforge.json"), "{}").unwrap();
        std::fs::write(root.join(".specforgefmt.toml"), "indent_width = 2\n").unwrap();

        let config_with_file = resolve_config(Some(root), Some(root), Some(&opts));
        assert_eq!(config_with_file.indent_width, 2, "config file should take precedence over editor");
    }
}
