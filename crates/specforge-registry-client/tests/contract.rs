//! Every adapter of the `RegistryClient` seam keeps one contract (ADR 0044): the HTTP client against the
//! real server in process, and the in-memory client.

use specforge_registry_client::HttpRegistryClient;
use specforge_registry_client::registry_config::{AuthMethod, RegistryConfig, RegistryCredential};
use specforge_registry_client::testing::{MemoryClient, assert_client_contract};
use specforge_registry_server::testing::LocalRegistry;

#[specforge_test_macros::test(
    port = "RegistryClient",
    verify = "RegistryClient contract is satisfied"
)]
fn the_http_client_keeps_the_client_contract() {
    let server = LocalRegistry::start();
    let registry = RegistryConfig {
        alias: "local".to_string(),
        url: server.url().to_string(),
        scope_filter: None,
        default_registry: true,
    };
    let credential = RegistryCredential {
        alias: "local".to_string(),
        auth_method: AuthMethod::Bearer(server.token().to_string()),
    };
    assert_client_contract(&HttpRegistryClient::new(), &registry, &credential);
}

#[specforge_test_macros::test(
    port = "RegistryClient",
    verify = "RegistryClient contract is satisfied"
)]
fn the_memory_client_keeps_the_client_contract() {
    let registry = RegistryConfig {
        alias: "memory".to_string(),
        url: "memory://registry/v1".to_string(),
        scope_filter: None,
        default_registry: true,
    };
    let credential = RegistryCredential {
        alias: "memory".to_string(),
        auth_method: AuthMethod::Bearer("contract-token".to_string()),
    };
    assert_client_contract(
        &MemoryClient::new().accepting("contract-token"),
        &registry,
        &credential,
    );
}
