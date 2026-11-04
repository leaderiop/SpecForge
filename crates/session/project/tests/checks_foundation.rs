//! `@specforge/checks` as a project runs it: the committed blob, attached to a field in `specforge.json`.

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_effects::git::{Located, Sha};
use specforge_effects::testing::{FakeGit, FakeHttp, MemoryConsent};
use specforge_effects::{EffectsMode, OutcomeClass};
use specforge_project::CompiledProject;
use specforge_project::testing::TestProject;
use specforge_project::values::CheckStatus;
use specforge_registry::PlainDisk;
use std::path::PathBuf;
use std::sync::Arc;

use crate::value_probes::Scene;

const CHECKS: &str = "@specforge/checks";
const PRODUCT: &str = "@specforge/product";

/// A project with one feature, its `owner` written as `owner` and checked by `checks`.
pub(crate) fn feature(owner: &str, checks: Value) -> TestProject {
    TestProject::new()
        .config(|c| c["checks"] = json!({ "fields": { "feature.owner": checks } }))
        .enabling(&[PRODUCT, CHECKS])
        .file(
            "a.spec",
            format!(
                "feature f \"F\" {{\n  status planned\n  problem \"p\"\n  solution \"s\"\n  owner \"{owner}\"\n}}\n"
            ),
        )
}

fn coded(diagnostics: &[Diagnostic], code: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.code == code)
        .cloned()
        .collect()
}

#[specforge_test_macros::test(
    behavior = "ck_formats_family",
    verify = "a format that refuses a value reports E103 on the part of the value's span the kit points at"
)]
fn checks_formats_report_e103_on_the_value_span() {
    let project = feature("ada at example.com", json!(["@specforge/checks:email"])).compile();
    let diagnostics = project.diagnostics();
    let found = coded(&diagnostics, "E103");
    assert_eq!(found.len(), 1, "{diagnostics:?}");
    let span = found[0].span.as_ref().expect("on the value");
    assert_eq!(span.start_line, 5, "{span:?}");

    let at = feature(
        "1.2.3-rc.1",
        json!([{"checker": "@specforge/checks:semver", "args": {"prerelease": false}}]),
    )
    .compile()
    .diagnostics();
    let found = coded(&at, "E103");
    assert_eq!(found.len(), 1, "{at:?}");
    let span = found[0].span.as_ref().expect("on the part");
    // `  owner "` is nine columns, and the pre-release starts five bytes in.
    assert_eq!(span.start_col, 9 + 5 + 1, "{span:?}");
    assert_eq!(span.end_col, 9 + 10 + 1, "{span:?}");
}

#[specforge_test_macros::test(
    behavior = "ck_digits_family",
    verify = "a digit checker needs no scope and asks no approval"
)]
fn checks_digits_need_no_scope_and_no_approval() {
    let project = feature("79927398710", json!(["@specforge/checks:luhn"])).compile();
    let diagnostics = project.diagnostics();
    assert_eq!(coded(&diagnostics, "E103").len(), 1, "{diagnostics:?}");
    assert!(coded(&diagnostics, "W170").is_empty(), "{diagnostics:?}");
    assert!(coded(&diagnostics, "I206").is_empty(), "{diagnostics:?}");

    let clean = feature("79927398713", json!(["@specforge/checks:luhn"]))
        .compile()
        .diagnostics();
    assert!(coded(&clean, "E103").is_empty(), "{clean:?}");
}

#[specforge_test_macros::test(
    behavior = "ck_family_switches",
    verify = "a family whose switch is off reports nothing"
)]
fn checks_a_family_switched_off_reports_nothing() {
    let project = feature("79927398710", json!(["@specforge/checks:luhn"]))
        .config(|c| c["settings"] = json!({ CHECKS: { "families.digits": false } }))
        .compile();
    let diagnostics = project.diagnostics();
    assert!(coded(&diagnostics, "E103").is_empty(), "{diagnostics:?}");
}

