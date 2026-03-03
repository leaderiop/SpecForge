use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;
use specforge_codegen::generator::{GenerateContext, resolve_generator};

use crate::pipeline;

#[derive(Args)]
pub struct GenArgs {
    /// Generator name (e.g., "typescript", "json-schema")
    pub generator: String,

    /// Output directory (overrides the gen block's `out` field)
    pub dir: Option<PathBuf>,

    /// Check mode: compare generated files against disk, exit 1 on diff
    #[arg(long)]
    pub check: bool,

    /// Path to spec files. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

pub fn run(args: GenArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    // Find matching gen config
    let gen_config = result
        .config
        .gen_configs
        .iter()
        .find(|c| c.name == args.generator);

    let gen_config = match gen_config {
        Some(c) => c.clone(),
        None => {
            // No gen block found — create a default config
            specforge_common::GenConfig {
                name: args.generator.clone(),
                out: args
                    .dir
                    .as_ref()
                    .map(|d| d.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("generated/{}", args.generator)),
                result_style: specforge_common::ResultStyle::default(),
                readonly: false,
                naming: specforge_common::NamingStyle::default(),
                tests: None,
                extra: std::collections::HashMap::new(),
            }
        }
    };

    // CLI dir overrides gen block out
    let out_dir = args
        .dir
        .unwrap_or_else(|| PathBuf::from(&gen_config.out));

    let generator = resolve_generator(&gen_config.name);
    let ctx = GenerateContext {
        graph: &result.graph,
        files: &result.files,
        config: &result.config,
        gen_config: &gen_config,
    };
    let gen_result = generator.generate(&ctx);

    for warning in &gen_result.warnings {
        eprintln!("specforge: warning: {warning}");
    }

    for error in &gen_result.errors {
        eprintln!("specforge: error: {error}");
    }

    if !gen_result.errors.is_empty() {
        return 1;
    }

    if gen_result.files.is_empty() {
        eprintln!("specforge: no files generated for `{}`", gen_config.name);
        return 0;
    }

    if args.check {
        return run_check(&out_dir, &gen_result.files);
    }

    // Load previous entity checksums for incremental stats
    let state_path = out_dir.join(".specforge-gen-state.json");
    let prev_checksums = load_gen_state(&state_path);

    let code = run_write(&out_dir, &gen_result.files);

    // Report entity-level incremental stats
    if !gen_result.entity_checksums.is_empty() {
        let total = gen_result.entity_checksums.len();
        let changed = gen_result
            .entity_checksums
            .iter()
            .filter(|(id, checksum)| prev_checksums.get(id.as_str()) != Some(checksum))
            .count();
        let unchanged = total - changed;
        eprintln!("specforge: {total} entities generated ({changed} changed, {unchanged} unchanged)");
    }

    // Write updated entity checksums
    if let Ok(json) = serde_json::to_string_pretty(&gen_result.entity_checksums) {
        let _ = std::fs::write(&state_path, json);
    }

    code
}

fn run_check(out_dir: &std::path::Path, files: &[specforge_emitter::GeneratedFile]) -> i32 {
    let mut diffs = 0;
    for file in files {
        let path = out_dir.join(&file.path);
        match std::fs::read_to_string(&path) {
            Ok(existing) if existing == file.content => {}
            Ok(_) => {
                eprintln!("specforge: drift detected: {}", path.display());
                diffs += 1;
            }
            Err(_) => {
                eprintln!("specforge: missing: {}", path.display());
                diffs += 1;
            }
        }
    }
    if diffs > 0 {
        eprintln!("specforge: {diffs} file(s) out of date — run `specforge gen` to update");
        1
    } else {
        eprintln!("specforge: all generated files up to date");
        0
    }
}

fn run_write(out_dir: &PathBuf, files: &[specforge_emitter::GeneratedFile]) -> i32 {
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        eprintln!("specforge: cannot create output directory: {e}");
        return 1;
    }

    let mut written = 0;
    let mut skipped = 0;

    for file in files {
        let path = out_dir.join(&file.path);

        // Create parent directories for nested paths (e.g., __tests__/)
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("specforge: cannot create directory {}: {e}", parent.display());
                return 1;
            }
        }

        // Skip if file is unchanged (incremental)
        if let Ok(existing) = std::fs::read_to_string(&path) {
            if existing == file.content {
                skipped += 1;
                continue;
            }
        }

        if let Err(e) = std::fs::write(&path, &file.content) {
            eprintln!("specforge: cannot write {}: {e}", path.display());
            return 1;
        }
        written += 1;
    }

    eprintln!("specforge: {written} file(s) written, {skipped} unchanged");
    0
}

