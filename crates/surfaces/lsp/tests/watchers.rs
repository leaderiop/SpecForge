//! The file watchers the server asks the client for (ADR 0030, ADR 0035): what the session is built from, each input it
//! classifies, a nested project's config, a symlinked source's target and each hop to it, the directories under the
//! root, and the fact stores of a collected kind.

use specforge_test_macros::test as spec;

/// The watchers derive from the session: relative to each input's
/// directory for a client with relative pattern support, absolute globs
/// otherwise, and the static set with no project.
#[test]
fn file_watchers_follow_what_the_session_is_built_from() {
    use tower_lsp::lsp_types::{GlobPattern, OneOf};

    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"w","version":"0.1.0","extensions":["@acme/local=ext/local.wasm"]}"#,
    )
    .unwrap();
    let session = specforge_project::ProjectSession::open_with_runtime(
        &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
        Some(
            std::sync::Arc::new(specforge_wasm::testing::InProcessRuntime::new())
                as specforge_project::SharedRuntime,
        ),
    );
    let root = dir.path().to_string_lossy().into_owned();

    let absolute: Vec<String> = specforge_lsp::watchers::file_watchers(session.inputs(), false)
        .into_iter()
        .map(|w| match w.glob_pattern {
            GlobPattern::String(glob) => glob,
            other => panic!("expected an absolute glob, got {other:?}"),
        })
        .collect();
    assert_eq!(
        absolute,
        vec![
            format!("{root}/**/*.spec"),
            format!("{root}/**/specforge.json"),
            format!("{root}/**/*"),
            format!("{root}/specforge.json"),
            format!("{root}/specforge.lock"),
            format!("{root}/ext/local.wasm"),
            format!("{root}/.specforge/write-journal.json"),
        ]
    );

    let relative: Vec<(std::path::PathBuf, String)> =
        specforge_lsp::watchers::file_watchers(session.inputs(), true)
            .into_iter()
            .map(|w| match w.glob_pattern {
                GlobPattern::Relative(pattern) => {
                    let OneOf::Right(base) = pattern.base_uri else {
                        panic!("a base URI")
                    };
                    (base.to_file_path().unwrap(), pattern.pattern)
                }
                other => panic!("expected a relative pattern, got {other:?}"),
            })
            .collect();
    assert!(
        relative.contains(&(dir.path().join("ext"), "local.wasm".to_string())),
        "{relative:?}"
    );
    assert!(
        relative.contains(&(dir.path().to_path_buf(), "**/*.spec".to_string())),
        "{relative:?}"
    );
    assert!(
        relative.contains(&(dir.path().to_path_buf(), "**/*".to_string())),
        "{relative:?}"
    );

    let detached = specforge_project::ProjectSession::detached();
    assert_eq!(
        specforge_lsp::watchers::file_watchers(detached.inputs(), true),
        specforge_lsp::watchers::default_watchers()
    );
}

/// Whether `glob` (`**` any depth, `*` one path segment, else literal)
/// matches `path`.
fn glob_matches(glob: &str, path: &str) -> bool {
    fn go(glob: &[&str], path: &[&str]) -> bool {
        match glob.split_first() {
            None => path.is_empty(),
            Some((&"**", rest)) => (0..=path.len()).any(|skipped| go(rest, &path[skipped..])),
            Some((segment, rest)) => path.split_first().is_some_and(|(first, tail)| {
                (*segment == "*" || segment == first) && go(rest, tail)
            }),
        }
    }
    go(
        &glob.split('/').collect::<Vec<_>>(),
        &path.split('/').collect::<Vec<_>>(),
    )
}

