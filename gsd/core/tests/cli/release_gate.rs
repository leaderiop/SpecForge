//! gsd/E535 end to end (ADR 0390 D6, D7) on the `release` project of [`super::interface_gate`]: the close of the open
//! milestone v1.8 after the shipped v1.7 is refused for a package not at the milestone's version, for a version that
//! goes backwards, and for a last release whose tag resolves to no commit; an exempt package, a lightweight or an
//! annotated tag, and `git.create_tag: false` are not. Each silent case follows the same project firing. The trace
//! route shows the evidence.

use serde_json::{Value, json};

use super::harness::Project;
use super::interface_gate::{FRESH, PROFILES, collect_api, named, release};
use super::prompts_gates::refused_by;

const E535: &str = "gsd/E535";

/// Sets the crate's version in `Cargo.toml` (it is `1.8.0`), then collects the manifest.
fn at_version(p: &Project, version: &str) {
    let text = p.read("Cargo.toml");
    let from = text
        .lines()
        .find(|l| l.starts_with("version = "))
        .expect("the package version")
        .to_string();
    p.write(
        "Cargo.toml",
        &text.replacen(&from, &format!("version = \"{version}\""), 1),
    );
    let run = p.collect(&["--runner", "rust-cargo"]);
    assert_eq!(run.code, 0, "{}", run.all());
}

/// The project committed in a repository of its own, its tags collected by `gsd-git`.
fn tags_collected(p: &Project) {
    p.commit();
    collect_tags(p);
}

fn collect_tags(p: &Project) {
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
}

#[test]
fn e535_refuses_a_package_not_at_the_milestones_version_under_strict_and_plus() {
    for (profile, blocks) in PROFILES {
        let p = release(json!({ "profile": profile }));
        at_version(&p, "1.7.0");
        assert_eq!(
            p.fields_of("v1.8")["release"]["packages"],
            json!([{ "name": "adder", "version": "1.7.0" }]),
            "{profile}"
        );
        let found = named(&p, "v1.8", E535);
        if blocks {
            refused_by(&p, "complete_milestone", &["v1.8"], "E535");
            assert_eq!(
                found,
                ["package 'adder' is at version '1.7.0', not at the version of milestone 'v1.8'"],
                "{profile}"
            );
        } else {
            assert_eq!(found, Vec::<String>::new(), "{profile}");
        }
    }
}

#[test]
fn e535_is_silent_for_a_package_at_the_milestones_version() {
    let p = release(json!({ "profile": "gsd-strict" }));
    at_version(&p, "1.7.0");
    assert_eq!(named(&p, "v1.8", E535).len(), 1);
    at_version(&p, "1.8.0");
    assert_eq!(p.fields_of("v1.8")["release"]["packages"], json!([]));
    assert_eq!(named(&p, "v1.8", E535), Vec::<String>::new());
}

#[test]
fn e535_is_silent_for_an_exempt_package() {
    let p = release(json!({ "profile": "gsd-strict" }));
    at_version(&p, "1.7.0");
    assert_eq!(named(&p, "v1.8", E535).len(), 1);
    p.settings(json!({ "profile": "gsd-strict", "specforge.release_exempt": ["adder"] }));
    assert_eq!(named(&p, "v1.8", E535), Vec::<String>::new());
}

#[test]
fn e535_refuses_a_milestone_that_goes_backwards() {
    let p = release(json!({ "profile": "gsd-strict" }));
    at_version(&p, "1.8.0");
    assert_eq!(named(&p, "v1.8", E535), Vec::<String>::new());
    // The open milestone renamed v1.6 beside the shipped v1.7, its package at its own version.
    p.edit("spec/milestones.spec", "milestone v1.8 ", "milestone v1.6 ");
    p.edit(
        "spec/milestones.spec",
        "milestone        v1.8",
        "milestone        v1.6",
    );
    at_version(&p, "1.6.0");
    refused_by(&p, "complete_milestone", &["v1.6"], "E535");
    assert_eq!(
        named(&p, "v1.6", E535),
        ["milestone 'v1.6' goes backwards from the last shipped 'v1.7': its version is not later"]
    );
}

