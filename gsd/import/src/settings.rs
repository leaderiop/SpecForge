//! `config.json` to `specforge settings set` lines (ADR 0391 D6). The plan never writes `specforge.json`: the report
//! prints one line per mappable key, under the extension that owns it (`gsd/core/spec/settings.md`'s family table),
//! each value POSIX single-quoted because the line is meant to be pasted into a shell and the input is untrusted. A value
//! holding a control character is reported, never printed; a search provider's API key is reported by the name of the
//! environment variable that holds it, never printed.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::preflight::Manifest;
use crate::report::{Entry, Outcome, invisible};

/// The file the lines come from.
pub const CONFIG: &str = ".planning/config.json";

/// The owner of each key family: `gsd/core/spec/settings.md`'s table, token for token (a test holds them equal). A
/// token `a.b` names that key, `a.*` every key under `a.`, `a_*` every key starting `a_`; the longest token that
/// names a key wins, so `workflow.security_enforcement` is `@gsd/security`'s and not `workflow.*`'s.
pub const OWNERS: &[(&str, &[&str])] = &[
    (
        "@gsd/core",
        &[
            "mode",
            "granularity",
            "granularities.*",
            "model_profile",
            "model_profile_overrides.*",
            "model_policy.*",
            "models.*",
            "dynamic_routing.*",
            "runtime",
            "phase_id_convention",
            "project_code",
            "phase_naming",
            "response_language",
            "context_window",
            "context",
            "workflow.*",
            "executor.*",
            "planner.*",
            "parallelization.*",
            "gates.*",
            "git.*",
            "worktree.*",
            "ship.*",
            "review.*",
            "agent_skills",
            "agent_tools.*",
            "hooks.*",
            "statusline.*",
            "refactor.*",
            "model_overrides.*",
            "resolve_model_ids",
            "effort.*",
            "fast_mode.*",
            "manager.*",
            "plan_review.*",
            "audit.*",
            "features.*",
            "learnings.*",
            "claude_orchestration.*",
            "external_job.*",
            "agent_skills_security.*",
            "planning.chunked_parallel",
            "parallelization",
            // The key `parallelization` is the shorthand for.
            "parallelization.enabled",
            // The search availability flags `brave_search` … `jina`: all seven are in SECRETS.
            "brave_search",
            "jina",
        ],
    ),
    (
        "@gsd/security",
        &[
            "workflow.security_enforcement",
            "workflow.security_asvs_level",
            "workflow.security_block_on",
            "security.*",
        ],
    ),
    (
        "@gsd/ui",
        &[
            "workflow.ui_phase",
            "workflow.ui_safety_gate",
            "workflow.ui_review",
            "workflow.live_dom_uat",
            "workflow.ui_interaction_capture",
        ],
    ),
    (
        "@gsd/ai",
        &[
            "workflow.ai_integration_phase",
            "workflow.api_coverage_gate",
        ],
    ),
    (
        "@gsd/ops",
        &[
            "workflow.code_review",
            "workflow.code_review_*",
            "code_quality.*",
            "mempalace.*",
            "profile-pipeline.*",
        ],
    ),
    (
        "@gsd/codebase",
        &[
            "workflow.drift_threshold",
            "workflow.drift_action",
            "workflow.plan_drift_precheck",
            "intel.*",
        ],
    ),
];

/// The search availability flags (`@gsd/core` bools) and the environment variable that holds each one's API key.
pub const SECRETS: &[(&str, &str)] = &[
    ("brave_search", "BRAVE_API_KEY"),
    ("firecrawl", "FIRECRAWL_API_KEY"),
    ("exa_search", "EXA_API_KEY"),
    ("tavily_search", "TAVILY_API_KEY"),
    ("ref_search", "REF_API_KEY"),
    ("perplexity", "PERPLEXITY_API_KEY"),
    ("jina", "JINA_API_KEY"),
];

const NO_PLANNING: &str = "there is no .planning/: spec/gsd/ is source and is committed like code";
const HOST_FILES: &str = "host files are rendered by `specforge install`, which owns their paths";

