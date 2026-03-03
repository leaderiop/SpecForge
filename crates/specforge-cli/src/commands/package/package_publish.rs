use std::path::{Path, PathBuf};

pub fn run(path: &Path, out: Option<PathBuf>) -> i32 {
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        eprintln!(
            "specforge: no manifest.json found in {} — is this a package project?",
            path.display()
        );
        return 1;
    }

    let manifest = match specforge_wasm::load_manifest(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("specforge: {e}");
            return 1;
        }
    };

    // Check wasm binary exists
    if !manifest.wasm_path.exists() {
        eprintln!(
            "specforge: wasm binary not found at {} — run `specforge package build` first",
            manifest.wasm_path.display()
        );
        return 1;
    }

    let out_dir = out.unwrap_or_else(|| path.join("dist"));
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("specforge: cannot create output directory: {e}");
        return 1;
    }

    // Compute SHA-256 of the wasm binary
    let wasm_bytes = match std::fs::read(&manifest.wasm_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("specforge: cannot read wasm binary: {e}");
            return 1;
        }
    };
    let wasm_hash = specforge_wasm::loader::compute_sha256(&wasm_bytes);

    // Copy wasm binary to output
    let wasm_filename = manifest
        .wasm_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("plugin.wasm");
    let dest_wasm = out_dir.join(wasm_filename);
    if let Err(e) = std::fs::copy(&manifest.wasm_path, &dest_wasm) {
        eprintln!("specforge: cannot copy wasm binary: {e}");
        return 1;
    }

    // Write updated manifest with wasm path pointing to the filename only
    let publish_manifest = serde_json::json!({
        "package": manifest.package,
        "manifest_version": manifest.manifest_version,
        "kind": manifest.kind,
        "contributes": manifest.contributes,
        "wasm": wasm_filename,
        "version": manifest.version,
        "description": manifest.description,
        "sandbox": manifest.sandbox,
        "peer_dependencies": manifest.peer_dependencies,
        "enhancements": manifest.enhancements,
        "dynamic_edge_types": manifest.dynamic_edge_types,
        "entity_kinds": manifest.entity_kinds,
        "wasm_sha256": wasm_hash,
    });

    let manifest_json = serde_json::to_string_pretty(&publish_manifest).unwrap_or_default();
    let dest_manifest = out_dir.join("manifest.json");
    if let Err(e) = std::fs::write(&dest_manifest, &manifest_json) {
        eprintln!("specforge: cannot write manifest: {e}");
        return 1;
    }

    let wasm_size_kb = wasm_bytes.len() / 1024;
    eprintln!("specforge: packaged `{}`", manifest.package);
    eprintln!("  Output:    {}", out_dir.display());
    eprintln!("  Wasm:      {wasm_filename} ({wasm_size_kb} KB)");
    eprintln!("  SHA-256:   {wasm_hash}");
    eprintln!("  Version:   {}", manifest.version);

    0
}
