use super::*;
use crate::analysis::doctor::{DoctorReport, Part, diagnose};
use crate::testing::{ScriptedSolver, Viewed};
use specforge_effects::testing::FakeProcess;
use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
use specforge_project::testing::Assembled;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_test_macros::test as specforge_test;

/// `extension` with one collector `name` detected on `Cargo.toml`, running `run`, answering `answers`.
fn runner(
    extension: &str,
    name: &str,
    run: &[&str],
    answers: &[&str],
    install: Option<&str>,
) -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(extension, "1.0.0"));
    c.collector(name, |collector| {
        collector
            .detect_files(&["Cargo.toml"])
            .run(run)
            .answers(answers);
        if let Some(line) = install {
            collector.install(line);
        }
        // `{config}` in an argv is sound only beside the config file it names.
        if run.iter().any(|arg| arg.contains("{config}")) {
            collector.config("jest.json", "{\"output\": \"{report}\"}");
        }
        collector.collect(|_| Ok(Default::default()));
    });
    c.declaration()
}

/// `@acme/core`, cataloguing codes judging collected kinds: W104 coverage, W108 proof.
fn judge() -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/core", "1.0.0"));
    c.code("W104", |d| {
        d.title("Coverage on no item")
            .explanation("Serve the item.")
            .facts(&["coverage"]);
    });
    c.code("W108", |d| {
        d.title("Proof on no item")
            .explanation("Serve the item.")
            .facts(&["proof"]);
    });
    c.declaration()
}

fn cov() -> ExtensionDeclaration {
    runner(
        "@acme/cov",
        "cov",
        &["cargo-llvm-cov", "llvm-cov"],
        &["coverage"],
        Some("cargo install cargo-llvm-cov --locked"),
    )
}

/// The report of a project loading `declarations` and holding the empty `files`, diagnosed under `process`.
fn diagnosed(
    declarations: Vec<ExtensionDeclaration>,
    files: &[&str],
    process: &FakeProcess,
) -> DoctorReport {
    let fixture = Assembled::new().declarations(declarations);
    for file in files {
        fixture.write(file, "");
    }
    let view = fixture.view();
    diagnose(&view, &ScriptedSolver::ready()).with_tools(&view, process)
}

fn tools(report: &DoctorReport) -> Vec<&Finding> {
    report.about(Part::Tools).collect()
}

/// The missing programs `run`, by `@x/run`'s one collector, gives in a project holding `Cargo.toml` and `files`.
fn missing(run: &[&str], files: &[&str], process: &FakeProcess) -> Vec<String> {
    let mut held = vec!["Cargo.toml"];
    held.extend(files);
    let report = diagnosed(
        vec![runner("@x/run", "run", run, &[], None)],
        &held,
        process,
    );
    tools(&report)
        .iter()
        .map(|f| match &f.about {
            About::Tools { program, .. } => program.clone(),
            other => panic!("{other:?}"),
        })
        .collect()
}

const API_EXTRACTOR: &[&str] = &["npx", "--no", "--", "api-extractor", "run"];

#[specforge_test(requirement = "FEATURE-021")]
fn npx_names_the_program_after_its_double_dash() {
    assert_eq!(
        missing(API_EXTRACTOR, &[], &FakeProcess::new()),
        ["api-extractor"]
    );
    let npx_only = FakeProcess::new().on_path("npx", "/usr/bin/npx");
    assert_eq!(
        missing(API_EXTRACTOR, &[], &npx_only),
        ["api-extractor"],
        "npx on PATH says nothing of the program it would run"
    );
}

#[specforge_test(requirement = "FEATURE-021")]
fn an_npx_program_in_node_modules_bin_or_on_path_is_present() {
    for bin in [
        "node_modules/.bin/api-extractor",
        "node_modules/.bin/api-extractor.cmd",
    ] {
        assert!(
            missing(API_EXTRACTOR, &[bin], &FakeProcess::new()).is_empty(),
            "{bin}"
        );
    }
    let on_path = FakeProcess::new().on_path("api-extractor", "/usr/bin/api-extractor");
    assert!(missing(API_EXTRACTOR, &[], &on_path).is_empty());
}

