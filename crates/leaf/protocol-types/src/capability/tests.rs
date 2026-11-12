use super::*;
use serde_json::json;

#[test]
fn a_capability_round_trips_and_an_unknown_port_is_refused() {
    let wire = json!([
        {"name": "links", "why": "check links", "call_kinds": ["collect"], "port": "http",
         "schemes": ["https"], "methods": ["HEAD", "GET"], "hosts": {"list": ["*.example.com"]}},
        {"name": "values", "why": "check values", "port": "http", "hosts": "from_values"},
        {"name": "docs", "why": "read docs", "port": "fs", "roots": ["vendor"], "ops": ["stat", "read"]},
        {"name": "tests", "why": "run the tests", "port": "process", "argv": ["cargo", "test"], "deadline_ms": 1000},
        {"name": "log", "why": "read history", "port": "git", "ops": ["read"]}
    ]);
    let parsed: Vec<CapabilityDescriptor> = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(parsed[0].call_kinds, [Operation::Collect]);
    assert!(matches!(&parsed[1].port, PortScope::Http(h) if h.hosts == Hosts::FromValues));
    assert!(matches!(&parsed[3].port, PortScope::Process(p) if p.argv == ["cargo", "test"]));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    let unknown = json!([{"name": "dns", "why": "resolve", "port": "dns"}]);
    assert!(serde_json::from_value::<Vec<CapabilityDescriptor>>(unknown).is_err());
}

#[test]
fn a_secret_header_on_https_hosts_it_lists_naming_a_declared_secret_is_sound() {
    let capability: CapabilityDescriptor = serde_json::from_value(json!({
        "name": "api", "why": "ask the api", "port": "http", "schemes": ["https"],
        "hosts": {"list": ["api.example"]},
        "secret_header": {"header": "Authorization", "secret": "token", "scheme": "Bearer"}
    }))
    .unwrap();
    let mut declaration = ExtensionDeclaration {
        capabilities: vec![capability],
        ..Default::default()
    };
    assert_eq!(
        capability_problems(&declaration).len(),
        1,
        "the secret is not declared yet"
    );
    declaration
        .settings
        .push(SettingsItem::Secret(crate::SecretDescriptor {
            name: "token".to_string(),
            env: "API_TOKEN".to_string(),
            description: None,
        }));
    assert_eq!(capability_problems(&declaration), []);
    assert_eq!(declaration.sound_capabilities().len(), 1);
}

#[test]
fn a_capability_key_its_port_does_not_define_is_unknown() {
    use crate::{DeclaredCategory, DescribeResponse, HandshakeResponse, PROTOCOL_VERSION};
    let items = json!([
        {"name": "api", "why": "ask", "port": "http", "hosts": {"list": ["a.example"]}, "argv": ["x"],
         "secret_header": {"header": "X-Key", "secret": "key", "shceme": "Bearer"}},
        {"name": "tests", "why": "run", "port": "process", "argv": ["t"], "deadline": 5,
         "config": {"name": "t.toml", "template": "x", "tmpl": "y"}},
    ]);
    let handshake = HandshakeResponse {
        protocol_version: PROTOCOL_VERSION.to_string(),
        name: "@acme/caps".to_string(),
        version: "1.0.0".to_string(),
        ..Default::default()
    };
    let mut unknown = Vec::new();
    let declaration = ExtensionDeclaration::from_wire(
        handshake,
        |category| {
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items: if category == DeclaredCategory::Capabilities {
                    items.clone()
                } else {
                    json!([])
                },
            })
        },
        |key| unknown.push(format!("{}:{}", key.item, key.key)),
    )
    .unwrap();
    assert_eq!(declaration.capabilities.len(), 2);
    assert_eq!(
        unknown,
        [
            "api:argv",
            "api.secret_header:shceme",
            "tests:deadline",
            "tests.config:tmpl"
        ]
    );
}

#[test]
fn a_process_capability_carries_its_config_on_the_wire() {
    let wire = json!({
        "name": "tests", "why": "run the tests", "call_kinds": ["collect"], "port": "process",
        "argv": ["cargo-nextest", "nextest", "run", "--tool-config-file", "specforge:{config}"],
        "config": {"name": "nextest.toml", "template": "[profile.specforge.junit]\npath = \"{report}\"\n"}
    });
    let parsed: CapabilityDescriptor = serde_json::from_value(wire.clone()).unwrap();
    let PortScope::Process(process) = &parsed.port else {
        panic!("a process capability: {parsed:?}");
    };
    assert_eq!(
        process.config,
        Some(ProcessConfig {
            name: "nextest.toml".to_string(),
            template: "[profile.specforge.junit]\npath = \"{report}\"\n".to_string(),
        })
    );
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    let declaration = ExtensionDeclaration {
        capabilities: vec![parsed],
        ..Default::default()
    };
    assert_eq!(capability_problems(&declaration), []);
    let bare: CapabilityDescriptor = serde_json::from_value(json!(
        {"name": "tests", "why": "run", "port": "process", "argv": ["cargo", "test"]}
    ))
    .unwrap();
    assert!(
        serde_json::to_value(&bare).unwrap().get("config").is_none(),
        "an absent config is not serialised"
    );
}
