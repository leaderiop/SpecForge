//! The capabilities an extension declares (ADR 0335 D2.1): the problems the declaration check reports (W021). A
//! capability with one grants nothing (`ExtensionDeclaration::sound_capabilities`).

use specforge_protocol_types::{ExtensionDeclaration, Owner, capability_problems};

/// The messages (W021) about the capabilities `declaration` makes, and about its collectors' `capability`.
pub(crate) fn problems(declaration: &ExtensionDeclaration) -> Vec<String> {
    let name = declaration.name();
    capability_problems(declaration)
        .into_iter()
        .map(|problem| match problem.owner {
            Owner::Capability(index) => format!(
                "extension '{name}': capability '{}' grants nothing: {}",
                declaration.capabilities[index].name, problem.why
            ),
            Owner::Collector(index) => format!(
                "extension '{name}': collector '{}' cannot run: {}",
                declaration.collectors[index].name, problem.why
            ),
        })
        .collect()
}
