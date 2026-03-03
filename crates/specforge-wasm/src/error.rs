use std::fmt;
use std::path::PathBuf;

/// Errors that can occur during Wasm plugin operations.
#[derive(Debug, Clone)]
pub enum WasmError {
    /// The `manifest.json` file was not found at the expected path.
    ManifestNotFound {
        path: PathBuf,
    },
    /// The `manifest.json` could not be parsed.
    ManifestParseError {
        path: PathBuf,
        message: String,
    },
    /// The `.wasm` binary referenced in the manifest was not found.
    WasmBinaryNotFound {
        path: PathBuf,
        manifest_package: String,
    },
    /// Failed to load the Wasm module (compilation or instantiation error).
    WasmLoadFailed {
        package: String,
        message: String,
    },
    /// A Wasm trap occurred during plugin execution.
    TrapCaught(WasmTrapInfo),
    /// Sandbox policy violation (file path, domain, memory, or fuel).
    SandboxViolation {
        package: String,
        message: String,
    },
    /// A peer dependency is not satisfied.
    PeerDependencyUnsatisfied {
        package: String,
        dependency: String,
        required: String,
        found: Option<String>,
    },
    /// A cycle was detected in plugin peer dependencies.
    CycleDetected {
        participants: Vec<String>,
    },
    /// A host function received invalid input from the plugin.
    HostFunctionError {
        package: String,
        function: String,
        message: String,
    },
    /// Plugin called a host function in an invalid lifecycle state.
    InvalidLifecycleCall {
        package: String,
        function: String,
        expected_state: String,
        actual_state: String,
    },
}

/// Details about a Wasm trap.
#[derive(Debug, Clone)]
pub struct WasmTrapInfo {
    pub package: String,
    pub function: String,
    pub message: String,
}

impl fmt::Display for WasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManifestNotFound { path } => {
                write!(f, "manifest not found: {}", path.display())
            }
            Self::ManifestParseError { path, message } => {
                write!(f, "invalid manifest {}: {message}", path.display())
            }
            Self::WasmBinaryNotFound {
                path,
                manifest_package,
            } => {
                write!(
                    f,
                    "wasm binary not found for `{manifest_package}`: {}",
                    path.display()
                )
            }
            Self::WasmLoadFailed { package, message } => {
                write!(f, "failed to load `{package}`: {message}")
            }
            Self::TrapCaught(info) => {
                write!(
                    f,
                    "wasm trap in `{}::{}`: {}",
                    info.package, info.function, info.message
                )
            }
            Self::SandboxViolation { package, message } => {
                write!(f, "sandbox violation in `{package}`: {message}")
            }
            Self::PeerDependencyUnsatisfied {
                package,
                dependency,
                required,
                found,
            } => {
                write!(
                    f,
                    "`{package}` requires `{dependency}` {required}, found {}",
                    found.as_deref().unwrap_or("(not installed)")
                )
            }
            Self::CycleDetected { participants } => {
                write!(
                    f,
                    "dependency cycle detected: {}",
                    participants.join(" → ")
                )
            }
            Self::HostFunctionError {
                package,
                function,
                message,
            } => {
                write!(
                    f,
                    "host function error in `{package}::{function}`: {message}"
                )
            }
            Self::InvalidLifecycleCall {
                package,
                function,
                expected_state,
                actual_state,
            } => {
                write!(
                    f,
                    "`{package}` called `{function}` in state `{actual_state}` (expected `{expected_state}`)"
                )
            }
        }
    }
}

impl std::error::Error for WasmError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_manifest_not_found() {
        let err = WasmError::ManifestNotFound {
            path: PathBuf::from("/plugins/test/manifest.json"),
        };
        assert!(err.to_string().contains("manifest not found"));
    }

    #[test]
    fn display_peer_dep_unsatisfied() {
        let err = WasmError::PeerDependencyUnsatisfied {
            package: "@specforge/hexagonal".to_string(),
            dependency: "@specforge/product".to_string(),
            required: ">=1.0.0".to_string(),
            found: Some("0.5.0".to_string()),
        };
        let s = err.to_string();
        assert!(s.contains("@specforge/hexagonal"));
        assert!(s.contains("@specforge/product"));
        assert!(s.contains(">=1.0.0"));
        assert!(s.contains("0.5.0"));
    }

    #[test]
    fn display_cycle_detected() {
        let err = WasmError::CycleDetected {
            participants: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        };
        assert!(err.to_string().contains("a → b → a"));
    }

    #[test]
    fn display_trap_caught() {
        let err = WasmError::TrapCaught(WasmTrapInfo {
            package: "test-plugin".to_string(),
            function: "validate".to_string(),
            message: "unreachable".to_string(),
        });
        assert!(err.to_string().contains("wasm trap"));
        assert!(err.to_string().contains("test-plugin::validate"));
    }

    #[test]
    fn display_sandbox_violation() {
        let err = WasmError::SandboxViolation {
            package: "evil-plugin".to_string(),
            message: "path escapes sandbox: ../../../etc/passwd".to_string(),
        };
        assert!(err.to_string().contains("sandbox violation"));
        assert!(err.to_string().contains("evil-plugin"));
    }
}