#[specforge_test_macros::test(
    behavior = "ck_files_family",
    verify = "a missing file in a list reports E016 on that item's span, not the entity's"
)]
fn checks_files_e016_sits_on_the_values_span() {
    let refs = r#"  refs ["here.md", "missing.md"]"#;
    let project = TestProject::new()
        .config(|c| c["checks"] = json!({ "fields": { "feature.refs": ["@specforge/checks:file"] } }))
        .enabling(&[PRODUCT, CHECKS])
        .file("here.md", "here")
        .file(
            "a.spec",
            format!("feature f \"F\" {{\n  status planned\n  problem \"p\"\n  solution \"s\"\n{refs}\n}}\n"),
        )
        .compile();
    let diagnostics = project.diagnostics();
    let found = coded(&diagnostics, "E016");
    assert_eq!(found.len(), 1, "{diagnostics:?}");
    let span = found[0].span.as_ref().expect("on the item");
    let quote = refs.find("\"missing.md\"").unwrap() + 1;
    assert_eq!(span.start_line, 5, "{span:?}");
    assert!([quote, quote + 1].contains(&span.start_col), "{span:?}");
}

/// A feature whose `refs` holds `refs` (each quoted), checked by `checker` with `args`, with `spec_root: spec`.
fn refs_project(refs: &[&str], checker: &str, args: Value) -> TestProject {
    let items: Vec<String> = refs.iter().map(|r| format!("\"{r}\"")).collect();
    TestProject::new()
        .config(|c| {
            c["spec_root"] = "spec".into();
            c["checks"] = json!({ "fields": { "feature.refs": [{ "checker": checker, "args": args }] } });
        })
        .enabling(&[PRODUCT, CHECKS])
        .file(
            "spec/a.spec",
            format!(
                "feature f \"F\" {{\n  status planned\n  problem \"p\"\n  solution \"s\"\n  refs [{}]\n}}\n",
                items.join(", ")
            ),
        )
}

#[specforge_test_macros::test(
    behavior = "ck_files_family",
    verify = "a sibling is suggested only from inside the root"
)]
fn checks_files_suggest_a_sibling_inside_the_root_only() {
    let file = "@specforge/checks:file";
    let inside = refs_project(&["guid.md"], file, json!({ "root": "spec" }))
        .file("spec/guide.md", "")
        .compile()
        .diagnostics();
    let found = coded(&inside, "E016");
    assert_eq!(found.len(), 1, "{inside:?}");
    assert_eq!(
        found[0].suggestion.as_deref(),
        Some("did you mean 'guide.md'?")
    );

    // `notes.md` lies beside the spec root, not in it: no name from outside the root is offered.
    let outside = refs_project(&["note.md"], file, json!({ "root": "spec" }))
        .file("notes.md", "")
        .compile()
        .diagnostics();
    let found = coded(&outside, "E016");
    assert_eq!(found.len(), 1, "{outside:?}");
    assert_eq!(found[0].suggestion, None, "{outside:?}");
}

#[specforge_test_macros::test(
    behavior = "ck_files_family",
    verify = "a file whose digest differs from its pin is E096"
)]
fn checks_files_digest_pin_mismatch_is_e096() {
    let digest = "@specforge/checks:digest";
    // The SHA-256 of `abc`.
    let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let pinned = |bytes: &str| {
        refs_project(&[&format!("data.bin@sha256:{abc}")], digest, json!({}))
            .file("data.bin", bytes)
            .compile()
            .diagnostics()
    };
    let same = pinned("abc");
    assert!(coded(&same, "E096").is_empty(), "{same:?}");
    let other = pinned("abd");
    let found = coded(&other, "E096");
    assert_eq!(found.len(), 1, "{other:?}");
    let span = found[0].span.as_ref().expect("on the digest");
    // `  refs ["` is nine columns, and the digest starts sixteen bytes into the value.
    assert_eq!(span.start_col, 9 + 16 + 1, "{span:?}");
}

#[specforge_test_macros::test(
    behavior = "ck_files_family",
    verify = "a line range past the end of the file is W176"
)]
fn checks_files_line_range_past_the_end_is_w176() {
    let range = "@specforge/checks:line_range";
    let lines = |value: &str| {
        refs_project(&[value], range, json!({}))
            .file("src/a.rs", "one\ntwo\nthree\n")
            .compile()
            .diagnostics()
    };
    let within = lines("src/a.rs:2-3");
    assert!(coded(&within, "W176").is_empty(), "{within:?}");
    let past = lines("src/a.rs:2-9");
    let found = coded(&past, "W176");
    assert_eq!(found.len(), 1, "{past:?}");
    assert!(found[0].message.contains('9'), "{}", found[0].message);
}

