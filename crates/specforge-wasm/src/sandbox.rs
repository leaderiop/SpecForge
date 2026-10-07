//! The sandbox an extension runs in (ADR 0037).
//!
//! A component is granted no capability, whatever it declares: no preopened
//! directory, environment, arguments or stdin, no socket and no name lookup
//! (the component runtime builds that context; `sandbox_probe` proves it).
//! What an extension declares in its handshake's `sandbox_policy` is limits
//! only — a call's wall-clock budget and its instance's linear memory — each
//! held to the host's ceiling. Reading the handshake applies them
//! ([`ExtensionCalls::handshake`](crate::ExtensionCalls::handshake) through
//! [`WasmRuntime::apply_limits`](crate::runtime::WasmRuntime::apply_limits)).

use serde_json::Value;
use specforge_common::Diagnostic;

/// What the host holds every call into one extension to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Wall-clock budget of one call, in milliseconds (epoch interruption;
    /// never interrupted before it has elapsed).
    pub execution_ms: u32,
    /// Ceiling of the instance's linear memory, in MiB: a growth past it
    /// traps the call.
    pub memory_mb: u32,
}

impl Limits {
    /// The host's ceiling: the limits of an extension that declares none,
    /// and what no declaration exceeds (an extension may tighten its
    /// sandbox, never widen it).
    pub const CEILING: Limits = Limits {
        execution_ms: 30_000,
        memory_mb: 512,
    };
}

/// One extension's sandbox: the limits it runs under, and what its
/// declaration asks for that the host does not honour.
#[derive(Debug, Clone, PartialEq)]
pub struct Sandbox {
    pub limits: Limits,
    /// W153, one per key, in the order the declaration writes them.
    pub unhonoured: Vec<Diagnostic>,
}

/// The two limits a `sandbox_policy` may declare.
const MAX_EXECUTION_MS: &str = "max_execution_ms";
const MAX_MEMORY_MB: &str = "max_memory_mb";

impl Sandbox {
    /// The sandbox of `extension`, whose handshake answered `sandbox_policy`
    /// as sent (`None` when absent or `null`). Each declared limit
    /// (`max_execution_ms`, `max_memory_mb`) is applied as declared, held to
    /// [`Limits::CEILING`]; an undeclared one is the ceiling.
    pub fn of(_extension: &str, sandbox_policy: Option<&Value>) -> Sandbox {
        let declared = |key: &str| {
            sandbox_policy
                .and_then(|policy| policy.get(key))
                .and_then(Value::as_u64)
        };
        let held = |declared: Option<u64>, ceiling: u32| {
            declared.map_or(ceiling, |value| {
                u32::try_from(value).map_or(ceiling, |value| value.min(ceiling))
            })
        };
        Sandbox {
            limits: Limits {
                execution_ms: held(declared(MAX_EXECUTION_MS), Limits::CEILING.execution_ms),
                memory_mb: held(declared(MAX_MEMORY_MB), Limits::CEILING.memory_mb),
            },
            unhonoured: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use specforge_protocol_types::SandboxPolicy;

    #[test]
    fn limits_are_held_to_the_ceiling() {
        let policy = |value: Value| Sandbox::of("@acme/x", Some(&value)).limits;
        assert_eq!(
            policy(json!({"max_execution_ms": 60000, "max_memory_mb": 1024})),
            Limits::CEILING
        );
        assert_eq!(
            policy(json!({"max_execution_ms": 50})),
            Limits {
                execution_ms: 50,
                memory_mb: 512
            }
        );
        assert_eq!(
            policy(json!({"max_execution_ms": null, "max_memory_mb": 256})),
            Limits {
                execution_ms: 30_000,
                memory_mb: 256
            }
        );
        assert_eq!(Sandbox::of("@acme/x", None).limits, Limits::CEILING);
    }

    /// A limit the policy type carries must be a limit the sandbox reads.
    #[test]
    fn limit_keys_are_the_policy_fields() {
        let written = serde_json::to_value(SandboxPolicy {
            max_execution_ms: Some(1),
            max_memory_mb: Some(1),
            ..Default::default()
        })
        .unwrap();
        for key in [MAX_EXECUTION_MS, MAX_MEMORY_MB] {
            assert_eq!(written.get(key), Some(&json!(1)), "{key}");
        }
    }
}
