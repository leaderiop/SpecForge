//! The one loader of an extension's declaration.
//!
//! Everything the host knows about an extension it reads here, once per
//! environment load: the handshake (its protocol major checked, its
//! sandbox deadline applied), then every category of
//! [`DECLARED_CATEGORIES`], whatever its contribution flags say. Nothing
//! describes a category again outside this load.

use specforge_common::{Diagnostic, Severity};
use specforge_protocol_types::{ExtensionDeclaration, UnknownKey};

use super::host::ProtocolHost;
#[cfg(doc)]
use specforge_protocol_types::DECLARED_CATEGORIES;

use super::error::ProtocolError;
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
/// describe category, unconditionally. A handshake or category that fails,
/// or does not parse, fails the load, naming what failed.
pub fn load_declaration(
    runtime: &dyn WasmRuntime,
    extension: &str,
) -> Result<Loaded, ProtocolError> {
    let host = ProtocolHost::new(runtime);
    let handshake = host.handshake(extension)?;
    host.validate_protocol_version(&handshake)?;
    let mut warnings = Vec::new();
    let declaration = ExtensionDeclaration::from_wire(
        handshake,
        |category| host.describe(extension, category),
        |key| warnings.push(unknown_key(extension, key)),
    )?;
    Ok(Loaded {
        declaration,
        warnings,
    })
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
