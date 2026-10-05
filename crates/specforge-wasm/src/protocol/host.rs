use crate::runtime::{WasmCallResult, WasmRuntime};

use super::PROTOCOL_VERSION;
use super::SUPPORTED_CATEGORIES;
use specforge_protocol_types::ProtocolError;
use specforge_protocol_types::*;

/// The transport of the declaration load ([`super::load_declaration`]):
/// the `__handshake` and `__describe` calls over a `WasmRuntime`, each
/// answer parsed as its protocol type.
pub(crate) struct ProtocolHost<'a> {
    runtime: &'a dyn WasmRuntime,
}

impl<'a> ProtocolHost<'a> {
    pub(crate) fn new(runtime: &'a dyn WasmRuntime) -> Self {
        Self { runtime }
    }

    /// Perform the protocol handshake with an extension.
    /// Calls the `__handshake` export and parses the response.
    pub(crate) fn handshake(
        &self,
        extension_name: &str,
    ) -> Result<HandshakeResponse, ProtocolError> {
        let request = HandshakeRequest {
            host_version: PROTOCOL_VERSION.to_string(),
            supported_categories: SUPPORTED_CATEGORIES.iter().map(|s| s.to_string()).collect(),
        };
        let request_json = serde_json::to_vec(&request)
            .map_err(|e| ProtocolError::HandshakeFailed(e.to_string()))?;

        match self
            .runtime
            .call_export(extension_name, "__handshake", &request_json)
        {
            WasmCallResult::Ok(response_bytes) => {
                let response: HandshakeResponse =
                    serde_json::from_slice(&response_bytes).map_err(ProtocolError::from)?;
                // C7-10: the plugin's declared `max_execution_ms` now gates
                // every subsequent call into this extension (runtimes with
                // epoch interruption enforce it; others ignore the hint).
                if let Some(policy) = &response.sandbox_policy
                    && let Some(ms) = policy.max_execution_ms
                {
                    self.runtime
                        .set_execution_deadline_ms(extension_name, u64::from(ms));
                }
                Ok(response)
            }
            WasmCallResult::Trap(trap) => Err(ProtocolError::HandshakeFailed(format!(
                "{}: {}",
                trap.kind, trap.message
            ))),
        }
    }

    /// Validate that the extension's protocol version is compatible with the host.
    /// Uses semver major-version compatibility: same major version = compatible.
    pub(crate) fn validate_protocol_version(
        &self,
        response: &HandshakeResponse,
    ) -> Result<(), ProtocolError> {
        let host_ver = semver::Version::parse(PROTOCOL_VERSION).map_err(|e| {
            ProtocolError::HandshakeFailed(format!(
                "invalid host protocol version '{}': {}",
                PROTOCOL_VERSION, e
            ))
        })?;
        let ext_ver = semver::Version::parse(&response.protocol_version).map_err(|_| {
            ProtocolError::IncompatibleVersion {
                host_version: PROTOCOL_VERSION.to_string(),
                extension_version: response.protocol_version.clone(),
            }
        })?;

        if host_ver.major != ext_ver.major {
            return Err(ProtocolError::IncompatibleVersion {
                host_version: PROTOCOL_VERSION.to_string(),
                extension_version: response.protocol_version.clone(),
            });
        }
        Ok(())
    }

    /// Request a single describe category from an extension.
    /// Calls the `__describe` export with the category name.
    pub(crate) fn describe(
        &self,
        extension_name: &str,
        category: &str,
    ) -> Result<DescribeResponse, ProtocolError> {
        if !SUPPORTED_CATEGORIES.contains(&category) {
            return Err(ProtocolError::UnsupportedCategory(category.to_string()));
        }

        let request = DescribeRequest {
            category: category.to_string(),
        };
        let request_json =
            serde_json::to_vec(&request).map_err(|e| ProtocolError::DescribeFailed {
                category: category.to_string(),
                reason: e.to_string(),
            })?;

        match self
            .runtime
            .call_export(extension_name, "__describe", &request_json)
        {
            // An answer that is not a describe response names its category.
            WasmCallResult::Ok(response_bytes) => {
                serde_json::from_slice(&response_bytes).map_err(|e| ProtocolError::DescribeFailed {
                    category: category.to_string(),
                    reason: e.to_string(),
                })
            }
            WasmCallResult::Trap(trap) => Err(ProtocolError::DescribeFailed {
                category: category.to_string(),
                reason: format!("{}: {}", trap.kind, trap.message),
            }),
        }
    }
}
