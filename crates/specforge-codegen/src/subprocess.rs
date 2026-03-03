use crate::generator::{GenerateContext, GenerateResult, Generator, PluginError};
use specforge_emitter::GeneratedFile;
use std::collections::HashMap;
use std::process::Command;

/// A generator that delegates to an external `specforge-gen-{name}` executable.
pub struct SubprocessGenerator {
    pub name: String,
}

impl Generator for SubprocessGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn generate(&self, ctx: &GenerateContext) -> GenerateResult {
        let bin_name = format!("specforge-gen-{}", self.name);

        let bin_path = match which::which(&bin_name) {
            Ok(path) => path,
            Err(_) => {
                return GenerateResult {
                    files: Vec::new(),
                    warnings: Vec::new(),
                    errors: vec![PluginError::NotFound {
                        plugin_name: self.name.clone(),
                        message: format!("`{bin_name}` not on PATH"),
                    }],
                    entity_checksums: HashMap::new(),
                };
            }
        };

        // Serialize graph + config as JSON to stdin
        let input = build_subprocess_input(ctx);

        let result = Command::new(&bin_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(input.as_bytes())?;
                }
                child.wait_with_output()
            });

        match result {
            Ok(output) => {
                if !output.status.success() {
                    let stderr_text = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    return GenerateResult {
                        files: Vec::new(),
                        warnings: Vec::new(),
                        errors: vec![PluginError::ExecutionFailed {
                            plugin_name: self.name.clone(),
                            message: format!(
                                "exit code {}",
                                output.status.code().unwrap_or(-1)
                            ),
                            stderr: if stderr_text.is_empty() { None } else { Some(stderr_text) },
                        }],
                        entity_checksums: HashMap::new(),
                    };
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                match serde_json::from_str::<Vec<GeneratedFile>>(&stdout) {
                    Ok(files) => GenerateResult {
                        files,
                        warnings: Vec::new(),
                        errors: Vec::new(),
                        entity_checksums: HashMap::new(),
                    },
                    Err(e) => GenerateResult {
                        files: Vec::new(),
                        warnings: Vec::new(),
                        errors: vec![PluginError::InvalidOutput {
                            plugin_name: self.name.clone(),
                            message: e.to_string(),
                        }],
                        entity_checksums: HashMap::new(),
                    },
                }
            }
            Err(e) => GenerateResult {
                files: Vec::new(),
                warnings: Vec::new(),
                errors: vec![PluginError::SpawnFailed {
                    plugin_name: self.name.clone(),
                    message: e.to_string(),
                }],
                entity_checksums: HashMap::new(),
            },
        }
    }
}

/// Build the JSON input sent to external generators on stdin.
fn build_subprocess_input(ctx: &GenerateContext) -> String {
    let graph_json = specforge_emitter::render_json(ctx.graph, ctx.files, ctx.config);
    // Wrap graph JSON + gen config extras into a single object
    let mut extra = serde_json::Map::new();
    for (k, v) in &ctx.gen_config.extra {
        extra.insert(k.clone(), serde_json::Value::String(v.clone()));
    }
    let input = serde_json::json!({
        "graph": serde_json::from_str::<serde_json::Value>(&graph_json.content).unwrap_or_default(),
        "config": {
            "name": ctx.gen_config.name,
            "out": ctx.gen_config.out,
        },
        "extra": extra,
    });
    serde_json::to_string(&input).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_crash_produces_plugin_error() {
        let graph = specforge_graph::SpecGraph::new();
        let config = specforge_common::CompilerConfig::core_only("test");
        let gen_config = specforge_common::GenConfig {
            name: "nonexistent-generator-xyz".to_string(),
            out: "/tmp/test-out".to_string(),
            result_style: specforge_common::ResultStyle::default(),
            readonly: false,
            naming: specforge_common::NamingStyle::default(),
            tests: None,
            extra: HashMap::new(),
        };
        let ctx = GenerateContext {
            graph: &graph,
            files: &[],
            config: &config,
            gen_config: &gen_config,
        };

        let generator = SubprocessGenerator {
            name: "nonexistent-generator-xyz".to_string(),
        };
        let result = generator.generate(&ctx);

        assert!(result.files.is_empty());
        assert!(result.warnings.is_empty());
        assert_eq!(result.errors.len(), 1);
        match &result.errors[0] {
            PluginError::NotFound { plugin_name, .. } => {
                assert_eq!(plugin_name, "nonexistent-generator-xyz");
            }
            other => panic!("expected NotFound, got: {other:?}"),
        }
    }

    #[test]
    fn compiler_sends_graph_on_stdin() {
        let graph = specforge_graph::SpecGraph::new();
        let config = specforge_common::CompilerConfig::core_only("test");
        let mut extra = HashMap::new();
        extra.insert("target".to_string(), "es2022".to_string());
        let gen_config = specforge_common::GenConfig {
            name: "custom".to_string(),
            out: "/tmp/out".to_string(),
            result_style: specforge_common::ResultStyle::default(),
            readonly: false,
            naming: specforge_common::NamingStyle::default(),
            tests: None,
            extra,
        };
        let ctx = GenerateContext {
            graph: &graph,
            files: &[],
            config: &config,
            gen_config: &gen_config,
        };

        let input = build_subprocess_input(&ctx);
        let parsed: serde_json::Value = serde_json::from_str(&input).unwrap();

        // Input must contain "graph", "config", and "extra" top-level keys
        assert!(parsed.get("graph").is_some(), "stdin must contain graph JSON");
        assert!(parsed.get("config").is_some(), "stdin must contain config");
        assert!(parsed.get("extra").is_some(), "stdin must contain extra settings");

        // Config must include name and out
        let config_obj = parsed.get("config").unwrap();
        assert_eq!(config_obj.get("name").unwrap(), "custom");
        assert_eq!(config_obj.get("out").unwrap(), "/tmp/out");

        // Extra settings must be passed through
        let extra_obj = parsed.get("extra").unwrap();
        assert_eq!(extra_obj.get("target").unwrap(), "es2022");
    }

    #[test]
    fn compiler_reads_files_from_stdout() {
        // Simulate what a plugin would write to stdout: a JSON array of GeneratedFile
        let stdout_json = r#"[
            {"path": "user.ts", "content": "export interface User {}\n"},
            {"path": "order.ts", "content": "export interface Order {}\n"}
        ]"#;

        let files: Vec<GeneratedFile> = serde_json::from_str(stdout_json).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "user.ts");
        assert!(files[0].content.contains("User"));
        assert_eq!(files[1].path, "order.ts");
        assert!(files[1].content.contains("Order"));
    }
}
