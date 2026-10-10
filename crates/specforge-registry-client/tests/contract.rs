//! Every adapter of the `RegistryClient` seam keeps one contract (ADR 0044): the HTTP client against the
//! real server in process, and the in-memory client.

use specforge_registry_client::HttpRegistryClient;
use specforge_registry_client::registry_config::{RegistryConfig, RegistryCredential};
use specforge_registry_client::testing::{
    MemoryClient, assert_client_contract, assert_private_client_contract,
};
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
    let credential = RegistryCredential::new("local", server.token());
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
    let credential = RegistryCredential::new("memory", "contract-token");
    assert_client_contract(
        &MemoryClient::new().accepting("contract-token"),
        &registry,
        &credential,
    );
}

#[specforge_test_macros::test(
    behavior = "support_private_registries",
    verify = "a private registry refuses an anonymous read and answers an authenticated one"
)]
fn the_private_contract_holds_over_http() {
    let server = LocalRegistry::start_private();
    let registry = RegistryConfig {
        alias: "local".to_string(),
        url: server.url().to_string(),
        scope_filter: None,
        default_registry: true,
    };
    let credential = RegistryCredential::new("local", server.token());
    assert_private_client_contract(&HttpRegistryClient::new(), &registry, &credential);
}

#[specforge_test_macros::test(
    behavior = "support_private_registries",
    verify = "a private registry refuses an anonymous read and answers an authenticated one"
)]
fn the_private_contract_holds_in_memory() {
    let registry = RegistryConfig {
        alias: "memory".to_string(),
        url: "memory://registry/v1".to_string(),
        scope_filter: None,
        default_registry: true,
    };
    let credential = RegistryCredential::new("memory", "contract-token");
    let client = MemoryClient::new()
        .accepting("contract-token")
        .private(&registry);
    assert_private_client_contract(&client, &registry, &credential);
}
