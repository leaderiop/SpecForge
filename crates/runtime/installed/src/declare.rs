//! What a binary declares, read the way every environment reads it.

use specforge_common::Diagnostic;
use specforge_protocol_types::ProtocolError;
use specforge_wasm::protocol::{Loaded, load_declaration};
use specforge_wasm::{Provenance, WasmRuntime};

use crate::LoadProblem;
use crate::refusal::{self, Subject};

/// The name a candidate is loaded under while it is read.
const CANDIDATE: &str = "__candidate";

/// What the binary `bytes` declare, loaded as every environment loads an
/// extension (under a candidate name, then unloaded): its declaration and
/// the warnings its load reports. E028 "not a loadable SpecForge extension:
/// …" when it doesn't load or answer its declaration. Its warnings name what
/// it declares, not the candidate name; E088 when it speaks another protocol
/// major or a newer minor, or imports a host port this specforge lacks
/// (ADR 0326). `add`, `publish` and `init` read a candidate through it.
pub fn declaration_of(bytes: &[u8], runtime: &dyn WasmRuntime) -> Result<Loaded, Diagnostic> {
    let refused = |problem: LoadProblem, declared: Option<String>| {
        refusal::diagnostic(&problem, &Subject::Candidate { declared })
            .expect("a refused load has its diagnostic")
    };
    runtime
        .load(CANDIDATE, bytes, Provenance::third_party(bytes))
        .map_err(|error| refused(refusal::of_load(error), None))?;
    let loaded = load_declaration(runtime, CANDIDATE);
    runtime.unload(CANDIDATE);
    loaded.map_err(|error| {
        let declared = match &error {
            ProtocolError::NewerVersion { extension, .. }
            | ProtocolError::OlderVersion { extension, .. }
            | ProtocolError::UnbackedImport { extension, .. } => Some(extension.clone()),
            _ => None,
        };
        refused(refusal::of_protocol(error), declared)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta, SandboxPolicy};
    use specforge_protocol_types::DeclaredCategory;
    use specforge_wasm::testing::InProcessRuntime;

    #[test]
    fn a_component_declares_what_its_handshake_says() {
        let bytes = b"\0asm a candidate";
        let runtime = InProcessRuntime::new().binary(bytes, || {
            ContributionsBuilder::new(ExtensionMeta::new("@acme/candidate", "2.1.0"))
        });

        let loaded = declaration_of(bytes, &runtime).unwrap();

        assert_eq!(loaded.declaration.name(), "@acme/candidate");
        assert_eq!(loaded.declaration.version(), "2.1.0");
        assert!(
            !runtime.unload(CANDIDATE),
            "the candidate is not left loaded"
        );
    }

    #[test]
    fn an_unbacked_import_is_e089_naming_the_extension_it_declares() {
        let bytes = b"\0asm a candidate";
        let runtime = InProcessRuntime::new()
            .binary(bytes, || {
                ContributionsBuilder::new(ExtensionMeta::new("@acme/candidate", "2.1.0"))
            })
            .importing(CANDIDATE, &["specforge:host/git@0.1.0"]);

        let refused = declaration_of(bytes, &runtime).unwrap_err();

        assert_eq!(refused.code, "E089");
        assert!(
            refused.message.contains("'@acme/candidate'"),
            "{}",
            refused.message
        );
        assert!(
            refused.message.contains("specforge:host/git@0.1.0"),
            "{}",
            refused.message
        );
    }

    #[test]
    fn a_candidates_warnings_name_the_extension_it_declares() {
        let bytes = b"\0asm a candidate that asks";
        let runtime = InProcessRuntime::new().binary(bytes, || {
            let mut meta = ExtensionMeta::new("@acme/candidate", "2.1.0");
            meta.sandbox_policy = Some(SandboxPolicy {
                max_memory_mb: Some(4096),
                max_execution_ms: None,
            });
            let mut b = ContributionsBuilder::new(meta);
            b.raw_category(
                DeclaredCategory::Entities,
                serde_json::json!([{ "name": "x", "testabel": true }]),
            );
            b
        });

        let loaded = declaration_of(bytes, &runtime).unwrap();

        let codes: Vec<&str> = loaded.warnings.iter().map(|w| w.code.as_str()).collect();
        assert_eq!(codes, ["W153", "W138"]);
        for warning in &loaded.warnings {
            assert!(
                warning.message.starts_with("extension '@acme/candidate': "),
                "{}",
                warning.message
            );
        }
    }

    #[test]
    fn a_candidate_that_does_not_load_names_no_load_key() {
        let error = declaration_of(b"not wasm", &InProcessRuntime::new()).unwrap_err();

        assert_eq!(error.code, "E028");
        assert!(!error.message.contains(CANDIDATE), "{}", error.message);
    }

    #[test]
    fn a_candidate_of_a_newer_protocol_minor_is_e088() {
        let bytes = b"\0asm a newer candidate";
        let version = specforge_protocol_types::PROTOCOL_VERSION;
        let (major, rest) = version.split_once('.').unwrap();
        let minor: u64 = rest.split('.').next().unwrap().parse().unwrap();
        let newer = format!("{major}.{}.0", minor + 1);
        let handshake = serde_json::json!({
            "protocol_version": newer, "name": "@acme/candidate", "version": "2.1.0",
            "peer_dependencies": [], "sandbox_policy": null,
        });
        let runtime = InProcessRuntime::new()
            .binary(bytes, || {
                ContributionsBuilder::new(ExtensionMeta::new("@acme/candidate", "2.1.0"))
            })
            .answer_raw(
                CANDIDATE,
                "__handshake",
                specforge_wasm::WasmCallResult::Ok(serde_json::to_vec(&handshake).unwrap()),
            );

        let error = declaration_of(bytes, &runtime).unwrap_err();

        assert_eq!(error.code, "E088");
        assert_eq!(
            error.message,
            format!(
                "extension '@acme/candidate' speaks protocol {newer}, newer than the {version} this \
                 specforge speaks: update specforge"
            )
        );
    }

    #[test]
    fn a_candidate_importing_a_host_port_this_specforge_lacks_is_e088() {
        let bytes = b"\0asm a newer port";
        let port = "specforge:host/http@9.9.0".to_string();
        let runtime = InProcessRuntime::new()
            .refusing(bytes, specforge_wasm::LoadError::UnknownPorts(vec![port]));

        let error = declaration_of(bytes, &runtime).unwrap_err();

        assert_eq!(error.code, "E088");
        assert!(
            error.message.starts_with(
                "not a loadable SpecForge extension: it imports the host port specforge:host/http@9.9.0"
            ),
            "{}",
            error.message
        );
        assert!(
            error
                .suggestion
                .as_deref()
                .unwrap()
                .starts_with("update specforge"),
            "{error:?}"
        );
    }

    #[test]
    fn a_candidate_of_a_newer_protocol_major_is_e088() {
        let bytes = b"\0asm a newer major";
        let major: u64 = specforge_protocol_types::PROTOCOL_VERSION
            .split('.')
            .next()
            .and_then(|major| major.parse().ok())
            .unwrap();
        let handshake = serde_json::json!({
            "protocol_version": format!("{}.0.0", major + 1), "name": "@acme/candidate",
            "version": "2.1.0", "peer_dependencies": [], "sandbox_policy": null,
        });
        let runtime = InProcessRuntime::new()
            .binary(bytes, || {
                ContributionsBuilder::new(ExtensionMeta::new("@acme/candidate", "2.1.0"))
            })
            .answer_raw(
                CANDIDATE,
                "__handshake",
                specforge_wasm::WasmCallResult::Ok(serde_json::to_vec(&handshake).unwrap()),
            );

        let error = declaration_of(bytes, &runtime).unwrap_err();

        assert_eq!(error.code, "E088");
    }

    #[test]
    fn bytes_that_are_not_a_component_are_e028() {
        let runtime = InProcessRuntime::new();

        let error = declaration_of(b"not wasm", &runtime).unwrap_err();

        assert_eq!(error.code, "E028");
        assert!(
            error
                .message
                .starts_with("not a loadable SpecForge extension:"),
            "{error:?}"
        );
        assert!(
            error
                .suggestion
                .as_deref()
                .unwrap()
                .contains("specforge-extension-sdk")
        );
    }
}
