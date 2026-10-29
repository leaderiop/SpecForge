#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use crate::support;

use specforge_hosts::renderer;
use specforge_ops::host::{Host, ModelChoice};

fn agent_text(model: ModelChoice, body: &str) -> String {
    let mut rendered = support::fixture(Host::Codex);
    rendered.agents.truncate(1);
    rendered.agents[0].model = model;
    rendered.agents[0].body = body.to_string();
    let tree = renderer(Host::Codex).unwrap().render(&rendered);
    tree.owned
        .into_iter()
        .find(|f| f.path.ends_with("t-planner.toml"))
        .unwrap()
        .text
}

#[test]
fn a_codex_agent_never_gets_an_anthropic_model() {
    for id in ["claude-opus-4", "sonnet", "Haiku-4.5"] {
        let text = agent_text(ModelChoice::Id(id.into()), "b");
        assert!(!text.contains("model ="), "{text}");
        assert!(!text.contains("model_reasoning_effort"), "{text}");
    }
    let tier = agent_text(ModelChoice::Tier("heavy"), "b");
    assert!(!tier.contains("model ="));
    let own = agent_text(ModelChoice::Id("gpt-5-codex".into()), "b");
    assert!(
        own.contains("model = \"gpt-5-codex\"\nmodel_reasoning_effort = \"high\""),
        "{own}"
    );
}

#[test]
fn developer_instructions_holding_triple_quotes_are_escaped() {
    let text = agent_text(ModelChoice::Inherit, "say '''hi'''\nline two");
    let parsed: toml_edit::DocumentMut = text.parse().unwrap();
    assert_eq!(
        parsed["developer_instructions"].as_str().unwrap(),
        "say '''hi'''\nline two"
    );
    let plain = agent_text(ModelChoice::Inherit, "say hi");
    assert!(plain.contains("developer_instructions = '''\nsay hi\n'''"));
    assert_eq!(
        plain.parse::<toml_edit::DocumentMut>().unwrap()["developer_instructions"]
            .as_str()
            .unwrap(),
        "say hi\n"
    );
}
