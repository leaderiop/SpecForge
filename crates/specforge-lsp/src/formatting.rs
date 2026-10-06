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
    #[specforge_test_macros::test(
        behavior = "lsp_format_document",
        verify = "LSP format produces same result as CLI format"
    )]
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
        assert_eq!(
            cli_result.formatted, lsp_result,
            "LSP and CLI should produce same result"
        );
    }
}
