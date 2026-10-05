//! Sandbox policy core (C7-04). Every knob is **deny by default**: file
//! system access and network access are off unless the extension's
//! handshake explicitly enables them, and an empty allowlist permits
//! nothing. This layer is enforced by the component host-import surface
//! (planned) and by the package registry, which refuses a declaration that
//! asks for network access; no shipped guest has host access — pure-compute
//! components never touch it.
use specforge_protocol_types::SandboxPolicy;

/// Built-in sandbox defaults, applied to an extension whose handshake
/// declares no `sandbox_policy` (its `max_execution_ms` is the default call
/// deadline).
pub fn default_sandbox_policy() -> SandboxPolicy {
    SandboxPolicy {
        max_memory_mb: Some(64),
        max_execution_ms: Some(30_000),
        allowed_domains: vec![],
        allowed_paths: vec![],
        allowed_output_extensions: vec![
            ".json".into(),
            ".html".into(),
            ".csv".into(),
            ".svg".into(),
            ".dot".into(),
            ".xml".into(),
            ".txt".into(),
            ".pdf".into(),
        ],
        network_access: Some(false),
        file_system_access: Some(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_policy_denies_host_access() {
        let policy = default_sandbox_policy();
        assert_eq!(policy.max_memory_mb, Some(64));
        assert_eq!(policy.max_execution_ms, Some(30_000));
        assert_eq!(policy.network_access, Some(false));
        assert_eq!(policy.file_system_access, Some(false));
        assert!(policy.allowed_domains.is_empty());
        assert!(policy.allowed_paths.is_empty());
    }
}
