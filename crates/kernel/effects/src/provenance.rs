//! Where a loaded extension's code came from (ADR 0335 D2, O6): the host vouches for a builtin, so a builtin's
//! approval is keyed by its name and survives an upgrade; a third-party one's by the digest of its blob.

use serde::{Deserialize, Serialize};
use specforge_package::Sha256;

/// A loaded extension's provenance, and the SHA-256 of the bytes it was loaded from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "origin", rename_all = "snake_case")]
pub enum Provenance {
    /// Embedded in this specforge.
    Builtin { digest: String },
    /// Installed from a registry, or a `.wasm` file entry.
    ThirdParty { digest: String },
}

impl Provenance {
    /// A builtin loaded from `bytes`.
    pub fn builtin(bytes: &[u8]) -> Self {
        Provenance::Builtin {
            digest: Sha256::of(bytes).to_string(),
        }
    }

    /// A third-party extension loaded from `bytes`.
    pub fn third_party(bytes: &[u8]) -> Self {
        Provenance::ThirdParty {
            digest: Sha256::of(bytes).to_string(),
        }
    }

    /// The SHA-256 of the loaded bytes, lowercase hex.
    pub fn digest(&self) -> &str {
        match self {
            Provenance::Builtin { digest } | Provenance::ThirdParty { digest } => digest,
        }
    }

    pub fn is_builtin(&self) -> bool {
        matches!(self, Provenance::Builtin { .. })
    }
}
