use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, EntityId, SourceSpan, ValidationCode};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::manifest::{PackageContributions, SandboxPolicy};
use crate::sandbox;

/// Shared state accessible by all host functions for a single package invocation.
#[derive(Debug)]
pub struct HostContext {
    /// Package name of the plugin (for error messages).
    pub package: String,
    /// Pre-serialized graph JSON for `query_graph`.
    pub graph_json: String,
    /// Diagnostics emitted by the plugin via `emit_diagnostic`.
    pub diagnostics: Vec<Diagnostic>,
    /// Entity kinds registered by the plugin via `register_entity`.
    pub registered_entities: Vec<RegisteredEntity>,
    /// Edge types registered by the plugin via `register_edge`.
    pub registered_edges: Vec<RegisteredEdge>,
    /// Files emitted by the plugin via `emit_file`.
    pub emitted_files: Vec<EmittedFile>,
    /// Sandbox policy for validation.
    pub sandbox_policy: SandboxPolicy,
    /// Contribution permissions for this package.
    pub contributions: PackageContributions,
    /// Allowed output directory for file emission.
    pub output_dir: Option<PathBuf>,
    /// Whether we are in the initialization phase (register_entity/edge allowed).
    pub in_initialize: bool,
}

/// An entity kind registered by a Wasm package at initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredEntity {
    pub name: String,
    #[serde(default)]
    pub fields: Vec<RegisteredField>,
    #[serde(default)]
    pub reference_targets: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub testable: bool,
    /// Set by the host function from HostContext.package — not sent by the plugin.
    #[serde(skip)]
    pub source_plugin: String,
}

/// A field in a registered entity kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
}

/// An edge type registered by a Wasm package at initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredEdge {
    pub name: String,
    pub source_kind: String,
    pub target_kind: String,
    #[serde(default)]
    pub soft: bool,
}

/// A file emitted by a Wasm package via the `emit_file` host function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmittedFile {
    pub path: String,
    pub content: String,
}

/// Input to the `emit_diagnostic` host function.
#[derive(Debug, Deserialize)]
struct DiagnosticInput {
    severity: String,
    #[allow(dead_code)]
    code: Option<String>,
    message: String,
    file: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
}

/// Input to the `emit_file` host function.
#[derive(Debug, Deserialize)]
struct EmitFileInput {
    path: String,
    content: String,
}

/// Input to the `http_get` host function.
#[derive(Debug, Deserialize)]
struct HttpGetInput {
    url: String,
}

impl HostContext {
    pub fn new(
        package: &str,
        sandbox_policy: SandboxPolicy,
        contributions: PackageContributions,
    ) -> Self {
        Self {
            package: package.to_string(),
            graph_json: String::new(),
            diagnostics: Vec::new(),
            registered_entities: Vec::new(),
            registered_edges: Vec::new(),
            emitted_files: Vec::new(),
            sandbox_policy,
            contributions,
            output_dir: None,
            in_initialize: false,
        }
    }

    /// Take all collected diagnostics, leaving the vec empty.
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Take all registered entities, leaving the vec empty.
    pub fn take_registered_entities(&mut self) -> Vec<RegisteredEntity> {
        std::mem::take(&mut self.registered_entities)
    }

    /// Take all registered edges, leaving the vec empty.
    pub fn take_registered_edges(&mut self) -> Vec<RegisteredEdge> {
        std::mem::take(&mut self.registered_edges)
    }

    /// Take all emitted files, leaving the vec empty.
    pub fn take_emitted_files(&mut self) -> Vec<EmittedFile> {
        std::mem::take(&mut self.emitted_files)
    }
}

/// Build the set of Extism host functions wired to the shared context.
///
/// Returns the functions and the shared context handle.
pub fn build_host_functions(
    package: &str,
    sandbox_policy: SandboxPolicy,
    contributions: PackageContributions,
) -> (Vec<extism::Function>, Arc<Mutex<HostContext>>) {
    let user_data: extism::UserData<HostContext> =
        extism::UserData::new(HostContext::new(package, sandbox_policy, contributions));
    let ctx_handle = user_data.get().expect("UserData was just created");

    let functions = vec![
        build_query_graph(user_data.clone()),
        build_emit_diagnostic(user_data.clone()),
        build_register_entity(user_data.clone()),
        build_register_edge(user_data.clone()),
        build_emit_file(user_data.clone()),
        build_http_get(user_data),
    ];

    (functions, ctx_handle)
}