/// A feature whose `refs` holds `refs` and whose `owner` is `owner`, each field checked as `checks` says.
fn links_project(refs: &[&str], owner: &str, checks: Value) -> TestProject {
    let items: Vec<String> = refs.iter().map(|r| format!("\"{r}\"")).collect();
    TestProject::new()
        .config(|c| c["checks"] = json!({ "fields": checks }))
        .enabling(&[PRODUCT, CHECKS])
        .file(
            "a.spec",
            format!(
                "feature f \"F\" {{\n  status planned\n  problem \"p\"\n  solution \"s\"\n  owner \"{owner}\"\n  refs [{}]\n}}\n",
                items.join(", ")
            ),
        )
}

/// `dir` with the builtins, over `http`.
fn over(dir: TestProject, http: FakeHttp) -> Scene {
    Scene {
        dir,
        runtime: specforge_project::testing::component_runtime(),
        http: Arc::new(http),
    }
}

fn links_scene(refs: &[&str], owner: &str, checks: Value, http: FakeHttp) -> Scene {
    over(links_project(refs, owner, checks), http)
}

const URL: &str = "@specforge/checks:url";
const GONE: &str = "https://gone.example/";

#[specforge_test_macros::test(
    behavior = "ck_links_family",
    verify = "a URL that answers 404 is E093, and W183 on a field whose check lowers it"
)]
fn checks_links_404_is_e093_and_a_lowered_field_is_w183() {
    let http = || FakeHttp::new().route(GONE, 404, "");
    let plain = links_scene(&[GONE], "o", json!({ "feature.refs": [URL] }), http());
    let found = plain.online().diagnostics();
    assert_eq!(coded(&found, "E093").len(), 1, "{found:?}");
    let span = coded(&found, "E093")[0].span.clone().expect("on the value");
    assert_eq!(span.start_line, 6, "{span:?}");

    let lowered = json!({ "feature.refs": [{ "checker": URL, "args": { "status": "warn" } }] });
    let found = links_scene(&[GONE], "o", lowered, http())
        .online()
        .diagnostics();
    assert!(coded(&found, "E093").is_empty(), "{found:?}");
    assert_eq!(coded(&found, "W183").len(), 1, "{found:?}");
}

#[specforge_test_macros::test(
    behavior = "ck_links_family",
    verify = "without approval only the links scope is asked for"
)]
fn checks_links_unapproved_asks_only_the_links_scope() {
    let scene = links_scene(
        &[GONE],
        "o",
        json!({ "feature.refs": [URL] }),
        FakeHttp::new(),
    );
    let found = scene
        .checked(EffectsMode::Auto, &MemoryConsent::default())
        .diagnostics();
    let asked = coded(&found, "W170");
    assert_eq!(asked.len(), 1, "{found:?}");
    let message = &asked[0].message;
    assert!(message.contains("'links'"), "{message}");
    assert!(
        !message.contains("'files'") && !message.contains("'git'"),
        "{message}"
    );
    assert!(scene.http.requests().is_empty());
}

#[specforge_test_macros::test(
    behavior = "ck_links_family",
    verify = "a links family switched off sends no request"
)]
fn checks_links_off_switch_sends_no_probe() {
    let dir = links_project(&[GONE], "o", json!({ "feature.refs": [URL] }))
        .config(|c| c["settings"] = json!({ CHECKS: { "families.links": false } }));
    let scene = over(dir, FakeHttp::new());
    let found = scene
        .checked(EffectsMode::Auto, &MemoryConsent::default())
        .diagnostics();
    assert!(coded(&found, "W170").is_empty(), "{found:?}");
    assert!(coded(&found, "E093").is_empty(), "{found:?}");
    assert!(scene.http.requests().is_empty());
    assert!(scene.http.asked().is_empty());
}

#[specforge_test_macros::test(
    behavior = "ck_links_family",
    verify = "the remote's answers are machine-decided warnings, the project's own are not"
)]
fn checks_links_transport_warnings_are_machine() {
    let (down, moved, broken, closed, own) = (
        "https://down.example/",
        "https://moved.example/",
        "https://broken.example/",
        "https://closed.example/",
        "https://own.example/",
    );
    let http = FakeHttp::new()
        .redirect(moved, "http://moved.example/")
        .route(broken, 500, "")
        .route(closed, 403, "")
        .route(own, 404, "")
        .certificate(
            "own.example",
            443,
            "2000-01-01T00:00:00Z",
            "an issuer",
            true,
        );
    let checks = json!({
        "feature.refs": [URL],
        "feature.owner": [
            { "checker": URL, "args": { "status": "warn" } },
            "@specforge/checks:tls"
        ]
    });
    let found = links_scene(&[down, moved, broken, closed], own, checks, http)
        .online()
        .diagnostics();
    for (code, machine) in [
        ("W172", true),
        ("W173", true),
        ("W181", true),
        ("W182", true),
        ("W178", false),
        ("W183", false),
    ] {
        let reported = coded(&found, code);
        assert_eq!(reported.len(), 1, "{code}: {found:?}");
        assert_eq!(reported[0].machine, machine, "{code}");
    }
}

