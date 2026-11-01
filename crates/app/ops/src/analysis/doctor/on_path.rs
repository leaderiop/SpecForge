//! The `specforge` the hosts run: the hooks and the MCP server `specforge install` writes call `specforge` from
//! PATH, whichever binary that is. `specforge doctor` runs it with `--version` and says when it speaks an older
//! protocol than this specforge or an extension the project loads (W169, ADR 0326 D4). The user's machine, not
//! the project: the CLI reports it beside the project's findings, as it does credentials.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use specforge_common::process::run;
use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::{PROTOCOL_VERSION, protocol_minor};

use super::{About, Finding, FindingStatus};
use crate::view::ProjectView;

/// The finding code when no `specforge` is on PATH.
pub const NOT_ON_PATH: &str = "specforge_not_on_path";

/// How long `specforge --version` may take before it counts as no answer.
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// The first `specforge` on PATH and what its `--version` printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnPath {
    pub program: PathBuf,
    /// The first line its `--version` printed; none when it failed or did not answer in time.
    pub answer: Option<String>,
}

impl OnPath {
    /// The first `specforge` in the directories of `path` (a `PATH` value), run with `--version`; none when no
    /// directory holds one.
    pub fn find(path: Option<&OsStr>) -> Option<OnPath> {
        let name = format!("specforge{}", std::env::consts::EXE_SUFFIX);
        let program = std::env::split_paths(path?)
            .map(|dir| dir.join(&name))
            .find(|candidate| candidate.is_file())?;
        let answer = run(Command::new(&program).arg("--version"), PROBE_TIMEOUT)
            .ok()
            .filter(|ran| ran.status.success())
            .and_then(|ran| {
                ran.text()
                    .lines()
                    .next()
                    .map(|line| line.trim().to_string())
            });
        Some(OnPath { program, answer })
    }

    /// The protocol its version line names (`specforge 0.3.0 (9229143f5, protocol 1.14.0)`); none from a
    /// binary too old to print one.
    fn protocol(&self) -> Option<&str> {
        let (_, version) = self
            .answer
            .as_deref()?
            .strip_suffix(')')?
            .rsplit_once("protocol ")?;
        Some(version)
    }
}

