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

#[specforge_test_macros::test(requirement = "FEATURE-128")]
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

#[specforge_test_macros::test(requirement = "FEATURE-128")]
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

/// The process scope of `argv` with a caller's `filter` arguments and inherited `env` names.
fn filtered(argv: &[&str], filter: &[&str], env: &[&str]) -> PortScope {
    PortScope::Process(ProcessScope {
        argv: argv.iter().map(|a| a.to_string()).collect(),
        filter: filter.iter().map(|a| a.to_string()).collect(),
        env: env.iter().map(|a| a.to_string()).collect(),
        ..ProcessScope::default()
    })
}

#[test]
fn a_process_summary_names_its_filter() {
    let scope = filtered(
        &["cargo-nextest", "nextest", "run"],
        &["-E", "{filter}"],
        &["CARGO_*", "RUSTFLAGS"],
    );
    assert_eq!(
        rules(&scope).summary(),
        "process cargo-nextest nextest run, runs in the caller's environment less credentials (reads \
         CARGO_*,RUSTFLAGS), takes a filter as -E {filter}"
    );
    // A command a guest may run inherits what it declares; neither passes a credential.
    assert_eq!(
        rules(&filtered(&["env"], &[], &["CARGO_*", "RUSTFLAGS"])).summary(),
        "process env, inherits or sets CARGO_*,RUSTFLAGS, never a credential"
    );
    // The filter is part of the approved scope: another fragment asks again.
    let other = filtered(
        &["cargo-nextest", "nextest", "run"],
        &["--filterset", "{filter}"],
        &["CARGO_*", "RUSTFLAGS"],
    );
    assert!(rules(&scope).covers(&scope));
    assert!(!rules(&scope).covers(&other));
    assert!(!rules(&scope).covers(&filtered(
        &["cargo-nextest", "nextest", "run"],
        &[],
        &["CARGO_*", "RUSTFLAGS"]
    )));
}

#[test]
fn a_guest_request_on_a_scope_with_a_filter_is_refused() {
    let scope = filtered(&["cargo", "test"], &["--", "{filter}"], &[]);
    for asked in [
        &["cargo", "test"][..],
        &["cargo", "test", "--", "api"],
        &["cargo", "test", "--", "{filter}"],
    ] {
        for user in [false, true] {
            assert!(!rules(&scope).admits(&run(asked), user), "{asked:?}");
        }
    }
    assert!(rules(&filtered(&["cargo", "test"], &[], &[])).admits(&run(&["cargo", "test"]), false));
}
