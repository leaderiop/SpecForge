use specforge_common::find_project_root;
use std::path::Path;

pub fn run(path: &Path, collector: Option<&str>, format: &str) -> i32 {
    let project_root = match find_project_root(path) {
        Some(root) => root,
        None => {
            let msg = "no specforge project found (missing specforge.json or specforge.spec)";
            if format == "json" {
                let output = serde_json::json!({
                    "error": msg,
                    "exit_code": 1,
                });
                println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
            } else {
                eprintln!("error: {}", msg);
            }
            return 1;
        }
    };

    let collector_name = match collector {
        Some(name) => name.to_string(),
        None => {
            // Auto-detect collector based on project files
            let files: Vec<String> = std::fs::read_dir(&project_root)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();

            let patterns: &[(&str, &str)] = &[
                ("junit", "rust"),
                ("jest", "javascript"),
                ("pytest", "python"),
            ];

            match specforge_wasm::auto_detect_collector(patterns, &files) {
                Ok(name) => name,
                Err(diag) => {
                    if format == "json" {
                        let output = serde_json::json!({
                            "error": diag.message,
                            "code": diag.code,
                            "exit_code": 1,
                        });
                        println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
                    } else {
                        eprintln!("{}: {}", diag.code, diag.message);
                        if let Some(suggestion) = &diag.suggestion {
                            eprintln!("  hint: {}", suggestion);
                        }
                    }
                    return 1;
                }
            }
        }
    };

    if format == "json" {
        let output = serde_json::json!({
            "collector": collector_name,
            "project_root": project_root.display().to_string(),
            "status": "ready",
        });
        println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
    } else {
        println!("collector: {}", collector_name);
        println!("project: {}", project_root.display());
    }

    0
}