/// What doctor says of `found`, the `specforge` on PATH, against this binary's protocol and the protocols of the
/// extensions `view` loaded: W169 when it speaks an older minor (or names none), an info finding when there is
/// no `specforge` on PATH, nothing when it is new enough.
pub fn finding(found: Option<&OnPath>, view: &ProjectView) -> Option<Finding> {
    let Some(found) = found else {
        return Some(Finding {
            about: About::Host { path: None },
            check: "no specforge on PATH: the hooks and the MCP server that `specforge install` writes run \
                    `specforge` from PATH"
                .into(),
            status: FindingStatus::Info,
            code: NOT_ON_PATH.into(),
            remediation: "put specforge on PATH before installing a host".into(),
        });
    };
    let spoken = found.protocol();
    let older = |needed: &str| spoken.is_none_or(|v| protocol_minor(v) < protocol_minor(needed));
    let extensions: Vec<String> = view
        .registries()
        .declarations()
        .iter()
        .filter(|d| older(&d.handshake.protocol_version))
        .map(|d| format!("{} ({})", d.name(), d.handshake.protocol_version))
        .collect();
    if !older(PROTOCOL_VERSION) && extensions.is_empty() {
        return None;
    }
    let path = found.program.display();
    let speaks = match (spoken, found.answer.as_deref()) {
        (Some(version), _) => format!("speaks extension protocol {version}"),
        (None, Some(answer)) => {
            format!("names no protocol (`specforge --version` printed `{answer}`)")
        }
        (None, None) => "does not answer `specforge --version`".to_string(),
    };
    // An old binary is older than every extension: three name the point, the count says the rest.
    let loads = match extensions.len() {
        0 => String::new(),
        n if n > 3 => format!(
            " and than {} and {} more extensions that the project loads",
            extensions[..3].join(", "),
            n - 3
        ),
        _ => format!(" and than {} that the project loads", extensions.join(", ")),
    };
    let diagnostic = Diagnostic::new(
        codes::W169,
        format!(
            "the specforge on PATH ({path}) {speaks}, older than this specforge's {PROTOCOL_VERSION}{loads}: \
             the hooks and the MCP server that `specforge install` writes run that binary"
        ),
    )
    .with_suggestion(format!(
        "update the specforge at {path} to one that speaks protocol {PROTOCOL_VERSION}, or put this \
         specforge first on PATH"
    ));
    Some(Finding::of(
        About::Host {
            path: Some(found.program.display().to_string()),
        },
        &diagnostic,
        view.registries().codes(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_protocol_types::{ExtensionDeclaration, HandshakeResponse};

    fn declaring(name: &str, protocol: &str) -> ExtensionDeclaration {
        ExtensionDeclaration {
            handshake: HandshakeResponse {
                name: name.into(),
                version: "1.0.0".into(),
                protocol_version: protocol.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn answering(answer: Option<&str>) -> OnPath {
        OnPath {
            program: PathBuf::from("/opt/bin/specforge"),
            answer: answer.map(str::to_string),
        }
    }

    fn judged(found: Option<&OnPath>) -> Option<Finding> {
        let project = specforge_project::testing::Assembled::over(
            specforge_registry::build_registries(vec![declaring("@acme/a", PROTOCOL_VERSION)]),
        );
        finding(found, &ProjectView::of(project.compiled()))
    }

    #[specforge_test_macros::test(
        behavior = "run_doctor_check",
        verify = "doctor warns when the specforge on PATH speaks an older protocol than the project needs"
    )]
    fn an_older_specforge_on_path_is_w169_naming_it_and_both_versions() {
        let finding = judged(Some(&answering(Some("specforge 0.2.0 (protocol 1.6.0)")))).unwrap();

        assert_eq!(finding.code, "W169");
        assert_eq!(finding.status, FindingStatus::Warn);
        assert_eq!(
            finding.check,
            format!(
                "the specforge on PATH (/opt/bin/specforge) speaks extension protocol 1.6.0, older than this \
                 specforge's {PROTOCOL_VERSION} and than @acme/a ({PROTOCOL_VERSION}) that the project loads: \
                 the hooks and the MCP server that `specforge install` writes run that binary"
            )
        );
        assert!(
            finding
                .remediation
                .starts_with("update the specforge at /opt/bin/specforge"),
            "{}",
            finding.remediation
        );
    }

    #[test]
    fn past_three_extensions_the_warning_counts_the_rest() {
        let declarations = ["@acme/a", "@acme/b", "@acme/c", "@acme/d", "@acme/e"]
            .map(|name| declaring(name, PROTOCOL_VERSION));
        let project = specforge_project::testing::Assembled::over(
            specforge_registry::build_registries(declarations.to_vec()),
        );
        let old = answering(Some("specforge 0.2.0 (protocol 1.6.0)"));

        let finding = finding(Some(&old), &ProjectView::of(project.compiled())).unwrap();

        assert!(
            finding.check.contains(&format!(
                "@acme/c ({PROTOCOL_VERSION}) and 2 more extensions that the project loads:"
            )),
            "{}",
            finding.check
        );
    }

    #[test]
    fn a_specforge_on_path_too_old_to_name_a_protocol_is_older() {
        let named = judged(Some(&answering(Some("specforge 0.3.0")))).unwrap();
        assert_eq!(named.code, "W169");
        assert!(
            named
                .check
                .contains("names no protocol (`specforge --version` printed `specforge 0.3.0`)"),
            "{}",
            named.check
        );

        let silent = judged(Some(&answering(None))).unwrap();
        assert!(
            silent
                .check
                .contains("does not answer `specforge --version`"),
            "{}",
            silent.check
        );
    }

    #[test]
    fn a_specforge_on_path_of_this_protocol_or_newer_says_nothing() {
        for line in [
            format!("specforge 0.3.0 (9229143f5, protocol {PROTOCOL_VERSION})"),
            "specforge 0.4.0 (protocol 1.99.0)".to_string(),
        ] {
            assert!(judged(Some(&answering(Some(&line)))).is_none(), "{line}");
        }
    }

    #[test]
    fn no_specforge_on_path_is_an_info_finding() {
        let finding = judged(None).unwrap();

        assert_eq!(finding.code, NOT_ON_PATH);
        assert_eq!(finding.status, FindingStatus::Info);
    }

    #[test]
    fn find_takes_the_first_specforge_on_path() {
        let empty = tempfile::tempdir().unwrap();
        let holding = tempfile::tempdir().unwrap();
        let program = holding
            .path()
            .join(format!("specforge{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&program, "not a program").unwrap();
        let path = std::env::join_paths([empty.path(), holding.path()]).unwrap();

        let found = OnPath::find(Some(&path)).unwrap();

        assert_eq!(found.program, program);
        assert_eq!(
            found.answer, None,
            "a file that does not run gives no answer"
        );
        assert_eq!(OnPath::find(Some(empty.path().as_os_str())), None);
        assert_eq!(OnPath::find(None), None);
    }
}
