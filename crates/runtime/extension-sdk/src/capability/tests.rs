use crate::prelude::*;

#[test]
fn a_collector_run_declares_its_process_capability() {
    let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/runner", "0.1.0"));
    b.collector("acme", |k| {
        k.run(&["acme", "--out={report}"])
            .collect(|_| Ok(CollectOutput::default()));
    });
    b.collector("reader", |k| {
        k.collect(|_| Ok(CollectOutput::default()));
    });
    let declaration = b.declaration();

    assert_eq!(
        declaration.collectors[0].capability.as_deref(),
        Some("acme")
    );
    assert_eq!(declaration.collectors[1].capability, None);
    assert_eq!(
        serde_json::to_value(&declaration.capabilities).unwrap(),
        serde_json::json!([{
            "name": "acme",
            "why": "run the command whose report the collector 'acme' reads",
            "call_kinds": ["collect"],
            "port": "process",
            "argv": ["acme", "--out={report}"],
        }])
    );
    assert!(specforge_protocol_types::capability_problems(&declaration).is_empty());
}

#[test]
fn a_collector_config_and_install_ride_its_declaration() {
    let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/runner", "0.1.0"));
    b.collector("acme", |k| {
        k.run(&["acme", "--config", "{config}"])
            .config("acme.toml", "out = \"{report}\"\n")
            .install("cargo install acme --locked")
            .collect(|_| Ok(CollectOutput::default()));
    });
    let declaration = b.declaration();

    assert_eq!(
        declaration.collectors[0].install.as_deref(),
        Some("cargo install acme --locked")
    );
    assert_eq!(
        serde_json::to_value(&declaration.capabilities).unwrap()[0]["config"],
        serde_json::json!({"name": "acme.toml", "template": "out = \"{report}\"\n"})
    );
    assert!(specforge_protocol_types::capability_problems(&declaration).is_empty());
}

#[test]
#[should_panic(expected = "declares a config but no command")]
fn a_collector_config_without_a_command_panics() {
    let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/runner", "0.1.0"));
    b.collector("acme", |k| {
        k.config("acme.toml", "x")
            .collect(|_| Ok(CollectOutput::default()));
    });
}
