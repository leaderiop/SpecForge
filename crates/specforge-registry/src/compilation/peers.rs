//! The registry build's half of the peer requirements (ADR 0041): what the peer rule reports of
//! the loaded declarations.

use specforge_common::Diagnostic;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_protocol_types::peers::{Member, Peers};

/// The peer diagnostics of `declarations`: each unsatisfied peer (E073, E027), declaration by
/// declaration.
pub(crate) fn check(declarations: &[ExtensionDeclaration]) -> Vec<Diagnostic> {
    Peers::of(declarations.iter().map(Member::from))
        .unsatisfied()
        .iter()
        .filter_map(|u| specforge_common::peers::of(u.dependent, u.declared, &u.verdict))
        .collect()
}
