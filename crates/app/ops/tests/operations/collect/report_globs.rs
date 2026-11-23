//! A collector's report may be a glob over project files the tool writes itself (ADR 0389): after a run the host
//! hands the export every fresh match of the shared walk (ADR 0293 D4), each held by containment.

use super::{Fixture, Machine, Runner, paths};
use specforge_ops::analysis::collect::{CollectField, Mode, Outcome, RunnerOutput};
use specforge_ops::asking::Asking;
use specforge_ops::testing::{ScriptedMachine, ScriptedRun};
use specforge_ops::{Containment, Refusal};
use specforge_test_macros::test as specforge_test;

const GLOB: &str = "**/rust-sbom.cdx.json";

/// Collector `acme` reporting `GLOB`, run as `acme` (no `{report}`).
fn sbom() -> Runner {
    Runner::new("acme").run(&["acme"]).report(GLOB)
}

/// A run writing `files` (relative to the project root, a glob's base) and exiting 0.
fn writing(files: &[&str]) -> ScriptedRun {
    ScriptedRun {
        files: files
            .iter()
            .map(|path| ((*path).into(), "{}".to_string()))
            .collect(),
        exit_code: Some(0),
        ..ScriptedRun::default()
    }
}

/// Run `acme`, named, with every capability approved.
fn run(fx: &Fixture, machine: &Machine) -> Result<Outcome, Refusal<CollectField>> {
    fx.collect_with(
        Some("acme"),
        Mode::Run(RunnerOutput::Discard),
        &Containment::unbounded(),
        Asking::Nobody,
        machine,
    )
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_report_glob_hands_over_every_match_the_run_wrote_sorted() {
    let fx = Fixture::new(vec![sbom()]);
    let machine = Machine::new()
        .approving_everything()
        .answering(writing(&["b/c/rust-sbom.cdx.json", "a/rust-sbom.cdx.json"]));

    run(&fx, &machine).unwrap();

    assert_eq!(
        paths(&fx.dispatched()[0]),
        ["a/rust-sbom.cdx.json", "b/c/rust-sbom.cdx.json"]
    );
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_match_older_than_the_run_is_not_handed_over_and_nothing_is_removed() {
    let fx = Fixture::new(vec![sbom()]);
    fx.write("old/rust-sbom.cdx.json", "{}");
    std::fs::File::options()
        .write(true)
        .open(fx.root().join("old/rust-sbom.cdx.json"))
        .unwrap()
        .set_modified(ScriptedMachine::started() - std::time::Duration::from_secs(1))
        .unwrap();
    let machine = Machine::new()
        .approving_everything()
        .answering(writing(&["a/rust-sbom.cdx.json"]));

    run(&fx, &machine).unwrap();

    assert_eq!(paths(&fx.dispatched()[0]), ["a/rust-sbom.cdx.json"]);
    assert!(fx.exists("old/rust-sbom.cdx.json"), "nothing is removed");
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_report_glob_skips_build_output_and_enters_no_link() {
    let fx = Fixture::new(vec![sbom()]);
    #[cfg(unix)]
    let outside = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        std::fs::write(outside.path().join("rust-sbom.cdx.json"), "{}").unwrap();
        std::os::unix::fs::symlink(outside.path(), fx.root().join("linked")).unwrap();
    }
    let machine = Machine::new().approving_everything().answering(writing(&[
        "target/debug/rust-sbom.cdx.json",
        "node_modules/x/rust-sbom.cdx.json",
        "a/rust-sbom.cdx.json",
    ]));

    run(&fx, &machine).unwrap();

    assert_eq!(paths(&fx.dispatched()[0]), ["a/rust-sbom.cdx.json"]);
}

#[cfg(unix)]
#[specforge_test(requirement = "FEATURE-128")]
fn a_match_linking_out_of_the_project_is_not_read_and_named_in_a_w203() {
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("bom.json"), "{\"secret\": 1}").unwrap();
    let linked = |fx: &Fixture| {
        std::fs::create_dir(fx.root().join("a")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("bom.json"),
            fx.root().join("a/rust-sbom.cdx.json"),
        )
        .unwrap();
    };
    let collect = |fx: &Fixture, machine: &Machine| {
        fx.collect_with(
            Some("acme"),
            Mode::Run(RunnerOutput::Discard),
            &Containment::within(fx.root()),
            Asking::Nobody,
            machine,
        )
    };

    let fx = Fixture::new(vec![sbom()]);
    linked(&fx);
    let machine = Machine::new()
        .approving_everything()
        .answering(writing(&["b/rust-sbom.cdx.json"]));
    let outcome = collect(&fx, &machine).unwrap();
    assert_eq!(paths(&fx.dispatched()[0]), ["b/rust-sbom.cdx.json"]);
    let warned: Vec<&str> = outcome
        .diagnostics
        .iter()
        .filter(|d| d.code == "W203")
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(warned.len(), 1, "{warned:?}");
    assert!(
        warned[0].contains("'acme'") && warned[0].contains("a/rust-sbom.cdx.json"),
        "{}",
        warned[0]
    );

    // The only match links out: the "no report" refusal names it.
    let fx = Fixture::new(vec![sbom()]);
    linked(&fx);
    let machine = Machine::new()
        .approving_everything()
        .answering(writing(&[]));
    let refusal = collect(&fx, &machine).unwrap_err();
    assert_eq!(refusal.error.code, "E045", "{refusal:?}");
    assert!(
        refusal.error.message.contains("produced no report")
            && refusal.error.message.contains("a/rust-sbom.cdx.json"),
        "{}",
        refusal.error.message
    );
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_report_glob_with_a_report_placeholder_or_a_capture_is_e058() {
    let told = Runner::new("acme").report(GLOB);
    let configured = sbom()
        .run(&["acme", "--config", "{config}"])
        .configuring("acme.toml", "out = \"{report}\"");
    let captured = sbom().capturing_stdout();
    for runner in [told, configured, captured] {
        let fx = Fixture::new(vec![runner]);
        let machine = Machine::new().approving_everything();
        let refusal = run(&fx, &machine).unwrap_err();
        assert_eq!(refusal.error.code, "E058", "{refusal:?}");
        assert!(
            refusal.error.message.contains("'acme'"),
            "{}",
            refusal.error.message
        );
        assert!(machine.runs().is_empty(), "{:?}", machine.runs());
    }
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_report_glob_matching_nothing_or_past_the_walk_cap_is_e045() {
    let fx = Fixture::new(vec![sbom()]);
    let machine = Machine::new()
        .approving_everything()
        .answering(writing(&["a/other.json"]));
    let refusal = run(&fx, &machine).unwrap_err();
    assert_eq!(refusal.error.code, "E045");
    assert!(
        refusal.error.message.contains("produced no report"),
        "{refusal:?}"
    );

    let fx = Fixture::new(vec![sbom()]);
    std::fs::create_dir(fx.root().join("many")).unwrap();
    for i in 0..20_001 {
        std::fs::write(fx.root().join(format!("many/{i}")), "").unwrap();
    }
    let machine = Machine::new()
        .approving_everything()
        .answering(writing(&["a/rust-sbom.cdx.json"]));
    let refusal = run(&fx, &machine).unwrap_err();
    assert_eq!(refusal.error.code, "E045");
    assert!(
        refusal.error.message.contains("20000"),
        "{}",
        refusal.error.message
    );
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_report_glob_runs_with_its_base_as_the_report_and_no_report_variable() {
    let fx = Fixture::new(vec![sbom()]);
    let machine = Machine::new()
        .approving_everything()
        .answering(writing(&["a/rust-sbom.cdx.json"]));

    run(&fx, &machine).unwrap();

    let runs = machine.runs();
    assert_eq!(runs[0].report, fx.root());
    assert!(!runs[0].report_in_env);

    let fx = Fixture::new(vec![
        Runner::new("acme").run(&["acme"]).report("out/**/a.json"),
    ]);
    let machine = Machine::new()
        .approving_everything()
        .answering(writing(&["x/a.json"]));
    run(&fx, &machine).unwrap();
    assert_eq!(machine.runs()[0].report, fx.root().join("out"));
    assert_eq!(paths(&fx.dispatched()[0]), ["out/x/a.json"]);
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_report_glob_ending_in_a_wildcard_is_e058_and_nothing_runs() {
    for glob in ["**/*", "**/*.pem", "**/.env*", "keys/?.json"] {
        let fx = Fixture::new(vec![Runner::new("acme").run(&["acme"]).report(glob)]);
        fx.write("a/id.pem", "secret");
        let machine = Machine::new().approving_everything();
        let refusal = run(&fx, &machine).unwrap_err();
        assert_eq!(refusal.error.code, "E058", "{glob}: {refusal:?}");
        assert!(
            refusal.error.message.contains("literal file name"),
            "{}",
            refusal.error.message
        );
        assert!(machine.runs().is_empty(), "{:?}", machine.runs());
        assert!(fx.dispatched().is_empty(), "{glob}");
    }
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_report_over_the_byte_cap_is_e045_naming_it() {
    let fx = Fixture::new(vec![sbom()]);
    let half = specforge_ops::analysis::collect::REPORT_BYTES_MAX / 2 + 1;
    for dir in ["a", "b"] {
        std::fs::create_dir(fx.root().join(dir)).unwrap();
        let file = std::fs::File::create(fx.root().join(dir).join("rust-sbom.cdx.json")).unwrap();
        file.set_len(half).unwrap();
    }
    let machine = Machine::new()
        .approving_everything()
        .answering(writing(&[]));
    let refusal = run(&fx, &machine).unwrap_err();
    assert_eq!(refusal.error.code, "E045", "{refusal:?}");
    assert!(
        refusal.error.message.contains("67108864"),
        "{}",
        refusal.error.message
    );
    assert!(fx.dispatched().is_empty());
}