fn build_query_graph(user_data: extism::UserData<HostContext>) -> extism::Function {
    extism::Function::new(
        "specforge.query_graph",
        [extism::ValType::I64],
        [extism::ValType::I64],
        user_data,
        |plugin: &mut extism::CurrentPlugin,
         _inputs: &[extism::Val],
         outputs: &mut [extism::Val],
         ud: extism::UserData<HostContext>| {
            let ctx = ud.get()?;
            let graph_json = ctx.lock().unwrap().graph_json.clone();
            let handle = plugin.memory_new(graph_json.as_str())?;
            outputs[0] = plugin.memory_to_val(handle);
            Ok(())
        },
    )
}

fn build_emit_diagnostic(user_data: extism::UserData<HostContext>) -> extism::Function {
    extism::Function::new(
        "specforge.emit_diagnostic",
        [extism::ValType::I64],
        [],
        user_data,
        |plugin: &mut extism::CurrentPlugin,
         inputs: &[extism::Val],
         _outputs: &mut [extism::Val],
         ud: extism::UserData<HostContext>| {
            let handle = plugin
                .memory_from_val(&inputs[0])
                .ok_or_else(|| extism::Error::msg("invalid memory handle for emit_diagnostic"))?;
            let data = plugin.memory_bytes(handle)?.to_vec();
            plugin.memory_free(handle)?;

            let input: DiagnosticInput = serde_json::from_slice(&data)
                .map_err(|e| extism::Error::msg(format!("invalid diagnostic JSON: {e}")))?;

            let span = match (&input.file, input.line, input.column) {
                (Some(file), Some(line), Some(col)) => {
                    SourceSpan::new(file, line, col, line, col)
                }
                (Some(file), Some(line), None) => SourceSpan::new(file, line, 1, line, 1),
                (Some(file), _, _) => SourceSpan::file_start(file),
                _ => SourceSpan::file_start("<wasm-plugin>"),
            };

            let code = match input.severity.as_str() {
                "error" => ValidationCode::E019,
                "warning" => ValidationCode::W025,
                "info" => ValidationCode::I007,
                _ => ValidationCode::W025,
            };

            let diagnostic = Diagnostic::new(code, span, input.message);
            let ctx = ud.get()?;
            ctx.lock().unwrap().diagnostics.push(diagnostic);
            Ok(())
        },
    )
}

fn build_register_entity(user_data: extism::UserData<HostContext>) -> extism::Function {
    extism::Function::new(
        "specforge.register_entity",
        [extism::ValType::I64],
        [],
        user_data,
        |plugin: &mut extism::CurrentPlugin,
         inputs: &[extism::Val],
         _outputs: &mut [extism::Val],
         ud: extism::UserData<HostContext>| {
            let handle = plugin
                .memory_from_val(&inputs[0])
                .ok_or_else(|| extism::Error::msg("invalid memory handle for register_entity"))?;
            let data = plugin.memory_bytes(handle)?.to_vec();
            plugin.memory_free(handle)?;

            let mut entity: RegisteredEntity = serde_json::from_slice(&data)
                .map_err(|e| extism::Error::msg(format!("invalid register_entity JSON: {e}")))?;

            let ctx = ud.get()?;
            let mut ctx = ctx.lock().unwrap();
            if !ctx.in_initialize {
                return Err(extism::Error::msg(
                    "register_entity can only be called during initialize",
                ));
            }

            if !ctx.contributions.entities {
                return Err(extism::Error::msg(
                    "register_entity requires `contributes.entities: true` in manifest",
                ));
            }

            // Layer 1: Guard — reject reserved/syntax keywords
            if EntityId::is_reserved_or_syntax_keyword(&entity.name) {
                return Err(extism::Error::msg(format!(
                    "entity kind name `{}` is a reserved keyword and cannot be registered",
                    entity.name
                )));
            }

            // Validate identifier format
            if !EntityId::is_valid_identifier(&entity.name) {
                return Err(extism::Error::msg(format!(
                    "entity kind name `{}` is not a valid identifier (2-60 chars, letters/digits/underscores, starts with letter)",
                    entity.name
                )));
            }

            // Set source plugin from host context
            entity.source_plugin = ctx.package.clone();

            ctx.registered_entities.push(entity);
            Ok(())
        },
    )
}

