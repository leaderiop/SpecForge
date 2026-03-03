use crate::pipeline;
use clap::Args;
use specforge_codegen::checksum;
use specforge_common::{EntityKind, FieldMap};
use std::path::PathBuf;

#[derive(Args)]
pub struct VerifyArgs {
    /// Generator name (e.g., "typescript")
    pub generator: String,

    /// Output directory to scan for generated files
    pub dir: Option<PathBuf>,

    /// Directory containing hand-written adapter files (defaults to output dir)
    #[arg(long)]
    pub adapters_dir: Option<PathBuf>,

    /// Path to spec files. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

pub fn run(args: VerifyArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let gen_config = result
        .config
        .gen_configs
        .iter()
        .find(|c| c.name == args.generator);

    let out_dir = args
        .dir
        .or_else(|| gen_config.map(|c| PathBuf::from(&c.out)))
        .unwrap_or_else(|| PathBuf::from(format!("generated/{}", args.generator)));

    if !out_dir.exists() {
        eprintln!("specforge: output directory does not exist: {}", out_dir.display());
        return 1;
    }

    // Collect port entity names
    let port_nodes = result.graph.nodes_of_kind(EntityKind::Port);
    if port_nodes.is_empty() {
        eprintln!("specforge: no port entities found — nothing to verify");
        return 0;
    }

    // Build entity_fields map for adapter scanning
    let entity_fields = build_entity_fields(&result.files);

    let adapters_dir = args.adapters_dir.as_ref().unwrap_or(&out_dir);

    let mut missing = 0;
    let mut tampered = 0;
    let mut verified = 0;
    let mut adapters_found = 0;
    let mut methods_missing = 0;
    let mut adapters_not_found = 0;

    for node in &port_nodes {
        let raw_id = node.id.raw();
        let file_stem = specforge_codegen::naming::to_file_name(raw_id);
        let generated_path = out_dir.join(format!("{file_stem}.ts"));

        // Checksum verification
        if !generated_path.exists() {
            eprintln!("specforge: missing generated file: {}", generated_path.display());
            missing += 1;
            continue;
        }

        match std::fs::read_to_string(&generated_path) {
            Ok(content) => {
                if checksum::verify_checksum(&content) {
                    verified += 1;
                } else {
                    eprintln!("specforge: checksum mismatch: {}", generated_path.display());
                    tampered += 1;
                }
            }
            Err(e) => {
                eprintln!("specforge: cannot read {}: {e}", generated_path.display());
                missing += 1;
            }
        }

        // Adapter method scanning
        let expected_methods = collect_port_methods(entity_fields.get(raw_id));
        if expected_methods.is_empty() {
            continue;
        }

        let adapter_path = find_adapter_file(adapters_dir, &file_stem);
        match adapter_path {
            Some(path) => {
                adapters_found += 1;
                if let Ok(adapter_content) = std::fs::read_to_string(&path) {
                    let found_methods = scan_adapter_methods(&adapter_content);
                    for method in &expected_methods {
                        if !found_methods.iter().any(|m| m == method) {
                            eprintln!(
                                "specforge: adapter {} missing method `{method}`",
                                path.display()
                            );
                            methods_missing += 1;
                        }
                    }
                }
            }
            None => {
                adapters_not_found += 1;
            }
        }
    }

    eprintln!(
        "specforge: {verified} verified, {tampered} tampered, {missing} missing ({} port(s) total)",
        port_nodes.len()
    );

    if adapters_found > 0 || adapters_not_found > 0 {
        eprintln!(
            "specforge: {adapters_found} adapter(s) found, {adapters_not_found} not found, {methods_missing} method(s) missing"
        );
    }

    if missing > 0 || tampered > 0 || methods_missing > 0 {
        1
    } else {
        0
    }
}

fn build_entity_fields(files: &[specforge_parser::SpecFile]) -> std::collections::HashMap<&str, &FieldMap> {
    let mut map = std::collections::HashMap::new();
    for file in files {
        for entity in &file.entities {
            map.insert(entity.id.raw(), &entity.fields);
        }
    }
    map
}

/// Collect expected method names from `method:*` keys in entity fields.
fn collect_port_methods(fields: Option<&&FieldMap>) -> Vec<String> {
    let Some(fields) = fields else {
        return Vec::new();
    };
    fields
        .iter()
        .filter_map(|(key, _)| key.strip_prefix("method:").map(|m| m.to_string()))
        .collect()
}

/// Look for adapter files matching `{file_stem}.adapter.ts` or `{file_stem}.impl.ts`.
fn find_adapter_file(dir: &std::path::Path, file_stem: &str) -> Option<PathBuf> {
    for suffix in &["adapter.ts", "impl.ts"] {
        let path = dir.join(format!("{file_stem}.{suffix}"));
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Scan adapter file content for method names using simple text matching.
/// Looks for patterns like `methodName(` or `methodName =` or `async methodName(`.
fn scan_adapter_methods(content: &str) -> Vec<String> {
    let mut methods = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        // Strip leading keywords: async, public, private, protected
        let stripped = trimmed
            .strip_prefix("async ")
            .unwrap_or(trimmed);
        let stripped = stripped
            .strip_prefix("public ")
            .or_else(|| stripped.strip_prefix("private "))
            .or_else(|| stripped.strip_prefix("protected "))
            .unwrap_or(stripped);
        let stripped = stripped
            .strip_prefix("async ")
            .unwrap_or(stripped);

        // Match `name(` pattern (method definition)
        if let Some(paren_pos) = stripped.find('(') {
            let candidate = stripped[..paren_pos].trim();
            if is_valid_identifier(candidate) {
                methods.push(candidate.to_string());
            }
        }
        // Match `name =` pattern (arrow function assignment)
        else if let Some(eq_pos) = stripped.find(" = ") {
            let candidate = stripped[..eq_pos].trim();
            if is_valid_identifier(candidate) {
                methods.push(candidate.to_string());
            }
        }
    }
    methods
}

fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(' ')
        && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::FieldValue;

    #[test]
    fn scan_adapter_finds_methods() {
        let content = "\
import { FileSystem } from './file-system';

export class FileSystemAdapter implements FileSystem {
  async readFile(path: string): Promise<string> {
    return fs.readFile(path, 'utf-8');
  }

  async writeFile(path: string, data: string): Promise<void> {
    await fs.writeFile(path, data);
  }

  async deleteFile(path: string): Promise<void> {
    await fs.unlink(path);
  }
}
";
        let methods = scan_adapter_methods(content);
        assert!(methods.contains(&"readFile".to_string()));
        assert!(methods.contains(&"writeFile".to_string()));
        assert!(methods.contains(&"deleteFile".to_string()));
    }

    #[test]
    fn scan_adapter_skips_comments() {
        let content = "\
// this is a comment mentioning save(
/* block comment with save( */
  save(data: string): void {
  }
";
        let methods = scan_adapter_methods(content);
        assert_eq!(methods, vec!["save".to_string()]);
    }

    #[test]
    fn collect_port_methods_from_fields() {
        let mut fields = FieldMap::new();
        fields.insert("direction", FieldValue::Enum("outbound".to_string()));
        fields.insert(
            "method:readFile",
            FieldValue::String("method readFile(path: string) -> string".to_string()),
        );
        fields.insert(
            "method:writeFile",
            FieldValue::String("method writeFile(path: string, data: string) -> void".to_string()),
        );

        let methods = collect_port_methods(Some(&&fields));
        assert!(methods.contains(&"readFile".to_string()));
        assert!(methods.contains(&"writeFile".to_string()));
        assert_eq!(methods.len(), 2);
    }

    #[test]
    fn adapter_extra_methods_allowed() {
        // An adapter with extra methods beyond what the port requires should be fine
        let content = "\
  save(data: string): void {}
  findById(id: string): User {}
  helperMethod(x: number): void {}
";
        let found = scan_adapter_methods(content);
        let expected = vec!["save".to_string(), "findById".to_string()];
        // All expected methods are present → extra methods don't cause issues
        for m in &expected {
            assert!(found.contains(m), "expected method `{m}` not found");
        }
        // Extra method is also found — that's fine per contract
        assert!(found.contains(&"helperMethod".to_string()));
    }

    #[test]
    fn find_adapter_file_checks_both_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let stem = "file-system";

        // No adapter file exists
        assert!(find_adapter_file(dir.path(), stem).is_none());

        // Create .adapter.ts
        std::fs::write(dir.path().join("file-system.adapter.ts"), "class Foo {}").unwrap();
        assert!(find_adapter_file(dir.path(), stem).is_some());

        // Remove .adapter.ts, create .impl.ts
        std::fs::remove_file(dir.path().join("file-system.adapter.ts")).unwrap();
        std::fs::write(dir.path().join("file-system.impl.ts"), "class Foo {}").unwrap();
        assert!(find_adapter_file(dir.path(), stem).is_some());
    }
}
