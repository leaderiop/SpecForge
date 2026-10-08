//! What a binary declares, read the way every environment reads it.

use specforge_common::{Diagnostic, codes};
use specforge_wasm::WasmRuntime;
use specforge_wasm::protocol::{Loaded, load_declaration};

use crate::module::Module;

/// The name a candidate is loaded under while it is read.
const CANDIDATE: &str = "__candidate";

/// What the binary `module` declares, loaded as every environment loads an
/// extension (under a candidate name, then unloaded): its declaration and
/// the warnings its load reports. E028 "not a loadable SpecForge extension:
/// …" when it doesn't load or answer its declaration. `add`, `publish` and
/// `init` read a candidate through it.
pub fn declaration_of(module: &Module, runtime: &dyn WasmRuntime) -> Result<Loaded, Diagnostic> {
    let invalid = |why: String| {
        Diagnostic::new(
            codes::E028,
            format!("not a loadable SpecForge extension: {why}"),
        )
        .with_suggestion("build it with specforge-extension-sdk for wasm32-wasip2".to_string())
    };
    runtime.load(CANDIDATE, module.bytes()).map_err(invalid)?;
    let loaded = load_declaration(runtime, CANDIDATE);
    runtime.unload(CANDIDATE);
    loaded.map_err(|error| invalid(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
    use specforge_wasm::testing::InProcessRuntime;

    #[test]
    fn a_component_declares_what_its_handshake_says() {
        let bytes = b"\0asm a candidate";
        let runtime = InProcessRuntime::new().binary(bytes, || {
            ContributionsBuilder::new(ExtensionMeta::new("@acme/candidate", "2.1.0"))
        });

        let loaded = declaration_of(&Module::new(bytes.to_vec()), &runtime).unwrap();

        assert_eq!(loaded.declaration.name(), "@acme/candidate");
        assert_eq!(loaded.declaration.version(), "2.1.0");
        assert!(
            !runtime.unload(CANDIDATE),
            "the candidate is not left loaded"
        );
    }

    #[test]
    fn bytes_that_are_not_a_component_are_e028() {
        let runtime = InProcessRuntime::new();

        let error = declaration_of(&Module::new(b"not wasm".to_vec()), &runtime).unwrap_err();

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
