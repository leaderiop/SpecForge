//! Own text names no GSD file (ADR 0332 D8): no own span of an asset, and no agent's recording section, names one
//! outside an upstream quote. Kept spans are upstream's text.

#[path = "common/asset_code.rs"]
mod asset_code;
#[path = "common/command_word.rs"]
mod command_word;
#[path = "common/files.rs"]
mod files;
#[path = "common/ledger.rs"]
mod ledger;
#[path = "common/planning_names.rs"]
mod planning_names;
#[path = "common/port_map.rs"]
mod port_map;
#[path = "common/specforge_only.rs"]
mod specforge_only;

use port_map::Span;

#[test]
fn no_own_span_names_a_gsd_file() {
    let mut hits = Vec::new();
    for asset in port_map::assets() {
        let body = port_map::asset_body(&asset).join("\n");
        let own: Vec<std::ops::Range<usize>> = port_map::map(&asset)
            .into_iter()
            .filter_map(|span| match span {
                Span::Own { asset, .. } => Some(asset),
                Span::Kept { .. } => None,
            })
            .collect();
        hits.extend(planning_names::hits(&asset, &body, |n| {
            own.iter().any(|r| r.contains(&n))
        }));
    }
    for agent in gsd_core::agents::roster::AGENTS {
        let recording = gsd_core::agents::recording(agent.id);
        hits.extend(planning_names::hits(
            &format!("agent:{}", agent.id),
            &recording,
            |_| true,
        ));
    }
    planning_names::assert_none(&hits);
}
