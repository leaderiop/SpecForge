use std::path::PathBuf;

const CARGO_TOML_TEMPLATE: &str = r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
extism-pdk = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
"#;

const LIB_RS_TEMPLATE: &str = r#"use extism_pdk::*;
use serde::{Deserialize, Serialize};

/// Called once when the plugin is loaded. Register entities and edges here.
#[plugin_fn]
pub fn initialize(_input: String) -> FnResult<String> {
    // Example: register a custom entity kind
    // let entity = serde_json::json!({
    //     "name": "microservice",
    //     "testable": true,
    //     "fields": [{"name": "port", "type": "integer", "required": true}]
    // });
    // host_call("specforge.register_entity", &serde_json::to_string(&entity)?)?;

    Ok(String::new())
}

/// Called during the validation phase with the spec graph as JSON.
#[plugin_fn]
pub fn validate(_input: String) -> FnResult<String> {
    // Read the graph via host function
    // let graph_json = host_call("specforge.query_graph", "")?;
    // let graph: serde_json::Value = serde_json::from_str(&graph_json)?;

    // Emit diagnostics via host function
    // let diagnostic = serde_json::json!({
    //     "severity": "warning",
    //     "message": "example warning from plugin",
    //     "file": "example.spec",
    //     "line": 1
    // });
    // host_call("specforge.emit_diagnostic", &serde_json::to_string(&diagnostic)?)?;

    Ok(String::new())
}
"#;

const MANIFEST_JSON_TEMPLATE: &str = r#"{
    "package": "@specforge/{name}",
    "manifest_version": "1",
    "kind": "plugin",
    "wasm": "target/wasm32-wasip1/release/{name_underscore}.wasm",
    "version": "0.1.0",
    "description": "{name} plugin for SpecForge",
    "sandbox": {
        "max_memory_bytes": 268435456,
        "allow_emit_file": true,
        "allow_http": false
    },
    "peer_dependencies": {},
    "enhancements": [],
    "entity_kinds": []
}
"#;

const CARGO_CONFIG_TEMPLATE: &str = r#"[build]
target = "wasm32-wasip1"
"#;

pub fn run(name: &str, dir: Option<PathBuf>) -> i32 {
    let project_dir = dir.unwrap_or_else(|| PathBuf::from(name));

    if project_dir.exists() {
        eprintln!("specforge: directory {} already exists", project_dir.display());
        return 1;
    }

    // Create directory structure
    let dirs = [
        project_dir.clone(),
        project_dir.join("src"),
        project_dir.join("fixtures"),
        project_dir.join(".cargo"),
    ];

    for d in &dirs {
        if let Err(e) = std::fs::create_dir_all(d) {
            eprintln!("specforge: cannot create {}: {e}", d.display());
            return 1;
        }
    }

    let name_underscore = name.replace('-', "_");

    // Write files
    let files = [
        (
            project_dir.join("Cargo.toml"),
            CARGO_TOML_TEMPLATE.replace("{name}", name),
        ),
        (
            project_dir.join("src/lib.rs"),
            LIB_RS_TEMPLATE.to_string(),
        ),
        (
            project_dir.join("manifest.json"),
            MANIFEST_JSON_TEMPLATE
                .replace("{name}", name)
                .replace("{name_underscore}", &name_underscore),
        ),
        (
            project_dir.join(".cargo/config.toml"),
            CARGO_CONFIG_TEMPLATE.to_string(),
        ),
        (
            project_dir.join("fixtures/.gitkeep"),
            String::new(),
        ),
    ];

    for (path, content) in &files {
        if let Err(e) = std::fs::write(path, content) {
            eprintln!("specforge: cannot write {}: {e}", path.display());
            return 1;
        }
    }

    eprintln!("specforge: created plugin project in {}", project_dir.display());
    eprintln!();
    eprintln!("  Next steps:");
    eprintln!("    cd {}", project_dir.display());
    eprintln!("    rustup target add wasm32-wasip1");
    eprintln!("    specforge plugin build");
    eprintln!("    specforge plugin test");

    0
}
