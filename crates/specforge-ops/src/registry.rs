//! Which registries a project uses.
//!
//! SpecForge ships no registry (ADR 0004 N1): the only registries are the
//! ones the project's `specforge.json` lists under `registries`. With none,
//! a registry operation fails with E063 before it makes any network call.

use crate::OpError;
use crate::config::CONFIG_FILE;
use specforge_registry::{RegistryConfig, parse_registries_from_config};
use std::path::Path;

/// The diagnostic a registry operation reports when no registry is
/// configured.
pub const NO_REGISTRY: &str = "E063";

/// How to configure a registry, for E063's suggestion.
pub const CONFIGURE_HINT: &str = "add a \"registries\" array to specforge.json, e.g. \
     \"registries\": [{\"alias\": \"main\", \"url\": \"<registry URL>\", \"default_registry\": true}]";

/// The registries the project at `root` configures, in declaration order.
///
/// `operation` names the command for the message (`add`, `search`, ...).
/// No `specforge.json`, one that can't be read, or one without a
/// `registries` entry all fail with E063.
pub fn configured(root: &Path, operation: &str) -> Result<Vec<RegistryConfig>, OpError> {
    let registries = std::fs::read_to_string(root.join(CONFIG_FILE))
        .map(|content| parse_registries_from_config(&content).0)
        .unwrap_or_default();
    if registries.is_empty() {
        return Err(no_registry(operation));
    }
    Ok(registries)
}

/// E063 for `operation`.
pub fn no_registry(operation: &str) -> OpError {
    OpError::new(
        NO_REGISTRY,
        format!(
            "no registry configured: `{operation}` needs one, and SpecForge has no built-in registry"
        ),
    )
    .with_suggestion(CONFIGURE_HINT)
}
