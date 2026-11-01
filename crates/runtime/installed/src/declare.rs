//! What a binary declares, read the way every environment reads it.

use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::ProtocolError;
use specforge_wasm::WasmRuntime;
use specforge_wasm::protocol::{Loaded, load_declaration};

/// The name a candidate is loaded under while it is read.
const CANDIDATE: &str = "__candidate";

/// What the binary `bytes` declare, loaded as every environment loads an
/// extension (under a candidate name, then unloaded): its declaration and
/// the warnings its load reports. E028 "not a loadable SpecForge extension:
/// …" when it doesn't load or answer its declaration. Its warnings name what
/// it declares, not the candidate name; E088 when its protocol minor is newer
/// than the host's (ADR 0326). `add`, `publish` and `init` read a candidate
/// through it.
pub fn declaration_of(bytes: &[u8], runtime: &dyn WasmRuntime) -> Result<Loaded, Diagnostic> {
    let invalid = |why: String| {
        Diagnostic::new(
            codes::E028,
            format!("not a loadable SpecForge extension: {why}"),
        )
        .with_suggestion("build it with specforge-extension-sdk for wasm32-wasip2".to_string())
    };
    runtime.load(CANDIDATE, bytes).map_err(invalid)?;
    let loaded = load_declaration(runtime, CANDIDATE);
    runtime.unload(CANDIDATE);
    loaded.map_err(|error| match error {
        // Its message names the extension, both versions and the remedy.
        ProtocolError::NewerVersion { .. } => Diagnostic::new(codes::E088, error.to_string()),
        error => invalid(error.to_string()),
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
        let handshake = serde_json::json!({
            "protocol_version": "1.15.0", "name": "@acme/candidate", "version": "2.1.0",
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
            "extension '@acme/candidate' speaks protocol 1.15.0, newer than the 1.14.0 this specforge \
             speaks: update specforge"
        );
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
