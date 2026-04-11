use crate::runtime::{ExtensionLifecycleState, LoadedModule, WasmTrapInfo};
use specforge_common::{Diagnostic, Severity};

/// Handle a Wasm trap: transition extension to Failed, produce diagnostic.
pub fn handle_wasm_trap(module: &mut LoadedModule, trap: &WasmTrapInfo) -> Diagnostic {
    module.state = ExtensionLifecycleState::Failed;
    Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!(
            "extension '{}': Wasm trap during {}() — {}: {}",
            module.extension_name, trap.export_name, trap.kind, trap.message
        ),
        span: None,
        suggestion: Some("check the extension for bugs or report to the extension author".to_string()),
    }
}

/// Check if a module should be skipped (is in Failed state).
pub fn should_skip_extension(module: &LoadedModule) -> bool {
    module.state == ExtensionLifecycleState::Failed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_module(name: &str) -> LoadedModule {
        LoadedModule {
            extension_name: name.to_string(),
            wasm_hash: "hash".to_string(),
            state: ExtensionLifecycleState::Initialized,
        }
    }

    // B:handle_wasm_trap — verify unit "catches trap during validate() export"
    #[test]
    fn test_catches_trap_during_validate() {
        let mut module = make_module("ext");
        let trap = WasmTrapInfo {
            kind: "unreachable".to_string(),
            message: "null pointer".to_string(),
            export_name: "validate".to_string(),
        };

        let diag = handle_wasm_trap(&mut module, &trap);
        assert_eq!(diag.code, "E028");
        assert!(diag.message.contains("validate()"));
        assert!(diag.message.contains("unreachable"));
    }

    // B:handle_wasm_trap — verify unit "catches trap during render() call"
    #[test]
    fn test_catches_trap_during_render() {
        let mut module = make_module("ext");
        let trap = WasmTrapInfo {
            kind: "out_of_bounds".to_string(),
            message: "memory access".to_string(),
            export_name: "render".to_string(),
        };

        let diag = handle_wasm_trap(&mut module, &trap);
        assert!(diag.message.contains("render()"));
    }

    // B:handle_wasm_trap — verify unit "extracts trap kind and message"
    #[test]
    fn test_extracts_trap_kind_and_message() {
        let mut module = make_module("ext");
        let trap = WasmTrapInfo {
            kind: "stack_overflow".to_string(),
            message: "recursion too deep".to_string(),
            export_name: "initialize".to_string(),
        };

        let diag = handle_wasm_trap(&mut module, &trap);
        assert!(diag.message.contains("stack_overflow"));
        assert!(diag.message.contains("recursion too deep"));
    }

    // B:handle_wasm_trap — verify unit "transitions extension to failed state"
    #[test]
    fn test_transitions_to_failed_state() {
        let mut module = make_module("ext");
        assert_eq!(module.state, ExtensionLifecycleState::Initialized);

        let trap = WasmTrapInfo {
            kind: "trap".to_string(),
            message: "error".to_string(),
            export_name: "validate".to_string(),
        };

        handle_wasm_trap(&mut module, &trap);
        assert_eq!(module.state, ExtensionLifecycleState::Failed);
    }

    // B:handle_wasm_trap — verify unit "remaining extensions continue after trap"
    #[test]
    fn test_remaining_extensions_continue_after_trap() {
        let mut trapped = make_module("bad-ext");
        let healthy = make_module("good-ext");

        let trap = WasmTrapInfo {
            kind: "trap".to_string(),
            message: "error".to_string(),
            export_name: "validate".to_string(),
        };

        handle_wasm_trap(&mut trapped, &trap);

        // Trapped module is failed
        assert!(should_skip_extension(&trapped));
        // Healthy module is unaffected
        assert!(!should_skip_extension(&healthy));
    }

    // B:handle_wasm_trap — verify contract "requires/ensures consistency for Wasm trap handling"
    #[test]
    fn test_handle_wasm_trap_contract() {
        // requires: trap_occurred
        let mut module = make_module("ext");
        let trap = WasmTrapInfo {
            kind: "unreachable".to_string(),
            message: "crash".to_string(),
            export_name: "validate".to_string(),
        };

        // ensures: wasm_trap_caught_emitted — diagnostic produced
        let diag = handle_wasm_trap(&mut module, &trap);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.code, "E028");

        // ensures: lifecycle_transitioned — extension is failed
        assert_eq!(module.state, ExtensionLifecycleState::Failed);

        // ensures: trapped_extension_skipped
        assert!(should_skip_extension(&module));

        // ensures: remaining_extensions_continue
        let other = make_module("other");
        assert!(!should_skip_extension(&other));
    }
}
