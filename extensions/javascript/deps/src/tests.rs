use super::*;
use serde_json::{json, Value};
use specforge_extension_sdk::ExtensionDeclaration;
use specforge_test::prelude::*;

/// cyclonedx-npm 7.0.1's BOM of the project `board`, recorded by the Phase 11 research.
const CYCLONEDX_NPM_7_0_1: &str = include_str!("../fixtures/cyclonedx-npm-7.0.1.cdx.json");
/// The OSV reader's osv-scanner 2.7.1 recording: `Cargo.lock` (smallvec 1.6.0) and `web/package-lock.json` (lodash
/// 4.17.15) of `/project`.
const OSV_SCANNER_2_7_1: &str = include_str!("../../../code/osv/fixtures/osv-scanner-2.7.1.json");
/// The served `package` facts: `pkg:npm/board` at the root, `pkg:npm/web` at `web`.
const PACKAGES: &str = include_str!("../fixtures/packages.json");

/// The input the host sends for these reports, each at `path`, at `/project` with the pack's packages served.
fn input(path: &str, reports: &[&str]) -> CollectInput {
    CollectInput {
        reports: reports
            .iter()
            .map(|content| CollectReportFile {
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

#[specforge_test(requirement = "FEATURE-147")]
fn javascript_deps_declares_cyclonedx_npm_over_the_lockfile_and_its_peers() {
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
            ("@javascript/source", "^1.0", false)
        ]
    );
    let collector = declaration.collectors.iter().find(|c| c.name == SBOM);
    let collector = collector.expect("a collector javascript-sbom");
    let capability = capability(&declaration, SBOM);
    assert_eq!(collector.input_formats, ["cyclonedx-json"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["package-lock.json".to_string()][..])
    );
    assert_eq!(collector.answers, ["dependency"]);
    assert_eq!(collector.inputs, [Input::facts(&["package"])]);
    assert_eq!(collector.report.as_deref(), Some(SBOM_REPORT));
    assert_eq!(
        collector.install.as_deref(),
        Some("npm install --save-dev @cyclonedx/cyclonedx-npm")
    );
    assert_eq!(
        capability["argv"],
        json!([
            "npx",
            "--no",
            "--",
            "cyclonedx-npm",
            "--output-format",
            "JSON",
            "--spec-version",
            "1.6",
            "--package-lock-only",
            "--output-file",
            "{report}"
        ])
    );
    assert_eq!(
        capability["env"],
        json!(["NODE_*", "NPM_CONFIG_*", "npm_config_*", "CI"])
    );
}

#[specforge_test(requirement = "FEATURE-147")]
fn a_recorded_cyclonedx_npm_bom_mints_one_dependency_per_component_of_the_root_package() {
    let bom = specforge_code_cyclonedx::read(CYCLONEDX_NPM_7_0_1).expect("the recording reads");
    // npm installs `spdx-expression-parse` 3.0.1 and `string-width` 7.2.0 twice, at two paths: one fact each.
    let purls: std::collections::BTreeSet<_> =
        bom.components.iter().map(|c| c.purl.as_deref()).collect();
    assert_eq!((bom.components.len(), purls.len()), (68, 66));
    let out = collect_sbom(&input(SBOM_REPORT, &[CYCLONEDX_NPM_7_0_1])).unwrap();
    assert_eq!(out.facts.len(), purls.len());
    for fact in &out.facts {
        assert_eq!(fact.kind, "dependency");
        assert!(fact.id.starts_with("dep:pkg:npm/board/"), "{}", fact.id);
        assert_eq!(fact.fields["package"], "pkg:npm/board");
    }
    assert_eq!(out.warnings, Vec::<String>::new());
}

#[specforge_test(requirement = "FEATURE-147")]
fn a_bom_about_no_served_package_is_an_error_naming_the_report() {
    let other = CYCLONEDX_NPM_7_0_1.replacen(
        "\"purl\": \"pkg:npm/board@1.0.0\"",
        "\"purl\": \"pkg:npm/elsewhere@1.0.0\"",
        1,
    );
    assert_ne!(other, CYCLONEDX_NPM_7_0_1, "the recording names its root");
    let why = collect_sbom(&input(SBOM_REPORT, &[&other])).unwrap_err();
    assert!(
        why.starts_with(&format!("{SBOM_REPORT}: ")) && why.contains("names no collected package"),
        "{why}"
    );
}

#[specforge_test(requirement = "FEATURE-147")]
fn zero_or_two_boms_or_a_bad_one_is_the_sbom_collectors_error() {
    for reports in [&[][..], &[CYCLONEDX_NPM_7_0_1, CYCLONEDX_NPM_7_0_1][..]] {
        let why = collect_sbom(&input(SBOM_REPORT, reports)).unwrap_err();
        assert!(why.contains(SBOM) && why.contains(SBOM_REPORT), "{why}");
    }
    let why = collect_sbom(&input(SBOM_REPORT, &["garbage"])).unwrap_err();
    assert!(why.starts_with(&format!("{SBOM_REPORT}: no JSON")), "{why}");
}

#[specforge_test(requirement = "FEATURE-147")]
fn javascript_deps_declares_osv_scanner_over_the_lockfile() {
    let declaration = crate::specforge_extension_build().declaration();
    let names: Vec<&str> = declaration
        .collectors
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, [SBOM, OSV]);
    let collector = &declaration.collectors[1];
    let capability = capability(&declaration, OSV);
    assert_eq!(collector.input_formats, ["osv-json"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["package-lock.json".to_string()][..])
    );
    assert_eq!(collector.answers, ["vulnerability"]);
    assert_eq!(collector.inputs, [Input::facts(&["package"])]);
    assert_eq!(collector.report.as_deref(), Some(OSV_REPORT));
    let install = collector.install.as_deref().expect("an install line");
    assert_eq!(
        install,
        "go install github.com/google/osv-scanner/v2/cmd/osv-scanner@latest (or: brew install osv-scanner)"
    );
    assert!(specforge_protocol_types::is_install_line(install));
    assert_eq!(
        capability["argv"],
        json!([
            "osv-scanner",
            "scan",
            "source",
            "--lockfile",
            "package-lock.json",
            "--format",
            "json",
            "--output-file",
            "{report}"
        ])
    );
    assert_eq!(
        capability["env"],
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

#[specforge_test(requirement = "FEATURE-147")]
fn a_recorded_osv_scan_cites_the_npm_package_and_warns_of_each_package_it_skips() {
    let out = collect_osv(&input(OSV_REPORT, &[OSV_SCANNER_2_7_1])).unwrap();
    let ids: Vec<&str> = out.facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "vuln:pkg:npm/web/GHSA-29mw-wpgm-hmr9",
            "vuln:pkg:npm/web/GHSA-35jh-r3h4-6jhm",
            "vuln:pkg:npm/web/GHSA-p6mc-m468-83gw",
            "vuln:pkg:npm/web/GHSA-r5fr-rjxr-66jc",
            "vuln:pkg:npm/web/GHSA-xxjr-mmjv-4gpg"
        ]
    );
    // smallvec of the root Cargo.lock: this pack serves no cargo package, so it is named, not dropped.
    assert_eq!(
        out.warnings,
        ["'smallvec' 1.6.0 in 'Cargo.lock' (2 records) is skipped: no collected pkg:cargo package lies at or \
          under the lockfile's directory"]
    );
}

#[specforge_test(requirement = "FEATURE-147")]
fn zero_or_two_scans_or_a_bad_one_is_the_osv_collectors_error() {
    for reports in [&[][..], &[OSV_SCANNER_2_7_1, OSV_SCANNER_2_7_1][..]] {
        let why = collect_osv(&input(OSV_REPORT, reports)).unwrap_err();
        assert!(why.contains(OSV) && why.contains(OSV_REPORT), "{why}");
    }
    let why = collect_osv(&input(OSV_REPORT, &["garbage"])).unwrap_err();
    assert!(why.starts_with(&format!("{OSV_REPORT}: no JSON")), "{why}");
    let no_cwd = CollectInput {
        cwd: None,
        ..input(OSV_REPORT, &[OSV_SCANNER_2_7_1])
    };
    assert!(collect_osv(&no_cwd).unwrap_err().contains("no cwd"));
}
