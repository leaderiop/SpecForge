//! gsd/E533 end to end (ADR 0390 D3 to D5, D7): the `release` project, a crate `adder` whose committed accepted
//! `tests/public-api.txt` lists `ops::add` and `ops::sub`, is collected over a saved cargo public-api capture
//! (`fresh-public-api.txt`) that lacks `ops::sub`. `@rust/api` marks the item `removed`, `derive` lists it in the open
//! milestone v1.8's `api_breaks`, and the close of v1.8 after the shipped v1.7 is refused under gsd-strict and gsd-plus.
//! Accepting the capture, a major bump and a first release each clear it.

use serde_json::{Value, json};

use super::harness::{Project, RUST_TEST_COLLECTORS, project};
use super::prompts_gates::{gate_failed, refused_by};
use super::prompts_support::prompt;

/// `(profile, blocks)`: the profiles that turn the release gates on (ADR 0390 D8).
pub(super) const PROFILES: [(&str, bool); 3] = [
    ("gsd-compat", false),
    ("gsd-strict", true),
    ("gsd-plus", true),
];

/// The builtins the `release` project loads beside `@gsd/core`: the code facts and the Rust packs whose collectors
/// read its manifest, its items and its public API.
const RUST_API: [&str; 4] = ["@code/core", "@rust/cargo", "@rust/source", "@rust/api"];

/// The saved capture of cargo public-api over the crate once `ops::sub` is dropped.
pub(super) const FRESH: &str = "fresh-public-api.txt";

const REMOVED: &str = "api:rust:adder::ops::sub";

/// The `release` project with [`RUST_API`] loaded and `settings` under `@gsd/core`; v1.7's `completion` is recorded
/// as `close_milestone` writes it, so v1.7 is shipped.
pub(super) fn release(settings: Value) -> Project {
    let p = project("release").with_builtins(&RUST_API);
    p.record_harness("v1.7", "completion", r#"{"at":"2026-10-01"}"#);
    p.settings(settings);
    p
}

/// `collect` of the manifest and the items, then of the API over the saved `capture`.
pub(super) fn collect_api(p: &Project, capture: &str) {
    let run = p.collect_report(&RUST_TEST_COLLECTORS, "rust-api", &p.root().join(capture));
    assert_eq!(run.code, 0, "{}", run.all());
}

/// `(code, message)` of each diagnostic the `complete_milestone` gate of `milestone` fails on; none when it renders.
pub(super) fn close(p: &Project, milestone: &str) -> Vec<(String, String)> {
    let run = prompt(p, "complete_milestone", &[milestone], "claude-code");
    if run.code == 0 {
        return Vec::new();
    }
    gate_failed(&run)["data"]["failing"]
        .as_array()
        .expect("failing")
        .iter()
        .map(|d| {
            let text = |key: &str| d[key].as_str().unwrap_or_default().to_string();
            (text("code"), text("message"))
        })
        .collect()
}

/// The messages of `code` (`gsd/E533`) at the close of `milestone`.
pub(super) fn named(p: &Project, milestone: &str, code: &str) -> Vec<String> {
    close(p, milestone)
        .into_iter()
        .filter(|(c, _)| c == code)
        .map(|(_, message)| message)
        .collect()
}

#[test]
fn e533_refuses_the_close_after_a_collected_removal_under_strict_and_plus() {
    for (profile, blocks) in PROFILES {
        let p = release(json!({ "profile": profile }));
        collect_api(&p, FRESH);
        let removed: Vec<Value> = p
            .nodes_of("api_item")
            .into_iter()
            .filter(|n| n["id"] == REMOVED)
            .collect();
        assert_eq!(removed.len(), 1, "{profile}: {removed:?}");
        assert_eq!(removed[0]["fields"]["change"], "removed", "{profile}");
        assert_eq!(
            p.fields_of("v1.8")["api_breaks"],
            json!([format!("removed {REMOVED}")]),
            "{profile}"
        );
        if blocks {
            refused_by(&p, "complete_milestone", &["v1.8"], "E533");
            let found = named(&p, "v1.8", "gsd/E533");
            assert_eq!(found.len(), 1, "{profile}: {found:?}");
            assert!(
                found[0].contains(REMOVED) && found[0].contains("'v1.7'"),
                "{profile}: {found:?}"
            );
        } else {
            assert_eq!(named(&p, "v1.8", "gsd/E533"), Vec::<String>::new());
        }
    }
}

#[test]
fn e533_refuses_each_break_of_a_close_once() {
    let p = release(json!({ "profile": "gsd-strict" }));
    // `ops::add` widened to i64 and `ops::sub` dropped: two breaks, one error each, sorted by id.
    p.write(
        "two-breaks.txt",
        "pub mod adder\npub mod adder::ops\npub fn adder::ops::add(a: i64, b: i64) -> i64\n",
    );
    collect_api(&p, "two-breaks.txt");
    assert_eq!(
        p.fields_of("v1.8")["api_breaks"],
        json!([
            "changed api:rust:adder::ops::add",
            format!("removed {REMOVED}")
        ])
    );
    assert_eq!(
        named(&p, "v1.8", "gsd/E533"),
        [
            "API item 'api:rust:adder::ops::add' is changed without a major version bump: milestone 'v1.8' is \
             compatible with the last shipped 'v1.7'",
            "API item 'api:rust:adder::ops::sub' is removed without a major version bump: milestone 'v1.8' is \
             compatible with the last shipped 'v1.7'"
        ]
    );
}

#[test]
fn e533_is_silent_once_the_fresh_api_is_committed_as_accepted() {
    let p = release(json!({ "profile": "gsd-strict" }));
    collect_api(&p, FRESH);
    assert_eq!(named(&p, "v1.8", "gsd/E533").len(), 1);
    // Accepting the break: the updated report committed in place of the old one, then collected again.
    p.write("tests/public-api.txt", &p.read(FRESH));
    collect_api(&p, FRESH);
    assert_eq!(p.fields_of("v1.8")["api_breaks"], json!([]));
    assert_eq!(named(&p, "v1.8", "gsd/E533"), Vec::<String>::new());
}

#[test]
fn e533_is_silent_for_a_major_bump() {
    let p = release(json!({ "profile": "gsd-strict" }));
    collect_api(&p, FRESH);
    assert_eq!(named(&p, "v1.8", "gsd/E533").len(), 1);
    p.edit("spec/milestones.spec", "milestone v1.8 ", "milestone v2.0 ");
    p.edit(
        "spec/milestones.spec",
        "milestone        v1.8",
        "milestone        v2.0",
    );
    // The break is still there; v2.0 is not compatible with v1.7, so it is allowed.
    assert_eq!(
        p.fields_of("v2.0")["api_breaks"],
        json!([format!("removed {REMOVED}")])
    );
    assert_eq!(named(&p, "v2.0", "gsd/E533"), Vec::<String>::new());
}

#[test]
fn e533_is_silent_for_a_first_release() {
    let p = release(json!({ "profile": "gsd-strict" }));
    collect_api(&p, FRESH);
    assert_eq!(named(&p, "v1.8", "gsd/E533").len(), 1);
    // v1.7 not shipped: there is no release before v1.8 to break.
    p.edit(
        "spec/milestones.spec",
        "  completion {\n    at 2026-10-01\n  }\n",
        "",
    );
    assert_eq!(p.fields_of("v1.7")["state"], "planned");
    assert_eq!(named(&p, "v1.8", "gsd/E533"), Vec::<String>::new());
}
