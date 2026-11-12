//! The capabilities an extension declares (ADR 0335 D2.1): the problems the declaration check reports (W021). A
//! capability with one grants nothing (`ExtensionDeclaration::sound_capabilities`).

use specforge_protocol_types::{
    ExtensionDeclaration, INSTALL_LINE_MAX, Owner, capability_problems, is_install_line,
};

/// The messages (W021) about the capabilities `declaration` makes, and about its collectors' `capability`: a
/// collector that keeps a `capture` with no command has no output to keep (ADR 0345 D2), and an install line must be
/// one printable line (ADR 0348 D4).
pub(crate) fn problems(declaration: &ExtensionDeclaration) -> Vec<String> {
    let name = declaration.name();
    let uncaptured = declaration
        .collectors
        .iter()
        .filter(|collector| collector.capture.is_some() && collector.capability.is_none())
        .map(move |collector| {
            format!(
                "extension '{name}': collector '{}' keeps a capture but declares no command, so there is no \
                 output to keep",
                collector.name
            )
        });
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
        .chain(uncaptured)
        .chain(declaration.collectors.iter().filter_map(move |collector| {
            let line = collector.install.as_deref()?;
            (!is_install_line(line)).then(|| {
                format!(
                    "extension '{name}': collector '{}': its install line is empty, over {INSTALL_LINE_MAX} bytes \
                     or not printable",
                    collector.name
                )
            })
        }))
        .collect()
}