/// The message of the tag part, naming the last shipped release.
const UNTAGGED: &str = "the tag 'v1.7' of the shipped milestone 'v1.7' resolves to no commit";

#[test]
fn e535_refuses_until_the_last_release_is_tagged() {
    for (profile, blocks) in PROFILES {
        let p = release(json!({ "profile": profile }));
        tags_collected(&p);
        assert_eq!(p.fields_of("v1.7")["tag"], json!({ "name": "v1.7" }));
        if !blocks {
            assert_eq!(named(&p, "v1.8", E535), Vec::<String>::new(), "{profile}");
            continue;
        }
        refused_by(&p, "complete_milestone", &["v1.8"], "E535");
        assert_eq!(named(&p, "v1.8", E535), [UNTAGGED], "{profile}");
        p.git(&["tag", "v1.7"], "2026-10-01T12:00:00Z");
        collect_tags(&p);
        let head = p.head();
        assert_eq!(p.fields_of("v1.7")["tag"]["commit"], json!(head));
        assert_eq!(named(&p, "v1.8", E535), Vec::<String>::new(), "{profile}");
    }
}

#[test]
fn e535_is_silent_for_an_annotated_tag() {
    let p = release(json!({ "profile": "gsd-strict" }));
    tags_collected(&p);
    assert_eq!(named(&p, "v1.8", E535), [UNTAGGED]);
    // An annotated tag resolves to the commit it tags, not to the tag object.
    p.git(
        &["tag", "-a", "v1.7", "-m", "release"],
        "2026-10-01T12:00:00Z",
    );
    collect_tags(&p);
    assert_eq!(p.fields_of("v1.7")["tag"]["commit"], json!(p.head()));
    assert_eq!(named(&p, "v1.8", E535), Vec::<String>::new());
}

#[test]
fn e535_is_silent_when_the_project_creates_no_tag() {
    let p = release(json!({ "profile": "gsd-strict" }));
    tags_collected(&p);
    assert_eq!(named(&p, "v1.8", E535), [UNTAGGED]);
    p.settings(json!({ "profile": "gsd-strict", "git.create_tag": false }));
    collect_tags(&p);
    assert_eq!(named(&p, "v1.8", E535), Vec::<String>::new());
}

/// `specforge trace <milestone> --format json`.
fn trace(p: &Project, milestone: &str) -> Value {
    let run = p.run(&["trace", milestone, "--format", "json"]);
    serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("trace prints JSON ({e}): {}", run.all()))
}

/// The start's annotation `name`'s values.
fn annotation(trace: &Value, name: &str) -> Value {
    trace["annotations"]
        .as_array()
        .expect("the start's annotations")
        .iter()
        .find(|a| a["name"] == name)
        .unwrap_or_else(|| panic!("no annotation {name}: {trace:#}"))["values"]
        .clone()
}

#[test]
fn trace_follows_the_milestone_release_route() {
    let p = release(json!({ "profile": "gsd-strict" }));
    at_version(&p, "1.7.0");
    collect_api(&p, FRESH);
    let found = trace(&p, "v1.8");
    assert_eq!(found["route"], "gsd/milestone-release", "{found:#}");
    assert_eq!(
        annotation(&found, "api_breaks"),
        json!(["removed api:rust:adder::ops::sub"]),
        "{found:#}"
    );
    let release: Value = serde_json::from_str(
        annotation(&found, "release")[0]
            .as_str()
            .expect("the release, as JSON text"),
    )
    .expect("JSON");
    assert_eq!(
        release,
        json!({ "packages": [{ "name": "adder", "version": "1.7.0" }] }),
        "{found:#}"
    );
    assert_eq!(found["steps"][0]["matches"][0]["entity_id"], "v1.8/01");
}
