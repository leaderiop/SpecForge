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

/// A collector's process capability running `argv`, appending `filter` when a caller passes one.
fn filtered(argv: &[&str], filter: &[&str]) -> ExtensionDeclaration {
    let capability: CapabilityDescriptor = serde_json::from_value(json!({
        "name": "tests", "why": "run the tests", "call_kinds": ["collect"], "port": "process",
        "argv": argv, "filter": filter,
    }))
    .unwrap();
    ExtensionDeclaration {
        capabilities: vec![capability],
        ..Default::default()
    }
}

/// The problems of `declaration`'s capabilities, their reasons only.
fn whys(declaration: &ExtensionDeclaration) -> Vec<String> {
    capability_problems(declaration)
        .into_iter()
        .map(|p| p.why)
        .collect()
}

#[test]
fn a_filter_holds_the_filter_placeholder_in_exactly_one_element() {
    let sound = filtered(&["cargo-nextest", "nextest", "run"], &["-E", "{filter}"]);
    assert_eq!(whys(&sound), Vec::<String>::new());
    let PortScope::Process(process) = &sound.capabilities[0].port else {
        panic!("a process capability");
    };
    assert_eq!(process.filter, ["-E", "{filter}"]);
    let wire = serde_json::to_value(&sound.capabilities[0]).unwrap();
    assert_eq!(wire["filter"], json!(["-E", "{filter}"]));
    let bare = filtered(&["cargo", "test"], &[]);
    assert!(
        serde_json::to_value(&bare.capabilities[0])
            .unwrap()
            .get("filter")
            .is_none(),
        "an empty filter is not serialised"
    );
    for wrong in [
        &["-E"][..],
        &["{filter}", "{filter}"],
        &["--expr={filter}{filter}"],
    ] {
        let problems = whys(&filtered(&["t"], wrong));
        assert!(
            problems
                .iter()
                .any(|p| p.contains("one argument holds it once")),
            "{wrong:?}: {problems:?}"
        );
    }
    let mut command = filtered(&["t"], &["{filter}"]);
    command.capabilities[0].call_kinds = vec![Operation::Command];
    assert!(
        whys(&command)
            .iter()
            .any(|p| p.contains("call_kinds must be exactly collect"))
    );
}

#[test]
fn the_filter_placeholder_in_argv_or_another_placeholder_in_the_filter_is_a_problem() {
    let in_argv = whys(&filtered(&["t", "{filter}"], &[]));
    assert!(
        in_argv.iter().any(|p| p.contains("only its `filter`")),
        "{in_argv:?}"
    );
    for host in ["--out={report}", "{config}"] {
        let problems = whys(&filtered(&["t"], &["{filter}", host]));
        assert!(
            problems
                .iter()
                .any(|p| p.contains("names a placeholder only the host fills")),
            "{host}: {problems:?}"
        );
    }
}

#[test]
fn a_filter_text_empty_long_dashed_or_with_a_control_character_is_refused() {
    assert_eq!(filter_problem("binary_id(=toy::api)"), None);
    assert_eq!(filter_problem("test(=a) | test(=b)"), None);
    assert_eq!(filter_problem("élève"), None);
    assert_eq!(filter_problem(&"t".repeat(FILTER_MAX)), None);
    for (text, why) in [
        ("", "empty"),
        ("  ", "empty"),
        (&"t".repeat(FILTER_MAX + 1)[..], "over"),
        ("-x", "starts with '-'"),
        ("--workspace", "starts with '-'"),
        ("test(=a)\n", "control character"),
        ("a\u{0}b", "control character"),
        ("a\u{1b}[31m", "control character"),
    ] {
        let problem = filter_problem(text).unwrap_or_default();
        assert!(problem.contains(why), "{text:?}: {problem}");
    }
}

#[test]
fn a_star_not_last_or_a_prefix_that_is_not_a_namespace_is_a_problem() {
    let declared = |name: &str| {
        let capability: CapabilityDescriptor = serde_json::from_value(json!({
            "name": "tests", "why": "run the tests", "port": "process", "argv": ["t"], "env": [name],
        }))
        .unwrap();
        whys(&ExtensionDeclaration {
            capabilities: vec![capability],
            ..Default::default()
        })
    };
    for refused in [
        "*", "A*", "C*", "CARG*", "RUST*", "_ABC_*", "CARGO_*X", "CAR-GO_*", "CARGO_**", "1ABC_*",
    ] {
        let problems = declared(refused);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("not a namespace prefix")),
            "{refused}: {problems:?}"
        );
        assert!(!is_env_prefix(refused), "{refused}");
    }
    for admitted in [
        "CARGO_*",
        "RUSTC_*",
        "RUSTUP_*",
        "ABC_*",
        "RUSTFLAGS",
        "CARGO_HOME",
    ] {
        assert_eq!(declared(admitted), Vec::<String>::new(), "{admitted}");
    }
}