/// settings.md's "Not applicable" keys (tokens as in [`OWNERS`]), each with its reason.
pub const NOT_APPLICABLE: &[(&str, &str)] = &[
    ("planning.commit_docs", NO_PLANNING),
    ("commit_docs", NO_PLANNING),
    ("phase_commit_docs.*", NO_PLANNING),
    ("planning.search_gitignored", NO_PLANNING),
    ("planning.pr_strict", NO_PLANNING),
    ("search_gitignored", NO_PLANNING),
    (
        "planning.sub_repos",
        "a sub-repository is another SpecForge project",
    ),
    (
        "git.repo_tagging",
        "a sub-repository is another SpecForge project",
    ),
    ("claude_md_path", HOST_FILES),
    ("claude_md_assembly.*", HOST_FILES),
    (
        "workflow.auto_prune_state",
        "STATE.md is derived, nothing to prune",
    ),
    (
        "capabilities.*",
        "an extension is enabled by specforge.json's extensions, and installing one is SpecForge's",
    ),
    (
        "graphify.*",
        "graphify is absorbed: SpecForge is the graph, compiled on every change",
    ),
];

/// The block settings (`{model, reasoning_effort}`) and how many key segments follow their prefix.
const BLOCKS: &[(&str, usize)] = &[
    ("model_profile_overrides", 2),
    ("model_policy.runtime_tiers", 2),
];

/// `text` as one POSIX shell word: single-quoted, each `'` written `'\''`.
pub fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// The settings lines of `config.json`'s `text`, in key order, and an entry for every key that has none.
pub fn settings_lines(text: &str, manifest: &Manifest) -> (Vec<String>, Vec<Entry>) {
    let mut out = Lines::default();
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(map)) => out.object("", &map),
        Ok(_) => out.file(Outcome::Unmapped, "not a JSON object".to_string()),
        Err(e) => out.file(Outcome::Unmapped, format!("not JSON: {e}")),
    }
    let mut lines = Vec::new();
    for (key, value) in std::mem::take(&mut out.values) {
        out.line(&key, &value, manifest, &mut lines);
    }
    out.entries.push(entry(
        "worktree.baseRef",
        Outcome::Unmapped,
        "GSD keeps it in .claude/settings.local.json, outside .planning/: not read".to_string(),
    ));
    (lines, out.entries)
}

/// The flattened keys and what is reported on the way.
#[derive(Default)]
struct Lines {
    values: BTreeMap<String, Value>,
    entries: Vec<Entry>,
}

impl Lines {
    fn file(&mut self, outcome: Outcome, reason: String) {
        self.entries.push(Entry::file(CONFIG, outcome, reason));
    }

    /// Flattens `map` under `prefix` to dotted keys, down to a block setting's depth.
    fn object(&mut self, prefix: &str, map: &Map<String, Value>) {
        if map.is_empty() && !prefix.is_empty() {
            self.entries.push(entry(
                prefix,
                Outcome::Ignored,
                "an empty object sets nothing".to_string(),
            ));
        }
        for (name, value) in map {
            // `agent_tools.*`'s key `*` is every agent: the family's key is a segment, so `all` (settings.md).
            let name = if prefix == "agent_tools" && name == "*" {
                "all"
            } else {
                name
            };
            let key = match prefix {
                "" => name.to_string(),
                _ => format!("{prefix}.{name}"),
            };
            match value {
                Value::Object(inner) if block_depth(&key).is_none_or(|left| left > 0) => {
                    self.object(&key, inner)
                }
                _ => {
                    // `*` and `all`, or `git.x` and `git: {x}`, flatten to one key: the later read is kept, reported.
                    if self.values.insert(key.clone(), value.clone()).is_some() {
                        self.entries.push(entry(
                            &key,
                            Outcome::Unmapped,
                            format!("{key} is given twice in config.json: the last is kept"),
                        ));
                    }
                }
            }
        }
    }