#[specforge_test_macros::test(
    behavior = "ck_links_family",
    verify = "a HEAD answered 405 or 501 is asked once more as a GET"
)]
fn checks_links_head_405_retries_as_get() {
    let refs = json!({ "feature.refs": [URL] });
    for refused in [405, 501] {
        let http = FakeHttp::new().route(GONE, 200, "").head(GONE, refused);
        let scene = links_scene(&[GONE], "o", refs.clone(), http);
        let found = scene.online().diagnostics();
        let methods: Vec<String> = scene
            .http
            .requests()
            .into_iter()
            .map(|r| r.method)
            .collect();
        assert_eq!(methods, ["HEAD", "GET"], "{refused}");
        assert!(coded(&found, "W181").is_empty(), "{found:?}");
    }
    let http = FakeHttp::new().route(GONE, 200, "").head(GONE, 404);
    let scene = links_scene(&[GONE], "o", refs, http);
    let found = scene.online().diagnostics();
    assert_eq!(scene.http.requests().len(), 1, "any other answer stands");
    assert_eq!(coded(&found, "E093").len(), 1, "{found:?}");
}

/// `scene` compiled online with `git` as its Git port.
fn compiled_with_git(scene: &Scene, git: FakeGit) -> CompiledProject {
    let env = scene.env().with_git(Arc::new(git));
    let root = scene.dir.project_root();
    let consent = MemoryConsent::default();
    let effects =
        specforge_project::effects::of(EffectsMode::Online, &scene.ports(), &env, &root, &consent);
    CompiledProject::compile_through(env.with_effects(effects), &PlainDisk)
}

fn with_git(scene: &Scene, git: FakeGit) -> Vec<Diagnostic> {
    compiled_with_git(scene, git).diagnostics()
}

/// A repository at the scene's root whose `HEAD` holds `files`, which knows `v1` as another commit.
fn repo(scene: &Scene, files: &[&str]) -> FakeGit {
    let mut git = FakeGit::repo(
        scene.dir.root(),
        "0000000000000000000000000000000000000001",
        Vec::new(),
    );
    git.named = vec![(
        "v1".into(),
        Sha::new("1234567890abcdef1234567890abcdef12345678"),
        Vec::new(),
    )];
    git.blobs = files
        .iter()
        .map(|f| (PathBuf::from(f), b"x".to_vec()))
        .collect();
    git
}

#[specforge_test_macros::test(
    behavior = "ck_git_family",
    verify = "a commit, ref or file at a revision the repository lacks is E106"
)]
fn checks_git_unknown_commit_is_e106() {
    let checks = json!({ "feature.refs": ["@specforge/checks:commit"] });
    let scene = links_scene(&["v1", "abcdef1234"], "o", checks, FakeHttp::new());
    let found = with_git(&scene, repo(&scene, &[]));
    let missing = coded(&found, "E106");
    assert_eq!(missing.len(), 1, "{found:?}");
    assert!(
        missing[0].message.contains("abcdef1234"),
        "{}",
        missing[0].message
    );
    // `v1` is no hex object name: refused before git is asked.
    assert_eq!(coded(&found, "E103").len(), 1, "{found:?}");

    let checks = json!({ "feature.refs": ["@specforge/checks:file_at"] });
    let scene = links_scene(
        &["v1:src/a.rs", "v1:src/b.rs", "v2:src/a.rs"],
        "o",
        checks,
        FakeHttp::new(),
    );
    let found = with_git(&scene, repo(&scene, &["src/a.rs"]));
    let missing = coded(&found, "E106");
    assert_eq!(missing.len(), 2, "{found:?}");
    assert!(
        missing.iter().any(|d| d.message.contains("src/b.rs")),
        "{missing:?}"
    );
    assert!(
        missing.iter().any(|d| d.message.contains("'v2'")),
        "{missing:?}"
    );
}

