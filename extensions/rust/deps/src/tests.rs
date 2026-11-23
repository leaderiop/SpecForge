use super::*;
use serde_json::{json, Value};
use specforge_extension_sdk::ExtensionDeclaration;
use specforge_test::prelude::*;

/// The CycloneDX reader's hand-written cargo-cyclonedx BOM of the root package `demo`.
const DEMO_BOM: &str = include_str!("../../../code/cyclonedx/fixtures/cargo.cdx.json");
/// The hand-written BOM of the member `ledger` at `crates/ledger`.
const LEDGER_BOM: &str = include_str!("../fixtures/ledger.cdx.json");
/// The OSV reader's osv-scanner 2.7.1 recording: `Cargo.lock` (smallvec 1.6.0) and `web/package-lock.json` (lodash
/// 4.17.15) of `/project`.
const OSV_SCANNER_2_7_1: &str = include_str!("../../../code/osv/fixtures/osv-scanner-2.7.1.json");
/// The served `package` facts: `pkg:cargo/demo` at the root, `pkg:cargo/ledger` at `crates/ledger`.
const PACKAGES: &str = include_str!("../fixtures/packages.json");

/// The input the host sends for these `(path, content)` reports at `/project`, the pack's packages served.
fn input(reports: &[(&str, &str)]) -> CollectInput {
    CollectInput {
        reports: reports
            .iter()
            .map(|(path, content)| CollectReportFile {
                path: path.to_string(),
                content: content.to_string(),
            })
            .collect(),
        cwd: Some("/project".to_string()),
        facts: serde_json::from_str(PACKAGES).expect("the served facts parse"),
        ..CollectInput::default()
    }
}

/// The process capability of the collector `name`, with no filter.
fn capability(declaration: &ExtensionDeclaration, name: &str) -> Value {
    let capabilities = serde_json::to_value(&declaration.capabilities).unwrap();
    let capability = capabilities
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("a capability {name}: {capabilities}"))
        .clone();
    assert_eq!(capability["port"], "process");
    assert!(capability.get("filter").is_none_or(Value::is_null));
    capability
}

#[specforge_test(requirement = "FEATURE-148")]
fn rust_deps_declares_cargo_cyclonedx_over_a_glob_and_osv_scanner_over_the_lockfile() {
    let declaration = crate::specforge_extension_build().declaration();
    let peers: Vec<(&str, &str, bool)> = declaration
        .handshake
        .peer_dependencies
        .iter()
        .map(|p| (p.name.as_str(), p.version.as_str(), p.optional))
        .collect();
    assert_eq!(
        peers,
        [
            ("@code/core", "^1.2", false),
            ("@rust/source", "^1.0", false)
        ]
    );
    let names: Vec<&str> = declaration
        .collectors
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, [SBOM, OSV]);

    let sbom = &declaration.collectors[0];
    assert_eq!(sbom.input_formats, ["cyclonedx-json"]);
    assert_eq!(
        sbom.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["Cargo.toml".to_string()][..])
    );
    assert_eq!(sbom.answers, ["dependency"]);
    assert_eq!(sbom.inputs, [Input::facts(&["package"])]);
    assert_eq!(sbom.report.as_deref(), Some("**/rust-sbom.cdx.json"));
    assert_eq!(
        sbom.install.as_deref(),
        Some("cargo install cargo-cyclonedx --locked")
    );
    let run = capability(&declaration, SBOM);
    assert_eq!(
        run["argv"],
        json!([
            "cargo-cyclonedx",
            "cyclonedx",
            "--format",
            "json",
            "--spec-version",
            "1.5",
            "--all-features",
            "--override-filename",
            "rust-sbom.cdx.json"
        ])
    );
    assert_eq!(
        run["env"],
        json!([
            "CARGO_*",
            "RUSTUP_*",
            "RUSTC",
            "RUSTC_*",
            "SOURCE_DATE_EPOCH"
        ])
    );

    let osv = &declaration.collectors[1];
    assert_eq!(osv.input_formats, ["osv-json"]);
    assert_eq!(
        osv.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["Cargo.lock".to_string()][..])
    );
    assert_eq!(osv.answers, ["vulnerability"]);
    assert_eq!(osv.inputs, [Input::facts(&["package"])]);
    assert_eq!(osv.report.as_deref(), Some(OSV_REPORT));
    let install = osv.install.as_deref().expect("an install line");
    assert!(
        specforge_protocol_types::is_install_line(install),
        "{install}"
    );
    let run = capability(&declaration, OSV);
    assert_eq!(
        run["argv"],
        json!([
            "osv-scanner",
            "scan",
            "source",
            "--lockfile",
            "Cargo.lock",
            "--format",
            "json",
            "--output-file",
            "{report}"
        ])
    );
    assert_eq!(
        run["env"],
        json!([
            "HTTPS_PROXY",
            "HTTP_PROXY",
            "NO_PROXY",
            "https_proxy",
            "http_proxy",
            "no_proxy",
            "OSV_*"
        ])
    );
}

