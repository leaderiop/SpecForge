use std::path::Path;
use std::process::Command;

pub fn run(path: &Path) -> i32 {
    // Verify Cargo.toml exists
    let cargo_toml = path.join("Cargo.toml");
    if !cargo_toml.exists() {
        eprintln!(
            "specforge: no Cargo.toml found in {} — is this a plugin project?",
            path.display()
        );
        return 1;
    }

    // Verify manifest.json exists
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        eprintln!(
            "specforge: no manifest.json found in {} — is this a plugin project?",
            path.display()
        );
        return 1;
    }

    // Check wasm32-wasip1 target
    eprintln!("specforge: building plugin...");

    let status = Command::new("cargo")
        .args(["build", "--target", "wasm32-wasip1", "--release"])
        .current_dir(path)
        .status();

    match status {
        Ok(s) if s.success() => {
            eprintln!("specforge: build successful");

            // Read manifest to find expected wasm path
            if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                if let Ok(manifest) =
                    serde_json::from_str::<specforge_wasm::PackageManifest>(&content)
                {
                    let wasm_path = path.join(&manifest.wasm);
                    if wasm_path.exists() {
                        if let Ok(meta) = std::fs::metadata(&wasm_path) {
                            let size_kb = meta.len() / 1024;
                            eprintln!(
                                "specforge: wasm binary: {} ({size_kb} KB)",
                                wasm_path.display()
                            );
                        }
                    } else {
                        eprintln!(
                            "specforge: warning: expected wasm at {} but not found",
                            wasm_path.display()
                        );
                    }
                }
            }

            0
        }
        Ok(s) => {
            eprintln!("specforge: build failed with exit code {}", s.code().unwrap_or(-1));
            eprintln!("hint: make sure wasm32-wasip1 target is installed: rustup target add wasm32-wasip1");
            1
        }
        Err(e) => {
            eprintln!("specforge: cannot run cargo: {e}");
            eprintln!("hint: make sure cargo is installed and on PATH");
            1
        }
    }
}
