use std::path::Path;

pub fn run_init(path: &Path, name: Option<&str>, format: &str) -> i32 {
    let ext_name = name.unwrap_or("my-extension");
    let ext_dir = path.join(ext_name);

    if ext_dir.exists() {
        let msg = format!("directory '{}' already exists", ext_dir.display());
        if format == "json" {
            println!("{}", serde_json::json!({"error": msg, "exit_code": 1}));
        } else {
            eprintln!("error: {}", msg);
        }
        return 1;
    }

    if let Err(e) = std::fs::create_dir_all(&ext_dir) {
        let msg = format!("cannot create directory: {}", e);
        if format == "json" {
            println!("{}", serde_json::json!({"error": msg, "exit_code": 1}));
        } else {
            eprintln!("error: {}", msg);
        }
        return 1;
    }

    // Write manifest.json
    let manifest = serde_json::json!({
        "name": format!("@local/{}", ext_name),
        "version": "0.1.0",
        "manifestVersion": 2,
        "wasmPath": format!("target/wasm32-wasip1/release/{}.wasm", ext_name.replace('-', "_")),
        "contributes": { "entities": true },
        "entityKinds": [],
        "edgeTypes": [],
        "fields": []
    });
    if let Err(e) = std::fs::write(
        ext_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialize JSON output"),
    ) {
        let msg = format!("cannot write manifest.json: {}", e);
        if format == "json" {
            println!("{}", serde_json::json!({"error": msg, "exit_code": 1}));
        } else {
            eprintln!("error: {}", msg);
        }
        return 1;
    }

    // Write src/lib.rs skeleton
    if let Err(e) = std::fs::create_dir_all(ext_dir.join("src")) {
        let msg = format!("cannot create src directory: {}", e);
        if format == "json" {
            println!("{}", serde_json::json!({"error": msg, "exit_code": 1}));
        } else {
            eprintln!("error: {}", msg);
        }
        return 1;
    }
    if let Err(e) = std::fs::write(
        ext_dir.join("src/lib.rs"),
        format!(
            r#"//! {} -- a SpecForge extension
//!
//! Build with: cargo build --target wasm32-wasip1 --release

#[no_mangle]
pub extern "C" fn _start() {{}}
"#,
            ext_name
        ),
    ) {
        let msg = format!("cannot write src/lib.rs: {}", e);
        if format == "json" {
            println!("{}", serde_json::json!({"error": msg, "exit_code": 1}));
        } else {
            eprintln!("error: {}", msg);
        }
        return 1;
    }

    // Write Cargo.toml
    if let Err(e) = std::fs::write(
        ext_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]
"#,
            ext_name
        ),
    ) {
        let msg = format!("cannot write Cargo.toml: {}", e);
        if format == "json" {
            println!("{}", serde_json::json!({"error": msg, "exit_code": 1}));
        } else {
            eprintln!("error: {}", msg);
        }
        return 1;
    }

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "path": ext_dir.display().to_string(),
                "name": ext_name,
                "files": ["manifest.json", "src/lib.rs", "Cargo.toml"],
            }))
            .expect("serialize JSON output")
        );
    } else {
        println!("Created extension scaffold at {}", ext_dir.display());
        println!("  manifest.json");
        println!("  src/lib.rs");
        println!("  Cargo.toml");
    }

    0
}

pub fn run_build(path: &Path, format: &str) -> i32 {
    // Check for Cargo.toml
    if !path.join("Cargo.toml").exists() {
        let msg = format!("no Cargo.toml found at {}", path.display());
        if format == "json" {
            println!(
                "{}",
                serde_json::json!({"error": msg, "code": "E040", "exit_code": 1})
            );
        } else {
            eprintln!("E040: {}", msg);
        }
        return 1;
    }

    // Check for manifest.json
    if !path.join("manifest.json").exists() {
        let msg = format!("no manifest.json found at {}", path.display());
        if format == "json" {
            println!(
                "{}",
                serde_json::json!({"error": msg, "code": "E040", "exit_code": 1})
            );
        } else {
            eprintln!("E040: {}", msg);
        }
        return 1;
    }

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "status": "validated",
                "path": path.display().to_string(),
            }))
            .expect("serialize JSON output")
        );
    } else {
        println!(
            "Extension project structure validated at {}",
            path.display()
        );
    }

    0
}

pub fn run_validate(path: &Path, format: &str) -> i32 {
    // Check manifest exists
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        let msg = format!("no manifest.json found at {}", path.display());
        if format == "json" {
            println!(
                "{}",
                serde_json::json!({"error": msg, "code": "E040", "exit_code": 1})
            );
        } else {
            eprintln!("E040: {}", msg);
        }
        return 1;
    }

    // Load and validate manifest
    let content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("cannot read manifest.json: {}", e);
            if format == "json" {
                println!("{}", serde_json::json!({"error": msg, "exit_code": 1}));
            } else {
                eprintln!("error: {}", msg);
            }
            return 1;
        }
    };

    let manifest: specforge_registry::ManifestV2 = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            let msg = format!("invalid manifest.json: {}", e);
            if format == "json" {
                println!(
                    "{}",
                    serde_json::json!({"error": msg, "code": "E030", "exit_code": 1})
                );
            } else {
                eprintln!("E030: {}", msg);
            }
            return 1;
        }
    };

    let diags = specforge_registry::validate_manifest(&manifest);
    if !diags.is_empty() {
        if format == "json" {
            let errs: Vec<_> = diags
                .iter()
                .map(|d| {
                    serde_json::json!({
                        "code": d.code,
                        "message": d.message,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "valid": false,
                    "diagnostics": errs,
                    "exit_code": 1,
                }))
                .expect("serialize JSON output")
            );
        } else {
            for d in &diags {
                eprintln!("{}: {}", d.code, d.message);
            }
        }
        return 1;
    }

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "valid": true,
                "name": manifest.name,
                "version": manifest.version,
            }))
            .expect("serialize JSON output")
        );
    } else {
        println!(
            "manifest.json is valid: {} v{}",
            manifest.name, manifest.version
        );
    }

    0
}
