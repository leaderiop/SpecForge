//! What every check after the graph build reads about one entity (ADR 0019).
//! Graph-free, so the registry checks and the rules read it without the
//! graph; `specforge_project` decides it from the parse tree and the
//! registries.

/// Why an entity owes no obligations of its own whatever its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exemption {
    /// A union (`type X = A | B`): no body to hold them.
    Union,
    /// It sets `field`, which its kind's registry entry declares
    /// `exempts_obligations` (`@specforge/formal`'s `abstract true`).
    Flag { field: String },
    /// Its kind accepts no `verify` statements (`supports_verify` unset), so
    /// it has nowhere to declare them. Exempts it from statement obligations
    /// only: a `no_verify_statements` rule whose obligation is another field
    /// ignores it.
    NoVerify,
}

impl Exemption {
    /// It exempts from obligations declared in a field other than `verify`
    /// statements: a union body or an exempting flag, not a kind that only
    /// lacks `verify`.
    pub fn exempts_fields(&self) -> bool {
        !matches!(self, Exemption::NoVerify)
    }
}