#[specforge_test(requirement = "FEATURE-148")]
fn every_bom_the_glob_hands_in_mints_the_dependencies_of_its_own_package() {
    // Given out of order: the collector reads them sorted by path.
    let out = collect_sbom(&input(&[
        ("crates/ledger/rust-sbom.cdx.json", LEDGER_BOM),
        ("rust-sbom.cdx.json", DEMO_BOM),
    ]))
    .unwrap();
    let minted: Vec<(&str, &str)> = out
        .facts
        .iter()
        .map(|f| {
            let package = f.fields["package"].as_str().unwrap();
            assert!(
                f.id.starts_with(&format!("dep:{package}/")),
                "{} of {package}",
                f.id
            );
            (package, f.fields["name"].as_str().unwrap())
        })
        .collect();
    assert_eq!(
        minted,
        [
            ("pkg:cargo/demo", "itoa"),
            ("pkg:cargo/demo", "serde"),
            ("pkg:cargo/demo", "serde_derive"),
            ("pkg:cargo/ledger", "rust_decimal"),
        ]
    );
    let direct: Vec<Option<bool>> = out
        .facts
        .iter()
        .map(|f| f.fields.get("direct").and_then(Value::as_bool))
        .collect();
    assert_eq!(direct, [Some(false), Some(true), Some(false), Some(true)]);
    assert_eq!(out.warnings, Vec::<String>::new());
}

#[specforge_test(requirement = "FEATURE-148")]
fn no_bom_or_one_about_no_served_package_is_the_sbom_collectors_error() {
    let why = collect_sbom(&input(&[])).unwrap_err();
    assert!(why.contains(SBOM) && why.contains(SBOM_REPORT), "{why}");
    let stranger = LEDGER_BOM.replace("pkg:cargo/ledger@", "pkg:cargo/stranger@");
    let why = collect_sbom(&input(&[
        ("rust-sbom.cdx.json", DEMO_BOM),
        ("crates/stranger/rust-sbom.cdx.json", &stranger),
    ]))
    .unwrap_err();
    assert!(
        why.starts_with("crates/stranger/rust-sbom.cdx.json: ")
            && why.contains("names no collected package"),
        "{why}"
    );
    let why = collect_sbom(&input(&[("rust-sbom.cdx.json", "garbage")])).unwrap_err();
    assert!(why.starts_with("rust-sbom.cdx.json: no JSON"), "{why}");
}

#[specforge_test(requirement = "FEATURE-148")]
fn a_recorded_osv_scan_cites_the_root_crate_and_warns_of_each_package_it_skips() {
    let out = collect_osv(&input(&[(OSV_REPORT, OSV_SCANNER_2_7_1)])).unwrap();
    let ids: Vec<&str> = out.facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "vuln:pkg:cargo/demo/GHSA-43w2-9j62-hq99",
            "vuln:pkg:cargo/demo/RUSTSEC-2021-0003"
        ]
    );
    // lodash of web/package-lock.json: this pack serves no npm package, so it is named, not dropped.
    assert_eq!(
        out.warnings,
        ["'lodash' 4.17.15 in 'web/package-lock.json' (5 records) is skipped: no collected pkg:npm package lies \
          at or under the lockfile's directory"]
    );
}

#[specforge_test(requirement = "FEATURE-148")]
fn zero_or_two_scans_or_a_bad_one_is_the_osv_collectors_error() {
    for reports in [
        &[][..],
        &[
            (OSV_REPORT, OSV_SCANNER_2_7_1),
            (OSV_REPORT, OSV_SCANNER_2_7_1),
        ][..],
    ] {
        let why = collect_osv(&input(reports)).unwrap_err();
        assert!(why.contains(OSV) && why.contains(OSV_REPORT), "{why}");
    }
    let why = collect_osv(&input(&[(OSV_REPORT, "garbage")])).unwrap_err();
    assert!(why.starts_with(&format!("{OSV_REPORT}: no JSON")), "{why}");
    let no_cwd = CollectInput {
        cwd: None,
        ..input(&[(OSV_REPORT, OSV_SCANNER_2_7_1)])
    };
    assert!(collect_osv(&no_cwd).unwrap_err().contains("no cwd"));
}
