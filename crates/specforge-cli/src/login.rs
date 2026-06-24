use serde_json::json;
use specforge_registry::{
    HttpRegistryClient, RegistryConfig,
    client::credentials::{credentials_path, read_credentials, write_credentials},
    parse_registries_from_config, validate_credentials,
    RegistryCredential, AuthMethod,
};
use std::path::Path;

pub fn run(registry_alias: Option<&str>, token: Option<&str>, path: &Path, format: &str) -> i32 {
    let alias = registry_alias.unwrap_or("default");

    let token_value = match token {
        Some(t) => t.to_string(),
        None => {
            print_error(format, "no token provided. Use --token <TOKEN>", "R-LOGIN-001");
            return 1;
        }
    };

    // Load registries to validate the token against the right endpoint
    let config_path = path.join("specforge.json");
    let registries = load_registries(&config_path);
    let registry = registries
        .iter()
        .find(|r| r.alias == alias)
        .or_else(|| registries.iter().find(|r| r.default_registry))
        .cloned()
        .unwrap_or_else(default_registry);

    // Validate token
    let credential = RegistryCredential {
        alias: alias.to_string(),
        auth_method: AuthMethod::Bearer(token_value.clone()),
    };

    let client = HttpRegistryClient::new();
    if let Err(diag) = validate_credentials(&client, &registry, &credential) {
        print_error(format, &diag.message, &diag.code);
        return 1;
    }

    // Store credentials
    let cred_path = credentials_path();
    let mut store = read_credentials(&cred_path).unwrap_or_default();
    store.set_token(alias, token_value);

    if let Err(diag) = write_credentials(&cred_path, &store) {
        print_error(format, &diag.message, &diag.code);
        return 1;
    }

    match format {
        "json" => {
            let output = json!({
                "action": "login",
                "registry": alias,
                "status": "authenticated",
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        _ => {
            println!("logged in to registry '{}'", alias);
        }
    }

    0
}

pub fn run_logout(registry_alias: Option<&str>, format: &str) -> i32 {
    let alias = registry_alias.unwrap_or("default");
    let cred_path = credentials_path();

    let mut store = read_credentials(&cred_path).unwrap_or_default();
    let removed = store.remove(alias);

    if !removed {
        match format {
            "json" => {
                let output = json!({
                    "action": "logout",
                    "registry": alias,
                    "status": "not_found",
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            _ => println!("no credentials found for registry '{}'", alias),
        }
        return 0;
    }

    if let Err(diag) = write_credentials(&cred_path, &store) {
        print_error(format, &diag.message, &diag.code);
        return 1;
    }

    match format {
        "json" => {
            let output = json!({
                "action": "logout",
                "registry": alias,
                "status": "removed",
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        _ => println!("logged out from registry '{}'", alias),
    }

    0
}

fn load_registries(config_path: &Path) -> Vec<RegistryConfig> {
    if !config_path.exists() {
        return vec![default_registry()];
    }

    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => return vec![default_registry()],
    };

    let (registries, _) = parse_registries_from_config(&content);
    if registries.is_empty() {
        vec![default_registry()]
    } else {
        registries
    }
}

fn default_registry() -> RegistryConfig {
    RegistryConfig {
        alias: "default".to_string(),
        url: "https://registry.specforge.dev/v1".to_string(),
        scope_filter: None,
        default_registry: true,
    }
}

fn print_error(format: &str, message: &str, code: &str) {
    match format {
        "json" => {
            let output = json!({"error": message, "code": code});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        _ => eprintln!("error[{}]: {}", code, message),
    }
}
