//! Which model each core agent gets, per `model_profile` ([Other presets](../../spec/settings.md#other-presets)).
//!
//! GSD resolves an agent's model as `model_overrides[agent]`, then `models[phase type]`, then the profile's alias.
//! A preset sets `model_overrides.<agent>` from the profile (the second step is not wired: a preset cannot read
//! another setting). The value is a host tier, `heavy` (opus), `standard` (sonnet) or `light` (haiku), which each host
//! maps to its own model; `inherit` presets nothing, so no model line is written.
//!
//! Also ports gsd-core bin/lib modules: model-adapter.cjs, model-catalog.cjs, model-resolver.cjs.

use specforge_extension_sdk::prelude::*;

/// `(agent, [quality, balanced, budget] alias, routing tier)`: the catalog's rows
/// (`gsd/parity/upstream/v1.19.0/model-catalog.json`, `agents`; its `golden` is `quality`).
pub const MODELS: &[(&str, [&str; 3], &str)] = &[
    (
        "gsd-advisor-researcher",
        ["opus", "sonnet", "haiku"],
        "standard",
    ),
    (
        "gsd-assumptions-analyzer",
        ["opus", "sonnet", "sonnet"],
        "heavy",
    ),
    ("gsd-doc-classifier", ["sonnet", "haiku", "haiku"], "light"),
    (
        "gsd-doc-synthesizer",
        ["opus", "sonnet", "haiku"],
        "standard",
    ),
    ("gsd-executor", ["opus", "sonnet", "sonnet"], "standard"),
    (
        "gsd-integration-checker",
        ["sonnet", "sonnet", "haiku"],
        "light",
    ),
    (
        "gsd-nyquist-auditor",
        ["sonnet", "sonnet", "haiku"],
        "light",
    ),
    ("gsd-pattern-mapper", ["sonnet", "sonnet", "haiku"], "light"),
    (
        "gsd-phase-researcher",
        ["opus", "sonnet", "haiku"],
        "standard",
    ),
    ("gsd-plan-checker", ["sonnet", "sonnet", "haiku"], "light"),
    ("gsd-planner", ["opus", "opus", "sonnet"], "heavy"),
    (
        "gsd-project-researcher",
        ["opus", "sonnet", "haiku"],
        "standard",
    ),
    (
        "gsd-research-synthesizer",
        ["sonnet", "sonnet", "haiku"],
        "light",
    ),
    ("gsd-roadmapper", ["opus", "sonnet", "sonnet"], "heavy"),
    ("gsd-verifier", ["sonnet", "sonnet", "haiku"], "standard"),
];

/// The profiles that preset a model, and the column of [`MODELS`] each reads (`None`: the routing tier).
pub const PROFILES: &[(&str, Option<usize>)] = &[
    ("quality", Some(0)),
    ("balanced", Some(1)),
    ("budget", Some(2)),
    ("adaptive", None),
];

/// The host tier of a catalog alias.
pub fn tier(alias: &str) -> &'static str {
    match alias {
        "opus" => "heavy",
        "haiku" => "light",
        _ => "standard",
    }
}

/// The setting that overrides one agent's model.
pub fn setting(agent: &str) -> String {
    format!("model_overrides.{agent}")
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.preset("model_profile", |p| {
        for (profile, column) in PROFILES {
            p.when(profile, |e| {
                for (agent, aliases, routing) in MODELS {
                    let chosen = match column {
                        Some(i) => tier(aliases[*i]),
                        None => routing,
                    };
                    e.set(&setting(agent), chosen);
                }
            });
        }
    });
}