const JEST: &[&str] = &[
    "node",
    "--env-file={config}",
    "node_modules/jest/bin/jest.js",
];

#[specforge_test(requirement = "FEATURE-021")]
fn a_node_script_missing_from_node_modules_names_its_package() {
    let node = FakeProcess::new().on_path("node", "/usr/bin/node");
    assert_eq!(missing(JEST, &[], &node), ["jest"]);
    assert!(missing(JEST, &["node_modules/jest/bin/jest.js"], &node).is_empty());
    let scoped = &["node", "node_modules/@acme/tool/cli.js"];
    assert_eq!(missing(scoped, &[], &node), ["@acme/tool"]);
}

#[specforge_test(requirement = "FEATURE-021")]
fn without_node_the_program_is_node() {
    assert_eq!(
        missing(
            JEST,
            &["node_modules/jest/bin/jest.js"],
            &FakeProcess::new()
        ),
        ["node"]
    );
}

#[specforge_test(requirement = "FEATURE-021")]
fn a_script_path_leaving_the_root_is_never_probed_and_reads_as_missing() {
    let node = FakeProcess::new().on_path("node", "/usr/bin/node");
    let escaping = &["node", "node_modules/../../x.js"];

    assert_eq!(missing(escaping, &[], &node), ["node_modules/../../x.js"]);
}

#[specforge_test(requirement = "FEATURE-021")]
fn collectors_sharing_a_missing_program_give_one_finding_with_the_union_of_codes() {
    let a = runner("@x/a", "a", &["cargo-kani"], &["proof"], None);
    let b = runner("@x/b", "b", &["cargo-kani"], &["coverage"], None);

    let report = diagnosed(vec![b, a, judge()], &["Cargo.toml"], &FakeProcess::new());

    let found = tools(&report);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].about,
        About::Tools {
            program: "cargo-kani".into(),
            collectors: vec!["a (@x/a)".into(), "b (@x/b)".into()],
            codes: vec!["acme/W104".into(), "acme/W108".into()],
        }
    );
    assert_eq!(
        found[0].remediation,
        "install `cargo-kani` and put it on PATH"
    );
}

/// The codes of the one tools finding of `report`.
fn codes_of(report: &DoctorReport) -> Vec<String> {
    let found = tools(report);
    let [finding] = &found[..] else {
        panic!("one finding: {found:?}");
    };
    let About::Tools { codes, .. } = &finding.about else {
        panic!("{:?}", finding.about);
    };
    codes.clone()
}

#[specforge_test(requirement = "FEATURE-021")]
fn a_kind_a_present_collector_still_answers_keeps_its_codes_judged() {
    // c8 still answers coverage, so losing cargo-llvm-cov switches no code off.
    let c8 = runner(
        "@acme/jscov",
        "jscov",
        &["npx", "--no", "--", "c8"],
        &["coverage"],
        None,
    );
    let process = FakeProcess::new().on_path("c8", "/usr/bin/c8");

    let report = diagnosed(vec![cov(), c8, judge()], &["Cargo.toml"], &process);

    assert_eq!(codes_of(&report), Vec::<String>::new());
    let check = &tools(&report)[0].check;
    assert!(check.contains("`cargo-llvm-cov`"), "{check}");
    assert!(!check.contains("not judged"), "{check}");
}

#[specforge_test(requirement = "FEATURE-021")]
fn a_code_judging_a_kind_still_answered_is_not_named() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/core", "1.0.0"));
    c.code("W104", |d| {
        d.title("Coverage on no item")
            .explanation("Serve the item.")
            .facts(&["coverage"]);
    });
    c.code("E101", |d| {
        d.title("Fact on no item")
            .explanation("Serve the item.")
            .facts(&["coverage", "item"]);
    });
    // A source reader that runs no program still answers item.
    c.collector("items", |k| {
        k.detect_files(&["Cargo.toml"])
            .answers(&["item"])
            .collect(|_| Ok(Default::default()));
    });

    let report = diagnosed(
        vec![cov(), c.declaration()],
        &["Cargo.toml"],
        &FakeProcess::new(),
    );

    assert_eq!(codes_of(&report), ["acme/W104"]);
    assert!(
        tools(&report)[0].check.contains("acme/W104"),
        "{}",
        tools(&report)[0].check
    );
}

