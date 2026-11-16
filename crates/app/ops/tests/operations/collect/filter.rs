//! A caller's filter (ADR 0360): the one selected collector's command appends its declared filter arguments with the
//! caller's text; a collect that runs nothing, several collectors, a collector declaring none or a refused text stops
//! before any command.

use super::{Fixture, Machine, Runner};
use specforge_effects::Approval;
use specforge_ops::analysis::collect::{CollectField, Mode, RunnerOutput};
use specforge_ops::asking::Asking;
use specforge_ops::testing::ScriptedRun;
use specforge_ops::{Containment, OpErrorKind};
use specforge_protocol_types::PortScope;

/// A run that writes the runner's report file and exits 0.
fn reports() -> ScriptedRun {
    ScriptedRun {
        files: vec![("".into(), "{}".to_string())],
        exit_code: Some(0),
        ..ScriptedRun::default()
    }
}

/// `acme`, whose command takes a filter as `--only <text>`.
fn filtering() -> Fixture {
    Fixture::new(vec![
        Runner::new("acme")
            .proving("a", "t")
            .filtering(&["--only", "{filter}"]),
    ])
}

/// Collect `runner` in `mode` with `filter`, asking nobody.
fn collect(
    fx: &Fixture,
    runner: Option<&str>,
    mode: Mode<'_>,
    filter: &str,
    machine: &Machine,
) -> Result<specforge_ops::analysis::collect::Outcome, specforge_ops::Refusal<CollectField>> {
    fx.collect_filtered(
        runner,
        mode,
        Some(filter),
        &Containment::unbounded(),
        Asking::Nobody,
        machine,
    )
}

const RUN: Mode<'static> = Mode::Run(RunnerOutput::Discard);

#[test]
fn a_filter_is_appended_through_the_collectors_declared_arguments() {
    let fx = filtering();
    let machine = Machine::new()
        .approving(fx.approval("acme"))
        .answering(reports())
        .answering(reports());

    collect(&fx, None, RUN, "api & not slow", &machine).unwrap();
    fx.run(Asking::Nobody, &machine).unwrap();

    let runs = machine.runs();
    let out = format!("--out={}", fx.report_of("acme").display());
    assert_eq!(runs[0].argv, ["acme", &out, "--only", "api & not slow"]);
    assert_eq!(
        runs[1].argv,
        ["acme", &out],
        "no filter, no filter arguments"
    );
}

#[test]
fn one_approval_covers_any_filter_text() {
    let fx = filtering();
    let machine = Machine::new()
        .approving(fx.approval("acme"))
        .answering(reports())
        .answering(reports());

    collect(&fx, Some("acme"), RUN, "first", &machine).unwrap();
    collect(&fx, Some("acme"), RUN, "second; rm -rf /", &machine).unwrap();

    let runs = machine.runs();
    assert_eq!(runs.len(), 2, "nothing asked: {runs:?}");
    assert_eq!(
        runs[1].argv.last().unwrap(),
        "second; rm -rf /",
        "one argument"
    );
}

#[test]
fn a_changed_filter_declaration_asks_again() {
    let fx = filtering();
    let Approval::Capability(mut approval) = fx.approval("acme") else {
        panic!("a capability approval");
    };
    let PortScope::Process(process) = &mut approval.granted else {
        panic!("a process scope");
    };
    process.filter = vec!["--grep".to_string(), "{filter}".to_string()];
    let machine = Machine::new()
        .approving(Approval::Capability(approval))
        .answering(reports());

    let refusal = collect(&fx, None, RUN, "api", &machine).unwrap_err();

    assert_eq!(refusal.error.code, "E059", "{refusal:?}");
    assert!(machine.runs().is_empty(), "{:?}", machine.runs());
}

#[test]
fn a_filter_for_a_collector_declaring_none_is_refused() {
    let fx = Fixture::new(vec![Runner::new("acme").proving("a", "t")]);
    let machine = Machine::new().approving_everything().answering(reports());

    let refusal = collect(&fx, None, RUN, "api", &machine).unwrap_err();

    assert_eq!(refusal.error.code, "E058");
    assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
    assert_eq!(refusal.subject, Some(CollectField::Filter));
    assert!(
        refusal.error.message.contains("declares no filter"),
        "{refusal:?}"
    );
    assert!(machine.runs().is_empty());
}

#[test]
fn a_filter_with_no_run_or_reports_is_refused() {
    let fx = filtering();
    fx.write("mine.json", "{}");
    let files = [fx.root().join("mine.json")];
    let machine = Machine::new().approving_everything();

    for mode in [Mode::NoRun, Mode::Reports(&files)] {
        let refusal = collect(&fx, Some("acme"), mode, "api", &machine).unwrap_err();
        assert_eq!(refusal.error.code, "E058");
        assert_eq!(refusal.subject, Some(CollectField::Filter));
        assert!(
            refusal.error.message.contains("without running"),
            "{refusal:?}"
        );
    }
    assert!(machine.runs().is_empty());
    assert!(fx.report().is_none(), "nothing recorded");
}

#[test]
fn a_filter_needs_one_collector() {
    let fx = Fixture::new(vec![
        Runner::new("first").filtering(&["{filter}"]),
        Runner::new("second").filtering(&["{filter}"]),
    ]);
    fx.write("first.cfg", "");
    fx.write("second.cfg", "");
    let machine = Machine::new().approving_everything().answering(reports());

    let refusal = collect(&fx, None, RUN, "api", &machine).unwrap_err();
    assert_eq!(refusal.subject, Some(CollectField::Filter));
    assert!(
        refusal.error.message.contains("first, second apply here"),
        "{refusal:?}"
    );
    assert!(machine.runs().is_empty());

    collect(&fx, Some("second"), RUN, "api", &machine).unwrap();
    assert_eq!(machine.runs()[0].argv.last().unwrap(), "api");
}

#[test]
fn a_filter_text_the_protocol_refuses_stops_before_any_command() {
    let fx = filtering();
    let machine = Machine::new().approving_everything();

    for text in ["", "-x", "--workspace", "a\nb"] {
        let refusal = collect(&fx, None, RUN, text, &machine).unwrap_err();
        assert_eq!(refusal.error.code, "E058", "{text:?}");
        assert_eq!(refusal.subject, Some(CollectField::Filter), "{text:?}");
    }
    assert!(machine.runs().is_empty());
}
