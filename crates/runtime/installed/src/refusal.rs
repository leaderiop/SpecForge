//! A refused load turned into its diagnostic, in one place for the candidate path (`add`, `init`, `publish`) and
//! the entry path (every environment load): what the runtime or the handshake refused, for whom.

use std::path::Path;

use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::{PROTOCOL_VERSION, ProtocolError};
use specforge_wasm::LoadError;
use specforge_wasm::runtime::{foreign, listed, unprovided};

use crate::LoadProblem;

/// The remedy of an E028 about a binary: it is not what the SDK builds.
const REBUILD: &str = "build it with specforge-extension-sdk for wasm32-wasip2";

/// What a problem is about, for its message.
pub(crate) enum Subject<'a> {
    /// An extension named by an entry.
    Named(&'a str),
    /// A `.wasm` file entry: the entry as written (trimmed), the file it
    /// resolves to, the path as written and the name written before `=`.
    File {
        key: &'a str,
        file: &'a Path,
        path: &'a str,
    },
    /// A binary `add`, `init` or `publish` reads before it is installed: the name its handshake declares, when it
    /// answered one.
    Candidate { declared: Option<String> },
}

impl Subject<'_> {
    /// The `code` about the subject for a problem the per-entry step finds, in
    /// the same words for every kind of subject: `what` says what failed.
    pub(crate) fn says(&self, code: specforge_common::Code, what: String) -> Diagnostic {
        match self {
            Subject::Named(name) => Diagnostic::new(code, format!("extension '{name}' {what}")),
            Subject::File { key, file, .. } => Diagnostic::new(
                code,
                format!(
                    "extension entry '{key}' in specforge.json: {} {what}",
                    file.display()
                ),
            )
            .with_suggestion(REBUILD),
            Subject::Candidate { declared } => {
                let prefix = match declared {
                    Some(name) => format!("extension '{name}' "),
                    None => "not a loadable SpecForge extension: it ".to_string(),
                };
                Diagnostic::new(code, format!("{prefix}{what}")).with_suggestion(REBUILD)
            }
        }
    }
}

/// The problem a runtime's refusal is.
pub(crate) fn of_load(error: LoadError) -> LoadProblem {
    match error {
        LoadError::Unloadable(reason) => LoadProblem::NotAComponent { reason },
        LoadError::UnknownPorts(ports) => LoadProblem::UnknownPorts {
            ports,
            host: PROTOCOL_VERSION.to_string(),
        },
        LoadError::ForeignImports(interfaces) => LoadProblem::ForeignImports { interfaces },
    }
}

/// The problem a declaration that does not load is.
pub(crate) fn of_protocol(error: ProtocolError) -> LoadProblem {
    match error {
        ProtocolError::NewerVersion {
            host_version,
            extension_version,
            ..
        } => LoadProblem::NewerProtocol {
            declared: extension_version,
            host: host_version,
        },
        ProtocolError::OlderVersion {
            host_version,
            extension_version,
            ..
        } => LoadProblem::OlderProtocol {
            declared: extension_version,
            host: host_version,
        },
        error => LoadProblem::NoDeclaration {
            reason: error.to_string(),
        },
    }
}

/// The one diagnostic of a refused load for `subject`; `None` for a problem that is no refused load.
pub(crate) fn diagnostic(problem: &LoadProblem, subject: &Subject) -> Option<Diagnostic> {
    Some(match (problem, subject) {
        (
            LoadProblem::NotAComponent { reason } | LoadProblem::NoDeclaration { reason },
            Subject::Candidate { .. },
        ) => Diagnostic::new(
            codes::E028,
            format!("not a loadable SpecForge extension: {reason}"),
        )
        .with_suggestion(REBUILD),
        (LoadProblem::NotAComponent { reason }, _) => subject.says(
            codes::E028,
            format!("does not load as an extension component: {reason}"),
        ),
        (LoadProblem::NoDeclaration { reason }, _) => subject.says(
            codes::E028,
            format!("has no declaration the host can read: {reason}"),
        ),
        (
            LoadProblem::NewerProtocol { declared, host },
            Subject::Candidate {
                declared: Some(name),
            },
        ) => Diagnostic::new(
            codes::E088,
            format!(
                "extension '{name}' speaks protocol {declared}, newer than the {host} this specforge speaks: \
                 update specforge"
            ),
        ),
        (LoadProblem::NewerProtocol { declared, host }, _) => subject
            .says(
                codes::E088,
                format!(
                    "speaks extension protocol {declared}, newer than the {host} this specforge speaks, \
                     so it is not loaded"
                ),
            )
            .with_suggestion(format!(
                "update specforge to a release that speaks protocol {declared} (`specforge --version` \
                 names it), or install a version of the extension built for protocol {host}"
            )),
        (LoadProblem::OlderProtocol { declared, host }, _) => subject
            .says(
                codes::E088,
                format!(
                    "speaks extension protocol {declared}, older than the {host} this specforge speaks, so it \
                     is not loaded"
                ),
            )
            .with_suggestion(format!(
                "install a version of the extension built for protocol {}.x, or rebuild it with a \
                 specforge-extension-sdk of that protocol",
                semver::Version::parse(host).map_or_else(|_| host.clone(), |host| host.major.to_string())
            )),
        (LoadProblem::UnknownPorts { ports, host }, _) => subject
            .says(
                codes::E088,
                format!(
                    "{}, so it is not loaded",
                    unprovided(ports, &format!(" (extension protocol {host})"))
                ),
            )
            .with_suggestion(format!(
                "update specforge to a release that provides {} (`specforge --version` names the protocol a \
                 binary speaks), or install a version of the extension built for protocol {host}",
                listed(ports)
            )),
        (LoadProblem::ForeignImports { interfaces }, _) => subject.says(
            codes::E028,
            format!("{}, so no specforge loads it", foreign(interfaces)),
        ),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_candidate_and_an_entry_refused_alike_read_alike() {
        let problem = LoadProblem::NotAComponent {
            reason: "failed to compile component: bad magic".to_string(),
        };
        let file = Path::new("/p/x.wasm");
        let candidate = diagnostic(&problem, &Subject::Candidate { declared: None }).unwrap();
        let entry = diagnostic(&problem, &Subject::Named("@acme/x")).unwrap();
        let file_entry = diagnostic(
            &problem,
            &Subject::File {
                key: "x.wasm",
                file,
                path: "x.wasm",
            },
        )
        .unwrap();

        for refused in [&entry, &file_entry] {
            assert_eq!(refused.code, candidate.code);
            assert!(
                refused
                    .message
                    .ends_with("failed to compile component: bad magic"),
                "{refused:?}"
            );
        }
        assert!(
            candidate
                .message
                .ends_with("failed to compile component: bad magic")
        );
        assert_eq!(file_entry.suggestion, candidate.suggestion);
        assert_eq!(candidate.suggestion.as_deref(), Some(REBUILD));
    }
}
