//! The one loader of an extension's declaration.
//!
//! Everything the host knows about an extension it reads here, once per
//! environment load: the handshake (its protocol major checked, its
//! sandbox deadline applied), then every category of
//! [`DECLARED_CATEGORIES`], whatever its contribution flags say. Nothing
//! describes a category again outside this load.

use specforge_common::{Diagnostic, Severity};
#[cfg(doc)]
use specforge_protocol_types::DECLARED_CATEGORIES;
use specforge_protocol_types::{
    ExtensionDeclaration, HandshakeResponse, PROTOCOL_VERSION, ProtocolError, UnknownKey,
};

use crate::calls::{CallError, CallFailure, ExtensionCalls};
use crate::runtime::WasmRuntime;

/// A loaded declaration, with what its load found worth a warning.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub declaration: ExtensionDeclaration,
    /// W138: describe item keys the protocol does not define, in the order
    /// the categories were read.
    pub warnings: Vec<Diagnostic>,
}

/// Load `extension`'s declaration through `runtime`: the handshake
/// (protocol major checked, sandbox deadline applied), then every declared
/// describe category, unconditionally, each one call of
/// [`ExtensionCalls`]. A handshake or category that fails, or does not
/// parse, fails the load, naming what failed.
pub fn load_declaration(
    runtime: &dyn WasmRuntime,
    extension: &str,
) -> Result<Loaded, ProtocolError> {
    let calls = ExtensionCalls::new(runtime);
    let handshake = calls.handshake(extension).map_err(|error| {
        match error.failure {
            // A handshake that is not one is a deserialization error.
            CallFailure::Malformed { reason, .. } => ProtocolError::DeserializationError(reason),
            _ => ProtocolError::HandshakeFailed(reason(&error)),
        }
    })?;
    check_protocol_version(&handshake)?;
    let mut warnings = Vec::new();
    let declaration = ExtensionDeclaration::from_wire(
        handshake,
        |category| {
            calls
                .describe(extension, category)
                .map_err(|error| ProtocolError::DescribeFailed {
                    category: category.to_string(),
                    reason: reason(&error),
                })
        },
        |key| warnings.push(unknown_key(extension, key)),
    )?;
    Ok(Loaded {
        declaration,
        warnings,
    })
}

/// Why a declaration call failed, as the load error states it.
fn reason(error: &CallError) -> String {
    match &error.failure {
        CallFailure::NotLoaded => "the extension is not loaded".to_string(),
        CallFailure::Trapped { kind, message } => format!("{kind}: {message}"),
        CallFailure::Malformed { reason, .. } | CallFailure::Unencodable { reason } => {
            reason.clone()
        }
    }
}

/// The extension's protocol must share the host's major version (semver).
fn check_protocol_version(response: &HandshakeResponse) -> Result<(), ProtocolError> {
    let incompatible = || ProtocolError::IncompatibleVersion {
        host_version: PROTOCOL_VERSION.to_string(),
        extension_version: response.protocol_version.clone(),
    };
    let host = semver::Version::parse(PROTOCOL_VERSION).map_err(|e| {
        ProtocolError::HandshakeFailed(format!(
            "invalid host protocol version '{PROTOCOL_VERSION}': {e}"
        ))
    })?;
    let extension =
        semver::Version::parse(&response.protocol_version).map_err(|_| incompatible())?;
    if host.major != extension.major {
        return Err(incompatible());
    }
    Ok(())
}

/// W138: a describe item key the protocol does not define, ignored.
fn unknown_key(extension: &str, key: UnknownKey) -> Diagnostic {
    let UnknownKey {
        category,
        item,
        key,
    } = key;
    Diagnostic {
        code: "W138".to_string(),
        severity: Severity::Warning,
        message: format!(
            "extension '{extension}': describe '{category}' item '{item}' has the key '{key}', \
             which the protocol does not define; it is ignored"
        ),
        span: None,
        suggestion: Some(
            "check the key's spelling against the protocol's descriptors \
             (docs/extension-protocol.md), or update specforge if the extension needs a newer host"
                .to_string(),
        ),
        data: None,
    }
}