    /// The line of `key`, or the entry saying why it has none.
    fn line(&mut self, key: &str, value: &Value, manifest: &Manifest, lines: &mut Vec<String>) {
        let mut report = |outcome, reason: String| self.entries.push(entry(key, outcome, reason));
        if !key.split('.').all(is_segment) {
            return report(Outcome::Unmapped, "not a setting name".to_string());
        }
        if value.is_null() {
            return report(Outcome::Ignored, "null: unset".to_string());
        }
        if let Some((_, why)) = NOT_APPLICABLE.iter().find(|(token, _)| names(token, key)) {
            return report(Outcome::Unmapped, why.to_string());
        }
        let Some(extension) = owner(key) else {
            return report(Outcome::Unmapped, "not a gsd-core 1.19 setting".to_string());
        };
        if extension != "@gsd/core" && !manifest.loaded.iter().any(|l| l == extension) {
            return report(
                Outcome::NeedsExtension,
                format!("{extension}'s setting, and the project does not load {extension}"),
            );
        }
        if has_invisible(value) {
            return report(
                Outcome::Unmapped,
                format!("control character in the value of {key}"),
            );
        }
        let words = match (key, value) {
            (_, Value::String(_)) if secret(key).is_some() => {
                let variable = secret(key).unwrap_or_default();
                report(
                    Outcome::Setting,
                    format!("a string is an API key, not printed: export {variable} to give it"),
                );
                "true".to_string()
            }
            ("resolve_model_ids", value) => match value {
                Value::Bool(false) => "aliases".to_string(),
                Value::Bool(true) => "ids".to_string(),
                Value::String(s) if s == "omit" => "omit".to_string(),
                _ => {
                    return report(
                        Outcome::Unmapped,
                        "resolve_model_ids takes false, true or \"omit\"".to_string(),
                    );
                }
            },
            (_, Value::String(model)) if block_depth(key) == Some(0) => {
                json_words(&serde_json::json!({ "model": model }))
            }
            (_, Value::Object(_)) => json_words(value),
            (_, Value::String(s)) => shell_quote(s),
            (_, Value::Bool(_) | Value::Number(_)) => value.to_string(),
            (_, Value::Array(items)) if !items.is_empty() && items.iter().all(Value::is_string) => {
                let words: Vec<String> = items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(shell_quote)
                    .collect();
                words.join(" ")
            }
            (_, Value::Array(_) | Value::Null) => json_words(value),
        };
        lines.push(format!("specforge settings set {extension} {key} {words}"));
    }
}

/// An entry about the key `key` of `config.json`.
fn entry(key: &str, outcome: Outcome, reason: String) -> Entry {
    Entry {
        file: CONFIG.to_string(),
        line: None,
        section: key.to_string(),
        outcome,
        reason,
    }
}

/// `--value-json '<compact JSON>'`.
fn json_words(value: &Value) -> String {
    format!("--value-json {}", shell_quote(&value.to_string()))
}

/// A setting key's segment: `[A-Za-z0-9_-]+`.
fn is_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Whether the token `token` names the key `key`: `a.*` / `a_*` a key past the prefix, `a` the key `a` and every key
/// under `a.` (settings.md's `agent_skills` is the family `agent_skills.*`).
fn names(token: &str, key: &str) -> bool {
    match token.strip_suffix('*') {
        Some(prefix) => key.starts_with(prefix) && key.len() > prefix.len(),
        None => key
            .strip_prefix(token)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('.')),
    }
}

/// The extension that owns `key`: the owner of the longest token naming it.
fn owner(key: &str) -> Option<&'static str> {
    if secret(key).is_some() {
        return Some("@gsd/core");
    }
    OWNERS
        .iter()
        .flat_map(|(extension, tokens)| tokens.iter().map(move |token| (*extension, *token)))
        .filter(|(_, token)| names(token, key))
        .max_by_key(|(_, token)| token.len())
        .map(|(extension, _)| extension)
}

/// The environment variable of a search flag.
fn secret(key: &str) -> Option<&'static str> {
    SECRETS
        .iter()
        .find(|(flag, _)| *flag == key)
        .map(|(_, variable)| *variable)
}

/// How many segments a key under a block setting's prefix lacks to be the block (`Some(0)`: it is one); none for a
/// key under no block prefix.
fn block_depth(key: &str) -> Option<usize> {
    BLOCKS.iter().find_map(|(prefix, segments)| {
        let rest = key.strip_prefix(prefix)?.strip_prefix('.')?;
        Some(segments.saturating_sub(rest.split('.').count()))
    })
}

/// Whether any string of `value` (or any object key) holds a character the report would escape ([`invisible`]), so a
/// printed settings line never holds an invisible character.
fn has_invisible(value: &Value) -> bool {
    match value {
        Value::String(s) => s.chars().any(invisible),
        Value::Array(items) => items.iter().any(has_invisible),
        Value::Object(map) => map
            .iter()
            .any(|(k, v)| k.chars().any(invisible) || has_invisible(v)),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