#[specforge_test(requirement = "FEATURE-021")]
fn a_missing_program_whose_kinds_no_code_judges_still_gets_a_finding() {
    let deps = runner("@x/deps", "deps", &["osv-scanner"], &["dependency"], None);

    let report = diagnosed(vec![deps, judge()], &["Cargo.toml"], &FakeProcess::new());

    let found = tools(&report);
    assert_eq!(found.len(), 1, "{found:?}");
    let About::Tools { codes, .. } = &found[0].about else {
        panic!("{:?}", found[0].about);
    };
    assert!(codes.is_empty(), "{codes:?}");
    assert!(!found[0].check.contains("not judged"), "{}", found[0].check);
}

#[specforge_test(requirement = "FEATURE-021")]
fn ten_missing_programs_leave_doctor_healthy() {
    let programs: Vec<String> = (0..10).map(|i| format!("tool-{i}")).collect();
    let declarations: Vec<ExtensionDeclaration> = programs
        .iter()
        .map(|program| runner(&format!("@x/{program}"), program, &[program], &[], None))
        .collect();

    let report = diagnosed(declarations, &["Cargo.toml"], &FakeProcess::new());

    assert_eq!(tools(&report).len(), 10);
    assert!(report.ok(), "tool notes never fail doctor");
    assert_eq!(report.about(Part::Toolchain).count(), 0);
    assert!(report.z3_available());
}

#[specforge_test(requirement = "FEATURE-021")]
fn a_missing_program_is_one_info_finding_naming_its_collectors_and_codes() {
    let report = diagnosed(vec![cov(), judge()], &["Cargo.toml"], &FakeProcess::new());

    let found = tools(&report);
    assert_eq!(found.len(), 1, "{found:?}");
    let finding = found[0];
    assert_eq!(finding.status, FindingStatus::Info);
    assert_eq!(finding.code, "tool_missing");
    let About::Tools {
        program,
        collectors,
        codes,
    } = &finding.about
    else {
        panic!("{:?}", finding.about);
    };
    assert_eq!(program, "cargo-llvm-cov");
    assert_eq!(collectors, &["cov (@acme/cov)"]);
    assert_eq!(codes, &["acme/W104"]);
    assert!(
        finding.check.contains("`cargo-llvm-cov`"),
        "{}",
        finding.check
    );
    assert!(finding.check.contains(&codes[0]), "{}", finding.check);
    assert_eq!(finding.remediation, "cargo install cargo-llvm-cov --locked");
    assert!(report.ok(), "a missing tool never fails doctor");
    assert!(report.z3_available(), "a tool note is no toolchain finding");
}

#[specforge_test(requirement = "FEATURE-021")]
fn a_program_on_path_says_nothing() {
    let process = FakeProcess::new().on_path("cargo-llvm-cov", "/usr/bin/cargo-llvm-cov");

    let report = diagnosed(vec![cov(), judge()], &["Cargo.toml"], &process);

    assert!(tools(&report).is_empty(), "{:?}", tools(&report));
    assert!(process.requests().is_empty(), "doctor runs nothing");
}

#[specforge_test(requirement = "FEATURE-021")]
fn a_collector_not_detected_here_is_not_probed() {
    let report = diagnosed(vec![cov(), judge()], &[], &FakeProcess::new());

    assert!(tools(&report).is_empty(), "{:?}", tools(&report));
}

#[specforge_test(requirement = "FEATURE-021")]
fn the_document_carries_the_tool_finding_and_its_fields() {
    let report = diagnosed(vec![cov(), judge()], &["Cargo.toml"], &FakeProcess::new());

    let document = serde_json::to_value(report.document()).unwrap();
    let finding = document["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["about"] == "tools")
        .expect("a tools finding");
    assert_eq!(finding["program"], "cargo-llvm-cov");
    assert_eq!(
        finding["collectors"],
        serde_json::json!(["cov (@acme/cov)"])
    );
    assert_eq!(finding["codes"].as_array().unwrap().len(), 1);
    assert_eq!(finding["status"], "info");
}