fn build_register_edge(user_data: extism::UserData<HostContext>) -> extism::Function {
    extism::Function::new(
        "specforge.register_edge",
        [extism::ValType::I64],
        [],
        user_data,
        |plugin: &mut extism::CurrentPlugin,
         inputs: &[extism::Val],
         _outputs: &mut [extism::Val],
         ud: extism::UserData<HostContext>| {
            let handle = plugin
                .memory_from_val(&inputs[0])
                .ok_or_else(|| extism::Error::msg("invalid memory handle for register_edge"))?;
            let data = plugin.memory_bytes(handle)?.to_vec();
            plugin.memory_free(handle)?;

            let edge: RegisteredEdge = serde_json::from_slice(&data)
                .map_err(|e| extism::Error::msg(format!("invalid register_edge JSON: {e}")))?;

            let ctx = ud.get()?;
            let mut ctx = ctx.lock().unwrap();
            if !ctx.in_initialize {
                return Err(extism::Error::msg(
                    "register_edge can only be called during initialize",
                ));
            }
            ctx.registered_edges.push(edge);
            Ok(())
        },
    )
}

fn build_emit_file(user_data: extism::UserData<HostContext>) -> extism::Function {
    extism::Function::new(
        "specforge.emit_file",
        [extism::ValType::I64],
        [],
        user_data,
        |plugin: &mut extism::CurrentPlugin,
         inputs: &[extism::Val],
         _outputs: &mut [extism::Val],
         ud: extism::UserData<HostContext>| {
            let handle = plugin
                .memory_from_val(&inputs[0])
                .ok_or_else(|| extism::Error::msg("invalid memory handle for emit_file"))?;
            let data = plugin.memory_bytes(handle)?.to_vec();
            plugin.memory_free(handle)?;

            let input: EmitFileInput = serde_json::from_slice(&data)
                .map_err(|e| extism::Error::msg(format!("invalid emit_file JSON: {e}")))?;

            let ctx = ud.get()?;
            let ctx_guard = ctx.lock().unwrap();

            if !ctx_guard.contributions.generators {
                return Err(extism::Error::msg(
                    "emit_file requires `contributes.generators: true` in manifest",
                ));
            }

            let output_dir = ctx_guard
                .output_dir
                .as_deref()
                .unwrap_or(std::path::Path::new("."));

            sandbox::validate_file_path(&input.path, &ctx_guard.sandbox_policy, output_dir)
                .map_err(extism::Error::msg)?;

            drop(ctx_guard);
            ctx.lock().unwrap().emitted_files.push(EmittedFile {
                path: input.path,
                content: input.content,
            });
            Ok(())
        },
    )
}

