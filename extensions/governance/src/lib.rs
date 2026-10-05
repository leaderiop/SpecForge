//! @governance — the governance vocabulary, authored with the extension SDK.
//!
//! Its kinds, edges and rules are declared with the SDK builders in
//! [`declaration`]; the handshake is derived by the SDK from the extension
//! metadata and the contributions.

mod declaration;

use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@specforge/governance",
    version = "1.0.0",
    description = "Governance: decisions, constraints and failure modes"
)]
struct Governance;

impl Contributions for Governance {
    fn contribute(c: &mut ContributionsBuilder) {
        // Diagrams (`model`, `outline`) draw the extension in this colour.
        c.theme_color("#e74c3c");
        c.meta.peer_dependencies.push(PeerDependency {
            name: "@specforge/software".to_string(),
            version: "^1.0".to_string(),
            // Only ConstrainsBehavior targets a software kind (behavior):
            // governance works without software (its manifest spec).
            optional: true,
        });
        c.meta.peer_dependencies.push(PeerDependency {
            name: "@specforge/product".to_string(),
            version: "^1.0".to_string(),
            optional: true,
        });

        declaration::declare(c);
    }
}

fn dispatch(_export: &str, _input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    None
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build, handler = dispatch);