#[specforge_test_macros::test(
    behavior = "ck_git_family",
    verify = "a path the commit at HEAD does not hold is W180"
)]
fn checks_git_ignored_path_is_w180() {
    let checks = json!({ "feature.refs": ["@specforge/checks:tracked"] });
    let dir = links_project(&["a.spec", "ignored.md"], "o", checks).file("ignored.md", "x");
    let scene = over(dir, FakeHttp::new());
    let found = with_git(&scene, repo(&scene, &["a.spec"]));
    let untracked = coded(&found, "W180");
    assert_eq!(untracked.len(), 1, "{found:?}");
    assert!(
        untracked[0].message.contains("ignored.md"),
        "{}",
        untracked[0].message
    );
}

#[specforge_test_macros::test(
    behavior = "ck_git_family",
    verify = "a git question about a withheld path is denied, as a read of it is"
)]
fn checks_git_withheld_path_is_denied() {
    let checks = json!({ "feature.refs": ["@specforge/checks:file_at"] });
    let scene = links_scene(
        &["v1:.env", "v1:.specforge/effects/journal.json"],
        "o",
        checks,
        FakeHttp::new(),
    );
    let held = &[".env", ".specforge/effects/journal.json"];
    let project = compiled_with_git(&scene, repo(&scene, held));
    let classes: Vec<_> = project
        .value_checks()
        .of("f")
        .iter()
        .map(|row| row.outcome_class)
        .collect();
    assert_eq!(classes, [Some(OutcomeClass::Denied); 2]);

    let checks = json!({ "feature.refs": ["@specforge/checks:tracked"] });
    let dir = links_project(&[".env"], "o", checks).file(".env", "SECRET=1");
    let scene = over(dir, FakeHttp::new());
    let project = compiled_with_git(&scene, repo(&scene, &[".env"]));
    let row = &project.value_checks().of("f")[0];
    assert_eq!(row.outcome_class, Some(OutcomeClass::Denied), "{row:?}");
}

/// `git` as a shallow clone.
fn shallow(mut git: FakeGit) -> FakeGit {
    if let Located::Repo(place) = &mut git.located {
        place.shallow = true;
    }
    git
}

#[specforge_test_macros::test(
    behavior = "ck_git_family",
    verify = "no repository, a shallow clone's missing history and a tag a clone did not fetch leave the check unchecked"
)]
fn checks_git_a_miss_git_cannot_decide_is_unchecked() {
    // No repository: a source tarball.
    let checks = json!({ "feature.refs": ["@specforge/checks:commit"] });
    let scene = links_scene(&["abcdef1234"], "o", checks, FakeHttp::new());
    let project = compiled_with_git(&scene, FakeGit::none());
    assert!(coded(&project.diagnostics(), "E106").is_empty());
    assert_eq!(
        project.value_checks().of("f")[0].status,
        CheckStatus::NotRun
    );
    let checks = json!({ "feature.refs": ["@specforge/checks:tracked"] });
    let dir = links_project(&["a.md"], "o", checks).file("a.md", "x");
    let found = with_git(&over(dir, FakeHttp::new()), FakeGit::none());
    assert!(coded(&found, "W180").is_empty(), "{found:?}");

    // A shallow clone: an older commit is not there to ask, but a commit it holds still decides its files.
    let checks = json!({ "feature.refs": ["@specforge/checks:commit"] });
    let scene = links_scene(&["abcdef1234"], "o", checks, FakeHttp::new());
    let found = with_git(&scene, shallow(repo(&scene, &[])));
    assert!(coded(&found, "E106").is_empty(), "{found:?}");
    let checks = json!({ "feature.refs": ["@specforge/checks:file_at"] });
    let scene = links_scene(
        &["v2:src/a.rs", "v1:src/b.rs"],
        "o",
        checks,
        FakeHttp::new(),
    );
    let found = coded(
        &with_git(&scene, shallow(repo(&scene, &["src/a.rs"]))),
        "E106",
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("src/b.rs"),
        "{}",
        found[0].message
    );

    // A full clone that fetched no tag cannot say one is missing; one that holds a tag can.
    let tag =
        json!({ "feature.refs": [{"checker": "@specforge/checks:ref", "args": {"kind": "tag"}}] });
    let scene = links_scene(&["v9"], "o", tag.clone(), FakeHttp::new());
    let found = with_git(&scene, repo(&scene, &[]));
    assert!(coded(&found, "E106").is_empty(), "{found:?}");
    let dir = links_project(&["v9"], "o", tag).file(
        ".git/refs/tags/v1",
        "1234567890abcdef1234567890abcdef12345678\n",
    );
    let scene = over(dir, FakeHttp::new());
    let found = with_git(&scene, repo(&scene, &[]));
    assert_eq!(coded(&found, "E106").len(), 1, "{found:?}");
}
