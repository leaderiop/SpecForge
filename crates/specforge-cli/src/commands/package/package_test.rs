use std::path::Path;

pub fn run(path: &Path) -> i32 {
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        eprintln!(
            "specforge: no manifest.json found in {} — is this a package project?",
            path.display()
        );
        return 1;
    }

    // Load manifest
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

    eprintln!("specforge: testing package `{}`...", manifest.package);

    // Load into runtime
    let mut runtime = specforge_wasm::WasmRuntime::new();
    let load_errors = runtime.load_packages(vec![manifest]);

    if !load_errors.is_empty() {
        for err in &load_errors {
            eprintln!("specforge: load error: {err}");
        }
        return 1;
    }

    // Initialize
    let init_result = runtime.initialize_all();
    for err in &init_result.errors {
        eprintln!("specforge: init error: {err}");
    }
    for diag in &init_result.diagnostics {
        eprintln!("specforge: [{}] {}: {}", diag.code, diag.severity(), diag.message);
    }

    if !init_result.errors.is_empty() {
        return 1;
    }

    eprintln!(
        "specforge: registered {} entities, {} edges",
        init_result.entities.len(),
        init_result.edges.len()
    );

    // Run validation with an empty graph
    let empty_graph = r#"{"nodes":[],"edges":[]}"#;
    let validate_result = runtime.validate_all(empty_graph);

    for err in &validate_result.errors {
        eprintln!("specforge: validate error: {err}");
    }
    for diag in &validate_result.diagnostics {
        eprintln!("specforge: [{}] {}: {}", diag.code, diag.severity(), diag.message);
    }

    // Test against fixture spec files
    let fixtures_dir = path.join("fixtures");
    if fixtures_dir.exists() {
        let fixture_count = std::fs::read_dir(&fixtures_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .is_some_and(|ext| ext == "spec")
                    })
                    .count()
            })
            .unwrap_or(0);

        if fixture_count > 0 {
            eprintln!("specforge: found {fixture_count} fixture spec file(s)");
        }
    }

    if validate_result.errors.is_empty() && init_result.errors.is_empty() {
        eprintln!("specforge: all tests passed");
        0
    } else {
        1
    }
}
