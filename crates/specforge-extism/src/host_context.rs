use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use specforge_common::Diagnostic;
use specforge_wasm::{
    default_sandbox_policy, filter_graph_by_query_scope, host_emit_diagnostic,
    host_read_file_check, CallSite, QueryScope,
};

pub struct HostContext {
    pub diagnostics: Arc<Mutex<Vec<Diagnostic>>>,
    pub spec_root: Option<PathBuf>,
    pub graph_json: Option<serde_json::Value>,
}

impl HostContext {
    pub fn new(diagnostics: Arc<Mutex<Vec<Diagnostic>>>) -> Self {
        Self {
            diagnostics,
            spec_root: None,
            graph_json: None,
        }
    }

    pub fn with_spec_root(mut self, spec_root: PathBuf) -> Self {
        self.spec_root = Some(spec_root);
        self
    }

    pub fn with_graph(mut self, graph_json: serde_json::Value) -> Self {
        self.graph_json = Some(graph_json);
        self
    }
}

impl Default for HostContext {
    fn default() -> Self {
        Self {
            diagnostics: Arc::new(Mutex::new(Vec::new())),
            spec_root: None,
            graph_json: None,
        }
    }
}

impl Clone for HostContext {
    fn clone(&self) -> Self {
        Self {
            diagnostics: self.diagnostics.clone(),
            spec_root: self.spec_root.clone(),
            graph_json: self.graph_json.clone(),
        }
    }
}

pub(crate) fn build_host_functions(ctx: HostContext) -> Vec<extism::Function> {
    vec![
        make_emit_diagnostic_fn(ctx.clone()),
        make_read_file_fn(ctx.clone()),
        make_query_graph_fn(ctx),
    ]
}

fn make_emit_diagnostic_fn(ctx: HostContext) -> extism::Function {
    use extism::{CurrentPlugin, Function, UserData, Val, ValType};

    Function::new(
        "host_emit_diagnostic",
        [ValType::I64],
        [ValType::I64],
        UserData::new(ctx),
        |plugin: &mut CurrentPlugin,
         inputs: &[Val],
         outputs: &mut [Val],
         user_data: UserData<HostContext>| {
            let input_bytes: Vec<u8> = plugin.memory_get_val(&inputs[0])?;
            let ctx_arc = user_data.get()?;
            let ctx = ctx_arc.lock().unwrap();

            match host_emit_diagnostic("wasm-extension", &input_bytes) {
                Ok(diag) => {
                    let mut diags = ctx.diagnostics.lock().unwrap();
                    diags.push(diag);
                    let response = serde_json::json!({"ok": true}).to_string();
                    let handle = plugin.memory_new(response)?;
                    outputs[0] = plugin.memory_to_val(handle);
                }
                Err(err_diag) => {
                    let mut diags = ctx.diagnostics.lock().unwrap();
                    diags.push(err_diag);
                    let response =
                        serde_json::json!({"ok": false, "error": "malformed diagnostic"})
                            .to_string();
                    let handle = plugin.memory_new(response)?;
                    outputs[0] = plugin.memory_to_val(handle);
                }
            }

            Ok(())
        },
    )
    .with_namespace("extism:host/user")
}

fn make_read_file_fn(ctx: HostContext) -> extism::Function {
    use extism::{CurrentPlugin, Function, UserData, Val, ValType};

    Function::new(
        "host_read_file",
        [ValType::I64],
        [ValType::I64],
        UserData::new(ctx),
        |plugin: &mut CurrentPlugin,
         inputs: &[Val],
         outputs: &mut [Val],
         user_data: UserData<HostContext>| {
            let input_bytes: Vec<u8> = plugin.memory_get_val(&inputs[0])?;
            let ctx_arc = user_data.get()?;
            let ctx = ctx_arc.lock().unwrap();

            let request: serde_json::Value = serde_json::from_slice(&input_bytes)
                .map_err(|e| extism::Error::msg(format!("invalid JSON: {e}")))?;

            let path_str = request["path"]
                .as_str()
                .ok_or_else(|| extism::Error::msg("missing 'path' field"))?;
            let path = std::path::Path::new(path_str);

            let spec_root = match &ctx.spec_root {
                Some(root) => root.clone(),
                None => {
                    let response =
                        serde_json::json!({"error": "no spec_root configured"}).to_string();
                    let handle = plugin.memory_new(response)?;
                    outputs[0] = plugin.memory_to_val(handle);
                    return Ok(());
                }
            };

            let policy = default_sandbox_policy();
            if let Err(diag) = host_read_file_check(
                "wasm-extension",
                path,
                &spec_root,
                CallSite::Validator,
                &policy,
            ) {
                let mut diags = ctx.diagnostics.lock().unwrap();
                diags.push(diag);
                let response = serde_json::json!({"error": "permission denied"}).to_string();
                let handle = plugin.memory_new(response)?;
                outputs[0] = plugin.memory_to_val(handle);
                return Ok(());
            }

            match std::fs::read_to_string(path) {
                Ok(contents) => {
                    let response = serde_json::json!({"contents": contents}).to_string();
                    let handle = plugin.memory_new(response)?;
                    outputs[0] = plugin.memory_to_val(handle);
                }
                Err(e) => {
                    let response =
                        serde_json::json!({"error": format!("read failed: {e}")}).to_string();
                    let handle = plugin.memory_new(response)?;
                    outputs[0] = plugin.memory_to_val(handle);
                }
            }

            Ok(())
        },
    )
    .with_namespace("extism:host/user")
}

fn make_query_graph_fn(ctx: HostContext) -> extism::Function {
    use extism::{CurrentPlugin, Function, UserData, Val, ValType};

    Function::new(
        "host_query_graph",
        [ValType::I64],
        [ValType::I64],
        UserData::new(ctx),
        |plugin: &mut CurrentPlugin,
         _inputs: &[Val],
         outputs: &mut [Val],
         user_data: UserData<HostContext>| {
            let ctx_arc = user_data.get()?;
            let ctx = ctx_arc.lock().unwrap();

            let graph = match &ctx.graph_json {
                Some(g) => g.clone(),
                None => serde_json::json!({"entities": [], "edges": []}),
            };

            let filtered = filter_graph_by_query_scope(&graph, &QueryScope::All);
            let response = serde_json::to_string(&filtered)
                .map_err(|e| extism::Error::msg(format!("failed to serialize graph: {e}")))?;
            let handle = plugin.memory_new(response)?;
            outputs[0] = plugin.memory_to_val(handle);

            Ok(())
        },
    )
    .with_namespace("extism:host/user")
}
