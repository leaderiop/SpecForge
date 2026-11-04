//! The one loader of an extension's declaration.
//!
//! Everything the host knows about an extension it reads here, once per
//! environment load: the handshake (its protocol version checked, its
//! sandbox applied), then every category of
//! [`DeclaredCategory::ALL`], the only categories the protocol defines. Nothing
//! describes a category again outside this load.

use specforge_common::{Diagnostic, codes};
#[cfg(doc)]
use specforge_protocol_types::DeclaredCategory;
use specforge_protocol_types::{
    ExtensionDeclaration, HandshakeResponse, PROTOCOL_VERSION, ProtocolError, UnknownKey,
};

use crate::calls::{CallError, CallFailure, ExtensionCalls, Handshake};
use crate::runtime::WasmRuntime;
use crate::sandbox;

/// A loaded declaration, with what its load found worth a warning.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub declaration: ExtensionDeclaration,
    /// W153 (what its sandbox declaration asks for that the host does not
    /// honour), then W138 (describe item keys the protocol does not
    /// define), in the order the handshake and the categories were read,
    /// each naming the extension by the name its handshake declares.
    pub warnings: Vec<Diagnostic>,
    /// The protocol version the guest's handshake states; its major is the host's and its minor no newer (checked).
    pub protocol_version: String,
}

/// Load `extension`'s declaration through `runtime`: the handshake
/// (protocol version checked, sandbox applied), then every declared
/// describe category, unconditionally, each one call of
/// [`ExtensionCalls`]. A handshake or category that fails, or does not
/// parse, fails the load, naming what failed.
pub fn load_declaration(
    runtime: &dyn WasmRuntime,
    extension: &str,
) -> Result<Loaded, ProtocolError> {
    let calls = ExtensionCalls::new(runtime);
    let Handshake { response, sandbox } = calls.handshake(extension).map_err(|error| {
        match error.failure {
            // A handshake that is not one is a deserialization error.
            CallFailure::Malformed { reason, .. } => ProtocolError::DeserializationError(reason),
            _ => ProtocolError::HandshakeFailed(reason(&error)),
        }
    })?;
    check_protocol_version(PROTOCOL_VERSION, &response)?;
    let protocol_version = response.protocol_version.clone();
    // The limits the extension declares (the ceiling when it declares
    // none) hold every call after the handshake.
    runtime.apply_limits(extension, sandbox.limits);
    let mut warnings = sandbox.unhonoured;
    // The warnings name the extension as its handshake declares it, whatever key it is loaded under.
    let declared = response.name.clone();
    let declaration = ExtensionDeclaration::from_wire(
        response,
        |category| {
            calls
                .describe(extension, category)
                .map_err(|error| ProtocolError::DescribeFailed {
                    category: category.name().to_string(),
                    reason: reason(&error),
                })
        },
        |key| {
            warnings.push(
                sandbox::unhonoured_surface_key(&declared, &key)
                    .unwrap_or_else(|| unknown_key(&declared, key)),
            )
        },
    )?;
    let ports = unbacked(&runtime.host_imports(extension), &declaration);
    if !ports.is_empty() {
        return Err(ProtocolError::UnbackedImport {
            extension: declaration.name().to_string(),
            ports,
        });
    }
    Ok(Loaded {
        declaration,
        warnings,
        protocol_version,
    })
}

/// The ports among `imports` no capability of `declaration` backs (ADR 0335 D5), in import order. `types` holds only
/// what the four share: importing it alone asks for nothing.
fn unbacked(imports: &[String], declaration: &ExtensionDeclaration) -> Vec<String> {
    imports
        .iter()
        .filter(|name| {
            let port = name
                .strip_prefix("specforge:host/")
                .and_then(|rest| rest.split_once('@'))
                .map_or(name.as_str(), |(port, _)| port);
            port != "types"
                && !declaration
                    .capabilities
                    .iter()
                    .any(|capability| capability.port.port() == port)
        })
        .cloned()
        .collect()
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

/// The extension's protocol must be semver, share the `host`'s major and be of no newer minor: a newer guest may
/// declare what this host would drop, and an older major was built for a protocol this host no longer speaks
/// (ADR 0326 D1, D1b). Pure, so a test can name a host other than [`PROTOCOL_VERSION`] (ADR 0337).
pub fn check_protocol_version(
    host: &str,
    response: &HandshakeResponse,
) -> Result<(), ProtocolError> {
    let host_semver = semver::Version::parse(host).map_err(|e| {
        ProtocolError::HandshakeFailed(format!("invalid host protocol version '{host}': {e}"))
    })?;
    let extension = semver::Version::parse(&response.protocol_version).map_err(|_| {
        ProtocolError::HandshakeFailed(format!(
            "protocol_version '{}' is not a semver version",
            response.protocol_version
        ))
    })?;
    let (extension_name, host_version, extension_version) = (
        response.name.clone(),
        host.to_string(),
        response.protocol_version.clone(),
    );
    if extension.major < host_semver.major {
        return Err(ProtocolError::OlderVersion {
            extension: extension_name,
            host_version,
            extension_version,
        });
    }
    if extension.major > host_semver.major || extension.minor > host_semver.minor {
        return Err(ProtocolError::NewerVersion {
            extension: extension_name,
            host_version,
            extension_version,
        });
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
    Diagnostic::new(
        codes::W138,
        format!(
            "extension '{extension}': describe '{category}' item '{item}' has the key '{key}', \
             which the protocol does not define; it is ignored"
        ),
    )
    .with_suggestion(
        "check the key's spelling against the protocol's descriptors \
             (docs/extension-protocol.md), or update specforge if the extension needs a newer host"
            .to_string(),
    )
}
