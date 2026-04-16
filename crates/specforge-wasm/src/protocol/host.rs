use specforge_common::Diagnostic;

use crate::runtime::{WasmCallResult, WasmRuntime};

use super::error::ProtocolError;
use super::types::*;
use super::PROTOCOL_VERSION;
use super::SUPPORTED_CATEGORIES;

/// Host-side protocol handler that loads extensions via `__handshake` and `__describe` Wasm exports.
/// Wraps a `WasmRuntime` and provides typed protocol operations.
pub struct ProtocolHost<'a> {
    runtime: &'a dyn WasmRuntime,
}

impl<'a> ProtocolHost<'a> {
    pub fn new(runtime: &'a dyn WasmRuntime) -> Self {
        Self { runtime }
    }

    /// Perform the protocol handshake with an extension.
    /// Calls the `__handshake` export and parses the response.
    pub fn handshake(&self, extension_name: &str) -> Result<HandshakeResponse, ProtocolError> {
        let request = HandshakeRequest {
            host_version: PROTOCOL_VERSION.to_string(),
            supported_categories: SUPPORTED_CATEGORIES.iter().map(|s| s.to_string()).collect(),
        };
        let request_json =
            serde_json::to_vec(&request).map_err(|e| ProtocolError::HandshakeFailed(e.to_string()))?;

        match self.runtime.call_export(extension_name, "__handshake", &request_json) {
            WasmCallResult::Ok(response_bytes) => {
                let response: HandshakeResponse =
                    serde_json::from_slice(&response_bytes).map_err(ProtocolError::from)?;
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
    pub fn validate_protocol_version(
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
    pub fn describe(
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
        let request_json = serde_json::to_vec(&request)
            .map_err(|e| ProtocolError::DescribeFailed {
                category: category.to_string(),
                reason: e.to_string(),
            })?;

        match self.runtime.call_export(extension_name, "__describe", &request_json) {
            WasmCallResult::Ok(response_bytes) => {
                let response: DescribeResponse =
                    serde_json::from_slice(&response_bytes).map_err(ProtocolError::from)?;
                Ok(response)
            }
            WasmCallResult::Trap(trap) => Err(ProtocolError::DescribeFailed {
                category: category.to_string(),
                reason: format!("{}: {}", trap.kind, trap.message),
            }),
        }
    }

    /// Request all describe categories enabled by the extension's contribution flags.
    /// Returns an `ExtensionDescriptions` with all typed descriptors.
    pub fn describe_all(
        &self,
        extension_name: &str,
        flags: &ContributionFlags,
    ) -> Result<ExtensionDescriptions, ProtocolError> {
        let mut descs = ExtensionDescriptions::default();

        // Categories gated by contribution flags
        if flags.entities {
            descs.entity_kinds = self.describe_typed(extension_name, "entities")?;
            descs.edge_types = self.describe_typed(extension_name, "edges")?;
            descs.fields = self.describe_typed(extension_name, "fields")?;
            descs.shared_fields = self.describe_typed(extension_name, "shared_fields")?;
            descs.enhancements = self.describe_typed(extension_name, "enhancements")?;
        }

        if flags.validators {
            descs.validation_rules = self.describe_typed(extension_name, "validation_rules")?;
        }

        if flags.grammars {
            descs.grammars = self.describe_typed(extension_name, "grammars")?;
        }

        if flags.body_parsers {
            descs.body_parsers = self.describe_typed(extension_name, "body_parsers")?;
        }

        if flags.collectors {
            descs.collectors = self.describe_typed(extension_name, "collectors")?;
        }

        // Always request surfaces, passes, and feature_flags if extension declares any
        if flags.entities || flags.validators || flags.collectors {
            if let Ok(surfaces_vec) = self.describe_typed::<SurfaceDescriptor>(extension_name, "surfaces") {
                descs.surfaces = surfaces_vec.into_iter().next();
            }
            descs.passes = self.describe_typed(extension_name, "passes")?;
            descs.feature_flags = self.describe_typed(extension_name, "feature_flags")?;
        }

        Ok(descs)
    }

    /// Validate that all required peer dependencies are satisfied.
    pub fn validate_peer_dependencies(
        &self,
        response: &HandshakeResponse,
        loaded_extensions: &[&str],
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for dep in &response.peer_dependencies {
            if dep.optional {
                continue;
            }
            if !loaded_extensions.contains(&dep.name.as_str()) {
                diagnostics.push(Diagnostic {
                    code: "E028".to_string(),
                    severity: specforge_common::Severity::Error,
                    message: format!(
                        "extension '{}' requires peer dependency '{}' ({}), but it is not loaded",
                        response.name, dep.name, dep.version
                    ),
                    span: None,
                    suggestion: Some(format!(
                        "add '{}' to the extensions list in specforge.json",
                        dep.name
                    )),
                });
            }
        }
        diagnostics
    }

    /// Helper: describe a category and parse items into typed Vec<T>.
    fn describe_typed<T: serde::de::DeserializeOwned>(
        &self,
        extension_name: &str,
        category: &str,
    ) -> Result<Vec<T>, ProtocolError> {
        let response = self.describe(extension_name, category)?;
        response.parse_items()
    }
}

/// Aggregated describe responses for one extension across all categories.
#[derive(Debug, Clone, Default)]
pub struct ExtensionDescriptions {
    pub entity_kinds: Vec<EntityKindDescriptor>,
    pub edge_types: Vec<EdgeTypeDescriptor>,
    pub fields: Vec<FieldDescriptor>,
    pub shared_fields: Vec<SharedFieldDescriptor>,
    pub enhancements: Vec<EntityEnhancementDescriptor>,
    pub validation_rules: Vec<ValidationRuleDescriptor>,
    pub surfaces: Option<SurfaceDescriptor>,
    pub grammars: Vec<GrammarDescriptor>,
    pub body_parsers: Vec<BodyParserDescriptor>,
    pub collectors: Vec<CollectorDescriptor>,
    pub passes: Vec<CompilerPassDescriptor>,
    pub feature_flags: Vec<FeatureFlagDescriptor>,
}

/// A fully loaded protocol extension: handshake metadata + all descriptions.
#[derive(Debug, Clone)]
pub struct ProtocolExtension {
    pub name: String,
    pub version: String,
    pub handshake: HandshakeResponse,
    pub descriptions: ExtensionDescriptions,
}

/// Load an extension via the protocol: handshake, validate version, describe all categories.
pub fn load_protocol_extension(
    host: &ProtocolHost,
    extension_name: &str,
) -> Result<ProtocolExtension, ProtocolError> {
    let handshake = host.handshake(extension_name)?;
    host.validate_protocol_version(&handshake)?;
    let descriptions = host.describe_all(extension_name, &handshake.contribution_flags)?;

    Ok(ProtocolExtension {
        name: handshake.name.clone(),
        version: handshake.version.clone(),
        handshake,
        descriptions,
    })
}
