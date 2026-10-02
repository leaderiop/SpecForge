use crate::OutputFormat;
use serde_json::json;
use specforge_registry_client::{
    AuthMethod, HttpRegistryClient, RegistryCredential,
    credentials::{credentials_path, read_credentials, write_credentials},
    validate_credentials,
};
use std::path::Path;

pub fn run(
    registry_alias: Option<&str>,
    token: Option<&str>,
    path: &Path,
    format: OutputFormat,
) -> i32 {
    let alias = registry_alias.unwrap_or("default");

    let token_value = match token {
        Some(t) => t.to_string(),
        None => {
            format.print_error("no token provided. Use --token <TOKEN>", "R-LOGIN-001");
            return 1;
        }
    };

    // The token is validated against a configured registry; with none,
    // fail before any network call (ADR 0004 N1).
    let registries = match specforge_ops::registry::configured(path, "login") {
        Ok(configured) => {
            format.eprint_diagnostics(&configured.diagnostics);
            configured.registries
        }
        Err(error) => {
            format.print_op_error(&error);
            return 1;
        }
    };
    let Some(registry) = registries
        .iter()
        .find(|r| r.alias == alias)
        .or_else(|| registries.iter().find(|r| r.default_registry))
        .cloned()
    else {
        let error = specforge_ops::OpError::new(
            specforge_ops::registry::NO_REGISTRY,
            format!("no registry '{alias}' configured, and none is the default"),
        )
        .with_suggestion(
            "pass --registry <alias> naming an entry of specforge.json's \"registries\", \
             or set \"default_registry\": true on one",
        );
        format.print_op_error(&error);
        return 1;
    };

    // Validate token
    let credential = RegistryCredential {
        alias: alias.to_string(),
        auth_method: AuthMethod::Bearer(token_value.clone()),
    };

    let client = HttpRegistryClient::new();
    let expires_at = match validate_credentials(&client, &registry, &credential) {
        Ok(expires_at) => expires_at,
        Err(diag) => {
            format.print_error(&diag.message, &diag.code);
            return 1;
        }
    };

    // Store credentials: secret into the OS keyring (0600-file fallback),
    // file keeps only metadata. Any legacy plaintext entry for this alias
    // is migrated by this write.
    let cred_path = credentials_path();
    let mut store = read_credentials(&cred_path).unwrap_or_default();
    if let Err(message) = store.set_token(alias, token_value, expires_at) {
        format.print_error(&message, "R-LOGIN-002");
        return 1;
    }

    if let Err(diag) = write_credentials(&cred_path, &store) {
        format.print_error(&diag.message, &diag.code);
        return 1;
    }

    match format {
        OutputFormat::Json => {
            let output = json!({
                "action": "login",
                "registry": alias,
                "status": "authenticated",
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            println!("logged in to registry '{}'", alias);
        }
    }

    0
}

pub fn run_logout(registry_alias: Option<&str>, format: OutputFormat) -> i32 {
    let alias = registry_alias.unwrap_or("default");
    let cred_path = credentials_path();

    let mut store = read_credentials(&cred_path).unwrap_or_default();
    let removed = store.remove(alias);
    specforge_registry_client::secrets::delete_secret(alias);

    if !removed {
        match format {
            OutputFormat::Json => {
                let output = json!({
                    "action": "logout",
                    "registry": alias,
                    "status": "not_found",
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            OutputFormat::Human => println!("no credentials found for registry '{}'", alias),
        }
        return 0;
    }

    if let Err(diag) = write_credentials(&cred_path, &store) {
        format.print_error(&diag.message, &diag.code);
        return 1;
    }

    match format {
        OutputFormat::Json => {
            let output = json!({
                "action": "logout",
                "registry": alias,
                "status": "removed",
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => println!("logged out from registry '{}'", alias),
    }

    0
}