fn build_http_get(user_data: extism::UserData<HostContext>) -> extism::Function {
    extism::Function::new(
        "specforge.http_get",
        [extism::ValType::I64],
        [extism::ValType::I64],
        user_data,
        |plugin: &mut extism::CurrentPlugin,
         inputs: &[extism::Val],
         outputs: &mut [extism::Val],
         ud: extism::UserData<HostContext>| {
            let handle = plugin
                .memory_from_val(&inputs[0])
                .ok_or_else(|| extism::Error::msg("invalid memory handle for http_get"))?;
            let data = plugin.memory_bytes(handle)?.to_vec();
            plugin.memory_free(handle)?;

            let input: HttpGetInput = serde_json::from_slice(&data)
                .map_err(|e| extism::Error::msg(format!("invalid http_get JSON: {e}")))?;

            {
                let ctx = ud.get()?;
                let ctx_guard = ctx.lock().unwrap();
                sandbox::validate_domain(&input.url, &ctx_guard.sandbox_policy)
                    .map_err(extism::Error::msg)?;
            }

            let body: String = ureq::get(&input.url)
                .call()
                .map_err(|e| extism::Error::msg(format!("HTTP GET failed: {e}")))?
                .into_body()
                .read_to_string()
                .map_err(|e| extism::Error::msg(format!("failed to read response body: {e}")))?;

            let result_handle = plugin.memory_new(body.as_str())?;
            outputs[0] = plugin.memory_to_val(result_handle);
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_context_new() {
        let ctx = HostContext::new(
            "@test/plugin",
            SandboxPolicy::default(),
            PackageContributions::default(),
        );
        assert_eq!(ctx.package, "@test/plugin");
        assert!(ctx.diagnostics.is_empty());
        assert!(ctx.registered_entities.is_empty());
        assert!(ctx.registered_edges.is_empty());
        assert!(ctx.emitted_files.is_empty());
        assert!(!ctx.in_initialize);
        assert!(ctx.contributions.is_default());
    }

    #[test]
    fn host_context_take_diagnostics() {
        let mut ctx = HostContext::new(
            "test",
            SandboxPolicy::default(),
            PackageContributions::default(),
        );
        ctx.diagnostics.push(Diagnostic::new(
            ValidationCode::W025,
            SourceSpan::file_start("test.spec"),
            "test warning",
        ));
        let taken = ctx.take_diagnostics();
        assert_eq!(taken.len(), 1);
        assert!(ctx.diagnostics.is_empty());
    }

    #[test]
    fn registered_entity_serde() {
        let entity = RegisteredEntity {
            name: "microservice".to_string(),
            fields: vec![RegisteredField {
                name: "port".to_string(),
                field_type: "integer".to_string(),
                required: true,
            }],
            reference_targets: std::collections::HashMap::new(),
            testable: true,
            source_plugin: String::new(),
        };
        let json = serde_json::to_string(&entity).unwrap();
        let parsed: RegisteredEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "microservice");
        assert!(parsed.testable);
        assert_eq!(parsed.fields.len(), 1);
        // source_plugin is #[serde(skip)] so it's empty after deserialization
        assert!(parsed.source_plugin.is_empty());
    }

    #[test]
    fn reserved_keyword_guard() {
        // Verify that EntityId::is_reserved_or_syntax_keyword catches
        // both built-in and DSL syntax keywords
        assert!(EntityId::is_reserved_or_syntax_keyword("behavior"));
        assert!(EntityId::is_reserved_or_syntax_keyword("define"));
        assert!(EntityId::is_reserved_or_syntax_keyword("verify"));
        assert!(!EntityId::is_reserved_or_syntax_keyword("microservice"));
    }

    #[test]
    fn valid_identifier_guard() {
        assert!(EntityId::is_valid_identifier("microservice"));
        assert!(EntityId::is_valid_identifier("epic"));
        assert!(!EntityId::is_valid_identifier("a")); // too short
        assert!(!EntityId::is_valid_identifier("1abc")); // leading digit
    }

    #[test]
    fn registered_edge_serde() {
        let edge = RegisteredEdge {
            name: "adapts".to_string(),
            source_kind: "adapter".to_string(),
            target_kind: "port".to_string(),
            soft: false,
        };
        let json = serde_json::to_string(&edge).unwrap();
        let parsed: RegisteredEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "adapts");
        assert!(!parsed.soft);
    }

    #[test]
    fn build_host_functions_creates_six() {
        let (functions, ctx) = build_host_functions(
            "test",
            SandboxPolicy::default(),
            PackageContributions::default(),
        );
        assert_eq!(functions.len(), 6);
        assert_eq!(ctx.lock().unwrap().package, "test");
    }

    #[test]
    fn contribution_check_entities() {
        // With entities=false, the contribution check should block
        let ctx = HostContext::new(
            "test",
            SandboxPolicy::default(),
            PackageContributions::default(),
        );
        assert!(!ctx.contributions.entities);

        // With entities=true, the contribution check should allow
        let ctx = HostContext::new(
            "test",
            SandboxPolicy::default(),
            PackageContributions::from_kind(crate::manifest::PluginKind::Plugin),
        );
        assert!(ctx.contributions.entities);
    }

    #[test]
    fn contribution_check_generators() {
        let ctx = HostContext::new(
            "test",
            SandboxPolicy::default(),
            PackageContributions::from_kind(crate::manifest::PluginKind::Generator),
        );
        assert!(ctx.contributions.generators);
        assert!(!ctx.contributions.entities);
    }

    #[test]
    fn contribution_check_providers() {
        let ctx = HostContext::new(
            "test",
            SandboxPolicy::default(),
            PackageContributions::from_kind(crate::manifest::PluginKind::Provider),
        );
        assert!(ctx.contributions.providers);
        assert!(!ctx.contributions.generators);
    }
}