/// Every input the session classifies, a file created beside a missing
/// referenced file included, is matched by a watcher glob, spelled under
/// the project root (or canonical outside it).
#[spec(
    behavior = "classify_project_changes",
    verify = "the LSP's watchers cover every input the session classifies"
)]
fn the_watchers_cover_what_the_session_classifies() {
    use specforge_project::ProjectSession;
    use tower_lsp::lsp_types::GlobPattern;

    let far = tempfile::TempDir::new().unwrap();
    let module_dir = std::fs::canonicalize(far.path()).unwrap().join("mods");
    std::fs::create_dir_all(&module_dir).unwrap();
    let module = module_dir.join("ext.wasm");
    // A file reference stays in its root (ADR 0335): the module is the input outside it.
    let dir = crate::session::docref_project(
        "gadget gadget_one \"G\" {\n  docs [\"<project>/docs/guide.md\", \"missing/sub.md\"]\n}\n",
    );
    let root = dir.path();
    std::fs::write(
        root.join("specforge.json"),
        serde_json::json!({
            "name": "p",
            "version": "0.1.0",
            "spec_root": "spec",
            "extensions": [
                "@specforge/software", "@specforge/governance", "@specforge/checks",
                "@sdk/docref=ext/docref.wasm",
                format!("@acme/far={}", module.display()),
            ],
        })
        .to_string(),
    )
    .unwrap();
    let session = ProjectSession::open_from(
        &specforge_common::ProjectRoot::of(root).unwrap(),
        specforge_project::RuntimeSource::project(),
    );

    let globs: Vec<String> = specforge_lsp::watchers::file_watchers(session.inputs(), false)
        .into_iter()
        .map(|w| match w.glob_pattern {
            GlobPattern::String(glob) => glob,
            other => panic!("expected an absolute glob, got {other:?}"),
        })
        .collect();
    let spec_root = root.join("spec");
    // The inputs as the watchers spell them: under the root as opened,
    // canonical outside it.
    let spelled = [
        (root.join("specforge.json"), root.join("specforge.json")),
        (root.join("specforge.lock"), root.join("specforge.lock")),
        (root.join("ext/docref.wasm"), root.join("ext/docref.wasm")),
        (module.clone(), module),
        (root.join("docs/guide.md"), root.join("docs/guide.md")),
        (
            spec_root.join("missing/sub.md"),
            spec_root.join("missing/sub.md"),
        ),
        // A file created beside a missing referenced file changes E016's
        // suggestion.
        (
            spec_root.join("missing/x.md"),
            spec_root.join("missing/x.md"),
        ),
    ];
    for (input, watched) in &spelled {
        std::fs::create_dir_all(input.parent().unwrap()).unwrap();
        std::fs::write(input, "new content").unwrap();
        assert!(
            !session.changes([input.as_path()]).is_empty(),
            "{}",
            input.display()
        );
        let watched = watched.display().to_string();
        assert!(
            globs.iter().any(|glob| glob_matches(glob, &watched)),
            "{watched} is matched by none of {globs:?}"
        );
    }
    assert!(!globs.iter().any(|g| g.contains("/../")), "{globs:?}");
}

#[spec(
    behavior = "classify_project_changes",
    verify = "the LSP's watchers report every specforge.json under the spec root"
)]
fn the_watchers_report_a_nested_projects_specforge_json() {
    use specforge_project::ProjectSession;
    use tower_lsp::lsp_types::{GlobPattern, OneOf, Url, WorkspaceFolder};

    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("spec")).unwrap();
    std::fs::write(root.join("specforge.json"), r#"{"spec_root":"spec"}"#).unwrap();
    std::fs::write(root.join("spec/a.spec"), "").unwrap();
    let session = ProjectSession::open_from(
        &specforge_common::ProjectRoot::of(root).unwrap(),
        specforge_project::RuntimeSource::project(),
    );
    let spec_root = session.inputs().spec_root().unwrap().to_path_buf();

    let absolute: Vec<String> = specforge_lsp::watchers::file_watchers(session.inputs(), false)
        .into_iter()
        .filter_map(|w| match w.glob_pattern {
            GlobPattern::String(glob) => Some(glob),
            GlobPattern::Relative(_) => None,
        })
        .collect();
    for glob in ["**/*.spec", "**/specforge.json"] {
        let expected = format!("{}/{glob}", spec_root.display());
        assert!(
            absolute.contains(&expected),
            "{expected} not in {absolute:?}"
        );
    }

    let relative: Vec<(String, OneOf<WorkspaceFolder, Url>)> =
        specforge_lsp::watchers::file_watchers(session.inputs(), true)
            .into_iter()
            .filter_map(|w| match w.glob_pattern {
                GlobPattern::Relative(r) => Some((r.pattern, r.base_uri)),
                GlobPattern::String(_) => None,
            })
            .collect();
    let spec_base = |base: &OneOf<WorkspaceFolder, Url>| match base {
        OneOf::Right(url) => url.to_file_path().ok().is_some_and(|p| p == spec_root),
        OneOf::Left(_) => false,
    };
    for glob in ["**/*.spec", "**/specforge.json"] {
        assert!(
            relative
                .iter()
                .any(|(pattern, base)| pattern == glob && spec_base(base)),
            "{glob} not in {relative:?}"
        );
    }
}

/// A symlinked source's target lies outside the project: a client that is
/// asked to report only the root never sees it change.
#[cfg(unix)]
#[spec(
    behavior = "classify_project_changes",
    verify = "a session's watch roots and the LSP's watchers cover every symlinked source's target"
)]
fn the_watchers_cover_a_symlinked_sources_target() {
    use tower_lsp::lsp_types::GlobPattern;

    let outside = tempfile::TempDir::new().unwrap();
    let outside_dir = std::fs::canonicalize(outside.path()).unwrap();
    std::fs::write(outside_dir.join("real.spec"), "").unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"w","version":"0.1.0"}"#,
    )
    .unwrap();
    std::os::unix::fs::symlink(outside_dir.join("real.spec"), dir.path().join("link.spec"))
        .unwrap();
    let session = specforge_project::ProjectSession::open_with_runtime(
        &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
        None,
    );

    let target = outside_dir.join("real.spec").display().to_string();
    let globs: Vec<String> = specforge_lsp::watchers::file_watchers(session.inputs(), false)
        .into_iter()
        .map(|w| match w.glob_pattern {
            GlobPattern::String(glob) => glob,
            other => panic!("expected an absolute glob, got {other:?}"),
        })
        .collect();
    assert!(
        globs.iter().any(|glob| glob_matches(glob, &target)),
        "{target} is matched by none of {globs:?}"
    );
}

