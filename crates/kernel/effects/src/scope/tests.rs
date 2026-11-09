use specforge_protocol_types::{
    EffectRequest, PortScope, ProcessConfig, ProcessRequest, ProcessScope,
};

use super::rules;

/// The process scope of `argv`, with a config of `template` when one is given.
fn process(argv: &[&str], template: Option<&str>) -> PortScope {
    PortScope::Process(ProcessScope {
        argv: argv.iter().map(|a| a.to_string()).collect(),
        config: template.map(|template| ProcessConfig {
            name: "nextest.toml".to_string(),
            template: template.to_string(),
        }),
        ..ProcessScope::default()
    })
}

/// A request to run `argv`.
fn run(argv: &[&str]) -> EffectRequest {
    EffectRequest::Process(ProcessRequest {
        argv: argv.iter().map(|a| a.to_string()).collect(),
        env: Vec::new(),
        stdin: None,
        deadline_ms: 1000,
        output_cap: 1024,
        text: false,
        inherit: None,
    })
}

const NEXTEST: [&str; 4] = [
    "cargo-nextest",
    "nextest",
    "--tool-config-file",
    "specforge:{config}",
];

#[specforge_test_macros::test(
    behavior = "approve_capability",
    verify = "a command whose argv names a placeholder admits no request, its expansion included"
)]
fn a_templated_argv_admits_no_request() {
    let expanded = [
        "cargo-nextest",
        "nextest",
        "--tool-config-file",
        "specforge:/p/.specforge/reports/nextest.nextest.toml",
    ];
    let other = [
        "cargo-nextest",
        "nextest",
        "--tool-config-file",
        "specforge:/elsewhere/setup.toml",
    ];
    let scope = process(&NEXTEST, Some("x"));
    for asked in [&expanded[..], &other, &NEXTEST] {
        for user in [false, true] {
            assert!(!rules(&scope).admits(&run(asked), user), "{asked:?}");
        }
    }
    let report = process(&["tool", "--out={report}"], None);
    assert!(!rules(&report).admits(&run(&["tool", "--out=/p/r.xml"]), false));
    // A command without a placeholder admits exactly itself.
    let plain = process(&["cargo", "test"], None);
    assert!(rules(&plain).admits(&run(&["cargo", "test"]), false));
    assert!(!rules(&plain).admits(&run(&["cargo", "test", "--all"]), false));
}

#[specforge_test_macros::test(
    behavior = "approve_capability",
    verify = "a changed config file asks again, as a changed command does"
)]
fn a_changed_config_template_asks_again() {
    let approved = process(&NEXTEST, Some("[profile.specforge]\n"));
    assert!(rules(&approved).covers(&process(&NEXTEST, Some("[profile.specforge]\n"))));
    assert!(!rules(&approved).covers(&process(
        &NEXTEST,
        Some("[profile.specforge]\nretries = 9\n")
    )));
    assert!(!rules(&approved).covers(&process(&NEXTEST, None)));
    assert!(!rules(&process(&NEXTEST, None)).covers(&approved));
}

#[test]
fn the_summary_names_the_config_file() {
    assert_eq!(
        rules(&process(&["cargo-nextest", "{config}"], Some("x"))).summary(),
        // sha256("x") = 2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881
        "process cargo-nextest {config}, writes nextest.toml (sha256:2d711642b726)"
    );
    assert_eq!(
        rules(&process(&["cargo", "test"], None)).summary(),
        "process cargo test"
    );
}
