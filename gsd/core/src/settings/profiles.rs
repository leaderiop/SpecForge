//! The `profile` setting, what each value presets, the one other preset of this round, and the search secrets
//! ([Profiles](../../spec/settings.md#profiles)).

use specforge_extension_sdk::prelude::*;

use crate::codes::{CODES, Severity};

const COMPAT: &str = "gsd-compat";
const PROFILES: [&str; 3] = [COMPAT, "gsd-strict", "gsd-plus"];

/// `(setting, [compat, strict, plus])`: what each profile presets.
const PRESETS: &[(&str, [bool; 3])] = &[
    ("workflow.windows_enforce", [false, true, true]),
    ("specforge.task_commits", [false, true, true]),
    ("specforge.traceability_gate", [false, false, true]),
    ("specforge.stale_verdict_gate", [false, false, true]),
    ("specforge.cross_workstream_conflicts", [false, false, true]),
];

/// `(setting, environment variable)`: the key half of each search availability flag. A secret may not share a
/// setting's name, so it is `<flag>_key`.
const SECRETS: &[(&str, &str)] = &[
    ("brave_search_key", "BRAVE_API_KEY"),
    ("firecrawl_key", "FIRECRAWL_API_KEY"),
    ("exa_search_key", "EXA_API_KEY"),
    ("tavily_search_key", "TAVILY_API_KEY"),
    ("ref_search_key", "REF_API_KEY"),
    ("perplexity_key", "PERPLEXITY_API_KEY"),
    ("jina_key", "JINA_API_KEY"),
];

fn severity(s: Severity) -> ValidationSeverity {
    match s {
        Severity::Error => ValidationSeverity::Error,
        Severity::Warning => ValidationSeverity::Warning,
        Severity::Info => ValidationSeverity::Info,
    }
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.setting("profile", |s| {
        s.enum_values(&PROFILES)
            .default(COMPAT)
            .describe("gsd-compat is GSD as it is; gsd-strict blocks where GSD warns; gsd-plus adds traceability gates.");
    });
    c.preset("profile", |p| {
        for (index, profile) in PROFILES.iter().enumerate() {
            p.when(profile, |e| {
                for (setting, values) in PRESETS {
                    e.set(setting, values[index]);
                }
                if *profile != COMPAT {
                    for code in CODES.iter().filter(|k| k.strict != k.severity) {
                        e.raise(code.id, severity(code.strict));
                    }
                }
            });
        }
    });
    c.preset("workflow.context_drift_action", |p| {
        p.when("block", |e| {
            e.raise("W406", ValidationSeverity::Error);
        });
    });
    for (name, env) in SECRETS {
        c.secret(
            name,
            env,
            "API key of the search source; read from the environment, never from specforge.json.",
        );
    }
}
