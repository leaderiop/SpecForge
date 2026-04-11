use serde_json::json;
use specforge_wasm::parse_extension_specifier;
use std::path::Path;

pub fn run(specifier: &str, path: &Path, format: &str) -> i32 {
    // 1. Parse the specifier to validate it
    let parsed = match parse_extension_specifier(specifier) {
        Ok(p) => p,
        Err(diag) => {
            match format {
                "json" => {
                    let output = json!({
                        "error": diag.message,
                        "code": diag.code,
                    });
                    println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
                }
                _ => {
                    eprintln!("error: {}", diag.message);
                    if let Some(suggestion) = &diag.suggestion {
                        eprintln!("  hint: {}", suggestion);
                    }
                }
            }
            return 1;
        }
    };

    // 2. Read specforge.json if it exists
    let config_path = path.join("specforge.json");
    let _config: Option<serde_json::Value> = if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(content) => serde_json::from_str(&content).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    // 3. Report what would be installed (dry-run — no network client yet)
    let (kind, name, version) = match &parsed {
        specforge_wasm::ExtensionSpecifier::Registry { name, version } => {
            ("registry", name.as_str(), version.as_str())
        }
        specforge_wasm::ExtensionSpecifier::Local { path } => {
            ("local", path.to_str().unwrap_or("<path>"), "local")
        }
        specforge_wasm::ExtensionSpecifier::Git { url, rev } => {
            ("git", url.as_str(), rev.as_deref().unwrap_or("HEAD"))
        }
    };

    match format {
        "json" => {
            let output = json!({
                "action": "add",
                "specifier": specifier,
                "source": kind,
                "name": name,
                "version": version,
                "dry_run": true,
                "message": "extension specifier validated (download not yet implemented)",
            });
            println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
        }
        _ => {
            println!("Validated extension specifier: {}", specifier);
            println!("  source: {}", kind);
            println!("  name: {}", name);
            println!("  version: {}", version);
            println!();
            println!("note: actual download is not yet implemented (dry-run)");
        }
    }

    0
}