/// Load previous entity checksums from `.specforge-gen-state.json`.
fn load_gen_state(path: &std::path::Path) -> HashMap<String, String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_emitter::GeneratedFile;

    #[test]
    fn check_no_drift_exits_zero() {
        let dir = tempfile::tempdir().unwrap();
        let files = vec![
            GeneratedFile {
                path: "user.ts".to_string(),
                content: "export interface User {}\n".to_string(),
            },
            GeneratedFile {
                path: "order.ts".to_string(),
                content: "export interface Order {}\n".to_string(),
            },
        ];

        // Write matching files to disk
        for f in &files {
            std::fs::write(dir.path().join(&f.path), &f.content).unwrap();
        }

        let code = run_check(dir.path(), &files);
        assert_eq!(code, 0, "no drift should exit with code 0");
    }

    #[test]
    fn check_drift_detected_exits_one() {
        let dir = tempfile::tempdir().unwrap();
        let files = vec![GeneratedFile {
            path: "user.ts".to_string(),
            content: "export interface User { name: string; }\n".to_string(),
        }];

        // Write different content to disk
        std::fs::write(dir.path().join("user.ts"), "// stale content\n").unwrap();

        let code = run_check(dir.path(), &files);
        assert_eq!(code, 1, "drift detected should exit with code 1");
    }

    #[test]
    fn check_mode_writes_no_files() {
        let dir = tempfile::tempdir().unwrap();
        let files = vec![GeneratedFile {
            path: "new-file.ts".to_string(),
            content: "export interface New {}\n".to_string(),
        }];

        // Directory is empty — file doesn't exist
        let code = run_check(dir.path(), &files);
        assert_eq!(code, 1, "missing file should exit with code 1");

        // Verify no files were written
        assert!(
            !dir.path().join("new-file.ts").exists(),
            "check mode MUST NOT write any files"
        );
    }

    #[test]
    fn multiple_gen_blocks_produce_independent_outputs() {
        let ts_gen = specforge_codegen::resolve_generator("typescript");
        let json_gen = specforge_codegen::resolve_generator("json-schema");

        assert_eq!(ts_gen.name(), "typescript");
        assert_eq!(json_gen.name(), "json-schema");

        // Build a graph with a type entity
        let source = r#"spec "test" {
  version "1.0"
  plugins []
}
type UserProfile {
  name  string
  email string @optional
}"#;

        let parsed = specforge_parser::parse(source, "test.spec");
        let resolved = specforge_resolver::resolve(vec![parsed], ".");
        let graph_result = specforge_graph::build_graph(&resolved.files);

        let ts_config = specforge_common::GenConfig {
            name: "typescript".to_string(),
            out: "generated/ts".to_string(),
            result_style: specforge_common::ResultStyle::default(),
            readonly: false,
            naming: specforge_common::NamingStyle::default(),
            tests: None,
            extra: HashMap::new(),
        };
        let json_config = specforge_common::GenConfig {
            name: "json-schema".to_string(),
            out: "generated/json".to_string(),
            result_style: specforge_common::ResultStyle::default(),
            readonly: false,
            naming: specforge_common::NamingStyle::default(),
            tests: None,
            extra: HashMap::new(),
        };

        let ts_ctx = specforge_codegen::GenerateContext {
            graph: &graph_result.graph,
            files: &resolved.files,
            config: &resolved.config,
            gen_config: &ts_config,
        };
        let json_ctx = specforge_codegen::GenerateContext {
            graph: &graph_result.graph,
            files: &resolved.files,
            config: &resolved.config,
            gen_config: &json_config,
        };

        let ts_result = ts_gen.generate(&ts_ctx);
        let json_result = json_gen.generate(&json_ctx);

        // Both produce files
        assert!(!ts_result.files.is_empty(), "TypeScript generator should produce files");
        assert!(!json_result.files.is_empty(), "JSON Schema generator should produce files");

        // Files are different (TS produces .ts, JSON Schema produces .json)
        let ts_paths: Vec<_> = ts_result.files.iter().map(|f| &f.path).collect();
        let json_paths: Vec<_> = json_result.files.iter().map(|f| &f.path).collect();
        assert_ne!(ts_paths, json_paths, "independent generators must produce different files");
    }

    #[test]
    fn language_specific_settings_are_isolated() {
        let source = r#"spec "test" {
  version "1.0"
  plugins []
}
type UserProfile {
  user_name string
}"#;

        let parsed = specforge_parser::parse(source, "test.spec");
        let resolved = specforge_resolver::resolve(vec![parsed], ".");
        let graph_result = specforge_graph::build_graph(&resolved.files);

        // TypeScript with CamelCase naming
        let ts_config = specforge_common::GenConfig {
            name: "typescript".to_string(),
            out: "generated/ts".to_string(),
            result_style: specforge_common::ResultStyle::default(),
            readonly: false,
            naming: specforge_common::NamingStyle::CamelCase,
            tests: None,
            extra: HashMap::new(),
        };

        // JSON Schema with default (PascalCase) naming
        let json_config = specforge_common::GenConfig {
            name: "json-schema".to_string(),
            out: "generated/json".to_string(),
            result_style: specforge_common::ResultStyle::default(),
            readonly: false,
            naming: specforge_common::NamingStyle::default(),
            tests: None,
            extra: HashMap::new(),
        };

        let ts_gen = specforge_codegen::resolve_generator("typescript");
        let json_gen = specforge_codegen::resolve_generator("json-schema");

        let ts_ctx = specforge_codegen::GenerateContext {
            graph: &graph_result.graph,
            files: &resolved.files,
            config: &resolved.config,
            gen_config: &ts_config,
        };
        let json_ctx = specforge_codegen::GenerateContext {
            graph: &graph_result.graph,
            files: &resolved.files,
            config: &resolved.config,
            gen_config: &json_config,
        };

        let ts_result = ts_gen.generate(&ts_ctx);
        let json_result = json_gen.generate(&json_ctx);

        // TS with CamelCase should have "userName" in output
        let ts_content: String = ts_result.files.iter().map(|f| f.content.clone()).collect();
        assert!(
            ts_content.contains("userName"),
            "TypeScript CamelCase config should produce camelCase field names, got: {ts_content}"
        );

        // JSON Schema uses the original field name (user_name) in property keys
        let json_content: String = json_result.files.iter().map(|f| f.content.clone()).collect();
        assert!(
            json_content.contains("user_name"),
            "JSON Schema should use original field names, got: {json_content}"
        );
    }
}
