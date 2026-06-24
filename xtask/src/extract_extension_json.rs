use specforge_emitter::builtins::runtime_for_extensions;
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};

fn main() {
    let extensions = [
        "@specforge/product",
        "@specforge/software",
        "@specforge/governance",
        "@specforge/formal",
    ];
    let ext_names: Vec<String> = extensions.iter().map(|s| s.to_string()).collect();
    let runtime = runtime_for_extensions(&ext_names);
    let categories = [
        "entities",
        "edges",
        "fields",
        "shared_fields",
        "enhancements",
        "validation_rules",
        "surfaces",
        "passes",
        "feature_flags",
    ];

    for ext_name in &extensions {
        let dir_name = ext_name.strip_prefix("@specforge/").unwrap();
        let out_dir = format!("extensions/{}/src", dir_name);
        std::fs::create_dir_all(&out_dir).unwrap();

        // Handshake
        if let WasmCallResult::Ok(b) =
            runtime.call_export(ext_name, "__handshake", &[])
        {
            let pretty: serde_json::Value = serde_json::from_slice(&b).unwrap();
            let json = serde_json::to_string_pretty(&pretty).unwrap();
            std::fs::write(format!("{}/handshake.json", out_dir), json).unwrap();
        }

        // Describe
        for cat in &categories {
            let input = format!(r#"{{"category":"{}"}}"#, cat);
            if let WasmCallResult::Ok(b) =
                runtime.call_export(ext_name, "__describe", input.as_bytes())
            {
                let pretty: serde_json::Value = serde_json::from_slice(&b).unwrap();
                let json = serde_json::to_string_pretty(&pretty).unwrap();
                std::fs::write(format!("{}/describe_{}.json", out_dir, cat), json).unwrap();
            }
        }

        eprintln!("Extracted: {}", ext_name);
    }
}
