//! What an extension package is called, which of its versions is asked for, which declaration is a
//! registry package, and what an update
//! moves a locked package to, and what the peers an extension declares require (ADR 0036, 0041,
//! 0050, 0200).
//!
//! The guest wire (`specforge_protocol_types`) carries these as text: a declaration's `name`, a
//! declared peer's `version` range. Every builtin blob is built from the wire, so the host's
//! reading of that text lives here, where no guest compiles it.
//!
//! Pure: no I/O and no diagnostics. Callers map its errors to their codes
//! (`specforge_common::package`: E072; `specforge_common::peers`: E073, E027).

mod declared;
mod digest;
mod name;
pub mod peers;
mod scope;
pub mod search;
pub mod upgrade;

pub use declared::{DeclaredPackage, NotAPackage};
pub use digest::Sha256;
pub use name::{
    MAX_NAME_LEN, PackageName, PackageNameError, PackageRef, SpecifierError, VersionRequirement,
    VersionRequirementError, highest_matching, split_requirement,
};
pub use scope::{Scope, ScopeError};
pub use semver::{Version, VersionReq};
pub use upgrade::{Reach, upgrade_target};

/// The largest package binary a registry stores (the reference server's upload limit), the most bytes any registry
/// answer may be (ADR 0250) and the most bytes read from one project file (ADR 0264).
pub const MAX_PACKAGE_BYTES: u64 = 64 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_package_ceiling_is_the_servers_upload_limit() {
        assert_eq!(MAX_PACKAGE_BYTES, 64 * 1024 * 1024);
    }
}
