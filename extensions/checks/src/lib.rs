//! @specforge/checks: the generic value checkers, in five families a setting switches (ADR 0338). Each is decided by a
//! cross-industry standard, so a domain's own checks go in a pack that builds on the same kit. The host performs every
//! probe; a family that is off is not called, sends no probe and adds no scope. The `checks` collector publishes what
//! each check said as `check_outcome` facts.

use specforge_extension_sdk::prelude::*;

mod codes;
mod facts;
mod families;

#[specforge_extension_sdk::extension(
    name = "@specforge/checks",
    version = "1.0.0",
    description = "The generic value checkers: files, formats, check digits, links and git"
)]
struct Checks;

impl Contributions for Checks {
    fn contribute(c: &mut ContributionsBuilder) {
        families::contribute(c);
        facts::contribute(c);
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
