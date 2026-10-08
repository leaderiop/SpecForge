//! `specforge login` and `specforge logout`: a registry credential, kept
//! under the alias of the registry it is for (ADR 0045).

use crate::OutputFormat;
use crate::outcome::{Exit, Refusal};
use serde_json::json;
use specforge_common::codes;
use specforge_ops::OpError;
use specforge_registry_client::{
    AuthMethod, HttpRegistryClient, RegistryCredential,
    credentials::{credentials_path, read_credentials, write_credentials},
    validate_credentials,
};
use std::path::Path;

/// Validate `token` against the registry `registry_alias` names (the
/// default registry when none is given) and keep it under that registry's
/// alias.
pub(crate) fn run(
    registry_alias: Option<&str>,
    token: Option<&str>,
    path: &Path,
    format: OutputFormat,
) -> Exit {
    let refuse = |error: &OpError| Refusal::of(format).report(error);
    let token_value = match token {
        Some(t) => t.to_string(),
        None => {
            return refuse(&OpError::diagnostic(
                codes::R_LOGIN_001,
                "no token provided. Use --token <TOKEN>",
            ));
        }
    };

    // The token is validated against a configured registry; with none,
    // fail before any network call (ADR 0004 N1).
    let configured = match specforge_ops_registry::configured(path, "login") {
        Ok(configured) => configured,
        Err(error) => return refuse(&error),
    };
    format.eprint_diagnostics(&configured.diagnostics);
    let registry = match configured.named(registry_alias) {
        Ok(registry) => registry.clone(),
        Err(error) => return refuse(&error),
    };
    let alias = registry.alias.as_str();

    // Validate token
    let credential = RegistryCredential {
        alias: alias.to_string(),
        auth_method: AuthMethod::Bearer(token_value.clone()),
    };

    let client = HttpRegistryClient::new();
    let expires_at = match validate_credentials(&client, &registry, &credential) {
        Ok(expires_at) => expires_at,
        Err(diag) => return refuse(&OpError::from(diag)),
    };

    // Store credentials: secret into the OS keyring (0600-file fallback),
    // file keeps only metadata. Any legacy plaintext entry for this alias
    // is migrated by this write.
    let cred_path = credentials_path();
    let mut store = read_credentials(&cred_path).unwrap_or_default();
    if let Err(message) = store.set_token(alias, token_value, expires_at) {
        return refuse(&OpError::diagnostic(codes::R_LOGIN_002, message));
    }

    if let Err(diag) = write_credentials(&cred_path, &store) {
        return refuse(&OpError::from(diag));
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
            println!("logged in to registry '{alias}'");
        }
    }

    Exit::Passed
}

/// Forget the credential of the registry `registry_alias` names, without
/// reading any project; with none given, of the default registry of the
/// project at `path`.
pub(crate) fn run_logout(registry_alias: Option<&str>, path: &Path, format: OutputFormat) -> Exit {
    let refuse = |error: &OpError| Refusal::of(format).report(error);
    let alias = match registry_alias {
        Some(alias) => alias.to_string(),
        None => match specforge_ops_registry::configured(path, "logout") {
            Ok(configured) => {
                format.eprint_diagnostics(&configured.diagnostics);
                match configured.named(None) {
                    Ok(registry) => registry.alias.clone(),
                    Err(error) => return refuse(&error),
                }
            }
            Err(error) => return refuse(&error),
        },
    };
    let alias = alias.as_str();
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
            OutputFormat::Human => println!("no credentials found for registry '{alias}'"),
        }
        return Exit::Passed;
    }

    if let Err(diag) = write_credentials(&cred_path, &store) {
        return refuse(&OpError::from(diag));
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
        OutputFormat::Human => println!("logged out from registry '{alias}'"),
    }

    Exit::Passed
}