/// A symlinked source behind a chain of links: the client is asked to report
/// each hop, not only the file the chain ends at.
#[cfg(unix)]
#[spec(
    behavior = "classify_project_changes",
    verify = "a session's watch roots and the LSP's watchers cover every symlinked source's target"
)]
fn the_watchers_cover_each_hop_of_a_symlinked_source() {
    use tower_lsp::lsp_types::GlobPattern;

    let outside = tempfile::TempDir::new().unwrap();
    let outside_dir = std::fs::canonicalize(outside.path()).unwrap();
    std::fs::write(outside_dir.join("real.spec"), "").unwrap();
    std::fs::create_dir_all(outside_dir.join("hops")).unwrap();
    std::os::unix::fs::symlink(outside_dir.join("real.spec"), outside_dir.join("hops/l1")).unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"w","version":"0.1.0"}"#,
    )
    .unwrap();
    std::os::unix::fs::symlink(outside_dir.join("hops/l1"), dir.path().join("link.spec")).unwrap();
    let session = specforge_project::ProjectSession::open_with_runtime(
        &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
        None,
    );

    let globs: Vec<String> = specforge_lsp::watchers::file_watchers(session.inputs(), false)
        .into_iter()
        .map(|w| match w.glob_pattern {
            GlobPattern::String(glob) => glob,
            other => panic!("expected an absolute glob, got {other:?}"),
        })
        .collect();
    for covered in ["hops/l1", "real.spec"] {
        let path = outside_dir.join(covered).display().to_string();
        assert!(
            globs.iter().any(|glob| glob_matches(glob, &path)),
            "{path} is matched by none of {globs:?}"
        );
    }
}

/// A folder moved or deleted sends no event for a `**/*.spec` glob: the
/// client is asked for every entry created or deleted under the root.
#[spec(
    behavior = "classify_project_changes",
    verify = "the LSP asks its client to report every directory created or deleted under the project root"
)]
fn the_client_reports_directories_under_the_root() {
    use tower_lsp::lsp_types::{GlobPattern, WatchKind};

    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"w","version":"0.1.0"}"#,
    )
    .unwrap();
    let session = specforge_project::ProjectSession::open_with_runtime(
        &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
        None,
    );
    let root = dir.path().to_string_lossy().into_owned();

    let watchers = specforge_lsp::watchers::file_watchers(session.inputs(), false);
    assert!(
        watchers.iter().any(|w| {
            matches!(&w.glob_pattern, GlobPattern::String(glob) if *glob == format!("{root}/**/*"))
                && w.kind == Some(WatchKind::Create | WatchKind::Delete)
        }),
        "{watchers:?}"
    );
}

/// The absolute globs the client is asked to watch for `session`.
fn globs(session: &specforge_project::ProjectSession) -> Vec<String> {
    specforge_lsp::watchers::file_watchers(session.inputs(), false)
        .into_iter()
        .filter_map(|w| match w.glob_pattern {
            tower_lsp::lsp_types::GlobPattern::String(glob) => Some(glob),
            _ => None,
        })
        .collect()
}

const FACTS_KEY: &str = "0123456789abcdef0123456789abcdef01234567";

/// A project holding `node`'s recorded fact store, enabling `extensions`.
fn with_a_fact_store(extensions: &[&str]) -> specforge_project::testing::TestProject {
    let p = specforge_project::testing::TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(extensions)
        .installing_enabled()
        .file("a.spec", "");
    let facts = serde_json::json!([{"kind": "test", "id": "js:a.test.js#adds"}]);
    specforge_project::testing::stored(p.root(), "node", FACTS_KEY, facts);
    p
}

/// A collect writes a store and rewrites the ledger: the client reports both (ADR 0327 D9).
#[spec(
    behavior = "watch_file_system_for_changes",
    verify = "a fact store and the harness ledger are watched inputs when a collector-written kind is loaded"
)]
fn the_watchers_cover_the_fact_store_when_a_collected_kind_is_loaded() {
    use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected};
    let p = with_a_fact_store(&COLLECTED_EXTENSIONS);
    let session = p.session_in(std::sync::Arc::new(Collected::new().runtime()));
    let root = p.root().to_string_lossy().into_owned();

    let globs = globs(&session);

    for glob in [
        format!("{root}/.specforge/facts/*"),
        format!("{root}/.specforge/facts/{FACTS_KEY}/*"),
        format!("{root}/specforge-harness"),
    ] {
        assert!(globs.contains(&glob), "{glob} in {globs:?}");
    }
}

#[test]
fn no_fact_watcher_without_a_collected_kind() {
    let p = with_a_fact_store(&[]);
    let session = p.session_bare();

    let globs = globs(&session);

    assert!(
        !globs
            .iter()
            .any(|glob| glob.contains(".specforge/facts") || glob.ends_with("specforge-harness")),
        "{globs:?}"
    );
}
