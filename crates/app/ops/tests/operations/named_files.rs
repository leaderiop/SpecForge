//! A spec file a request names is read into its source key once (ADR 0195): a typed create's `file`, outline's
//! `file`, the fixes' `file_path`.

use crate::edit::{TASKS, create, one};
use crate::navigate::{LIMIT, compile};
use specforge_ops::authoring::edit::{self, Edit, EditPlan};
use specforge_ops::authoring::fix::{self, FixField, FixQuery};
use specforge_ops::view::ProjectView;
use specforge_project::testing::compile_at;

/// A project whose spec root is `spec/`, holding `files` (named from that root), and the plan of `edits` over it.
fn plan_in_spec_dir(files: &[(&str, &str)], edits: &[Edit]) -> EditPlan {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        serde_json::json!({"name": "n", "spec_root": "spec", "extensions": ["@specforge/software"]})
            .to_string(),
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("spec")).unwrap();
    for (name, text) in files {
        std::fs::write(dir.path().join("spec").join(name), text).unwrap();
    }
    let project = compile_at(
        dir.path(),
        Some(specforge_project::testing::component_runtime()),
    );
    edit::plan(&ProjectView::of(&project), edits).unwrap()
}

const TESTING: &[&str] = &["@specforge/software", "@specforge/testing"];

#[specforge_test_macros::test(
    behavior = "create_entity",
    verify = "new reads every spelling of a file under the project root as that file"
)]
fn a_create_in_a_spelling_of_an_existing_file_adds_to_it() {
    let planned = plan_in_spec_dir(
        &[("tasks.spec", TASKS)],
        &[create(
            "behavior",
            "archive_task",
            "./spec/tasks.spec",
            &[("contract", one("The system MUST archive."))],
        )],
    );
    assert_eq!(planned.report().affected_files, ["spec/tasks.spec"]);
    assert!(planned.change().after()["spec/tasks.spec"].starts_with(TASKS));
}

#[specforge_test_macros::test(
    behavior = "create_entity",
    verify = "two spellings of one new file in one list are one file"
)]
fn two_spellings_of_one_new_file_are_one_file() {
    let planned = plan_in_spec_dir(
        &[("tasks.spec", TASKS)],
        &[
            create(
                "behavior",
                "alpha_one",
                "./spec/n.spec",
                &[("contract", one("The system MUST archive."))],
            ),
            create(
                "behavior",
                "beta_one",
                "spec/n.spec",
                &[("contract", one("The system MUST archive."))],
            ),
        ],
    );
    assert_eq!(planned.report().affected_files, ["spec/n.spec"]);
    let after = &planned.change().after()["spec/n.spec"];
    assert!(
        after.contains("behavior alpha_one") && after.contains("behavior beta_one"),
        "{after}"
    );
}

const LOGOUT: &str =
    "behavior login \"Login\" {\n  contract \"c\"\n  invariants [sesion_limit]\n}\n";

#[specforge_test_macros::test(
    behavior = "provide_mcp_suggest_fixes_tool",
    verify = "file_path reads every spelling of a source as that source"
)]
fn the_fixes_read_a_spelling_of_a_source_as_that_source() {
    let p = compile(TESTING, &[("limit.spec", LIMIT), ("login.spec", LOGOUT)]);
    let titles = |file: &str| -> Vec<String> {
        let query = FixQuery {
            file: Some(file),
            ..FixQuery::default()
        };
        fix::fixes(&p.navigator(), &p.project.diagnostics(), &query)
            .unwrap()
            .into_iter()
            .map(|f| f.title)
            .collect()
    };
    let exact = titles("login.spec");
    assert!(!exact.is_empty());
    let absolute = p.project.environment().spec_root().join("login.spec");
    assert_eq!(titles("./login.spec"), exact);
    assert_eq!(titles(absolute.to_str().unwrap()), exact);
}

#[specforge_test_macros::test(
    behavior = "provide_mcp_suggest_fixes_tool",
    verify = "a file_path that names no source is refused on file_path"
)]
fn the_fixes_refuse_a_file_that_is_no_source() {
    let p = compile(TESTING, &[("limit.spec", LIMIT)]);
    let refusal = |file: &str| {
        let query = FixQuery {
            file: Some(file),
            ..FixQuery::default()
        };
        fix::fixes(&p.navigator(), &p.project.diagnostics(), &query)
            .unwrap_err()
            .error
            .code
            .to_string()
    };
    assert_eq!(refusal("nope.spec"), "file_not_found");
    let query = FixQuery {
        file: Some("nope.spec"),
        ..FixQuery::default()
    };
    let refused = fix::fixes(&p.navigator(), &p.project.diagnostics(), &query).unwrap_err();
    assert_eq!(refused.subject, Some(FixField::FilePath));
    assert_eq!(refusal("../x.spec"), "not_a_source");
}

const A: &str = "invariant rule_a \"A\" {\n  guarantee \"x\"\n}\n";
const B: &str = "invariant rule_b \"B\" {\n  guarantee \"x\"\n}\n";

/// A project with `spec_root` `spec/`, holding `a.spec` and `sub/b.spec`, compiled.
fn spec_dir_project() -> (tempfile::TempDir, specforge_project::CompiledProject) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        serde_json::json!({"name": "n", "spec_root": "spec", "extensions": ["@specforge/software"]})
            .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("spec/sub")).unwrap();
    std::fs::write(dir.path().join("spec/a.spec"), A).unwrap();
    std::fs::write(dir.path().join("spec/sub/b.spec"), B).unwrap();
    let project = compile_at(
        dir.path(),
        Some(specforge_project::testing::component_runtime()),
    );
    (dir, project)
}

fn kind_and_code(error: &specforge_ops::OpError) -> (specforge_ops::OpErrorKind, &str) {
    (error.kind, &error.code)
}

#[specforge_test_macros::test(
    behavior = "read_a_named_spec_file",
    verify = "every spelling of a spec file under the project root reads as its source key"
)]
fn the_view_reads_every_spelling_into_its_source_key() {
    let (_dir, project) = spec_dir_project();
    let view = ProjectView::of(&project);
    let spec_root = project.environment().spec_root().to_path_buf();
    let absolute = spec_root.join("a.spec");
    for (named, key) in [
        ("spec/a.spec", "spec/a.spec"),
        ("./spec/a.spec", "spec/a.spec"),
        (".//spec/a.spec", "spec/a.spec"),
        (absolute.to_str().unwrap(), "spec/a.spec"),
        ("spec/sub/./b.spec", "spec/sub/b.spec"),
        ("spec/sub//b.spec", "spec/sub/b.spec"),
    ] {
        assert_eq!(view.source(named).unwrap(), key, "{named}");
        assert_eq!(view.source_place(named).unwrap(), key, "{named}");
    }
    // The LSP's and the stub fix's keys read as themselves.
    for key in project.source_keys() {
        assert_eq!(view.source(key).unwrap(), key);
        assert_eq!(view.source_place(key).unwrap(), key);
    }
    assert_eq!(
        view.source_place("spec/new/c.spec").unwrap(),
        "spec/new/c.spec"
    );
}

#[specforge_test_macros::test(
    behavior = "read_a_named_spec_file",
    verify = "a spelling whose key lies outside the project root and the spec root is not_a_source"
)]
fn the_view_refuses_a_spelling_outside_both_roots() {
    let (_dir, project) = spec_dir_project();
    let view = ProjectView::of(&project);
    for named in ["../x.spec", "/elsewhere/x.spec"] {
        for refused in [view.source(named), view.source_place(named)] {
            let error = refused.unwrap_err();
            assert_eq!(
                kind_and_code(&error),
                (specforge_ops::OpErrorKind::InvalidInput, "not_a_source"),
                "{named}"
            );
        }
    }
}

#[specforge_test_macros::test(
    behavior = "read_a_named_spec_file",
    verify = "a read of a file that is no compiled source is file_not_found"
)]
fn the_view_reads_no_source_as_file_not_found() {
    let (_dir, project) = spec_dir_project();
    let view = ProjectView::of(&project);
    for named in ["nope.spec", "", ".", "specforge.json", "notes.txt"] {
        let error = view.source(named).unwrap_err();
        assert_eq!(
            kind_and_code(&error),
            (specforge_ops::OpErrorKind::FileNotFound, "file_not_found"),
            "{named:?}"
        );
    }
    for named in ["notes.txt", "", "node_modules/x.spec"] {
        let error = view.source_place(named).unwrap_err();
        assert_eq!(kind_and_code(&error).1, "not_a_source", "{named:?}");
    }
}

#[specforge_test_macros::test(
    behavior = "read_a_named_spec_file",
    verify = "a new spec file under the spec root is a place, whatever directory it repeats"
)]
fn a_new_file_repeating_a_directory_stays_a_place() {
    let (_dir, project) = spec_dir_project();
    let view = ProjectView::of(&project);
    for named in [
        "spec/new/c.spec",
        "spec/specs/x.spec",
        "spec/sub/spec/z.spec",
        "spec/types/x.spec",
    ] {
        assert_eq!(view.source_place(named).unwrap(), named);
    }
}

#[specforge_test_macros::test(
    behavior = "read_a_named_spec_file",
    verify = "a create into a file that differs from a source only in case is not_a_source, naming the source"
)]
#[ignore = "flipped by T6"]
fn a_new_file_differing_from_a_source_only_in_case_is_not_a_source() {
    let (_dir, project) = spec_dir_project();
    let view = ProjectView::of(&project);
    for (named, source) in [
        ("spec/A.spec", "spec/a.spec"),
        ("spec/SUB/b.spec", "spec/sub/b.spec"),
        ("spec/sub/../A.spec", "spec/a.spec"),
    ] {
        let error = view.source_place(named).unwrap_err();
        assert_eq!(
            kind_and_code(&error),
            (specforge_ops::OpErrorKind::InvalidInput, "not_a_source"),
            "{named}"
        );
        assert_eq!(error.file(), Some(named), "{named}");
        assert!(error.message.contains(&format!("'{source}'")), "{error:?}");
    }
    assert_eq!(view.source_place("spec/new.spec").unwrap(), "spec/new.spec");
}

#[specforge_test_macros::test(
    behavior = "read_a_named_spec_file",
    verify = "a refusal of a named spec file names the file as the request spelled it"
)]
fn a_refusal_of_a_named_file_names_it() {
    let (_dir, project) = spec_dir_project();
    let view = ProjectView::of(&project);
    for named in ["nope.spec", "./gone.spec", "spec/gone.spec", "../x.spec"] {
        assert_eq!(
            view.source(named).unwrap_err().file(),
            Some(named),
            "{named}"
        );
    }
    for named in ["notes.txt", "spec/notes.txt", "../x.spec"] {
        assert_eq!(
            view.source_place(named).unwrap_err().file(),
            Some(named),
            "{named}"
        );
    }
}

#[specforge_test_macros::test(
    behavior = "read_a_named_spec_file",
    verify = "a spec file is named from the project root, as every document names it"
)]
fn the_view_reads_a_spec_file_from_the_project_root() {
    let (_dir, project) = spec_dir_project();
    let view = ProjectView::of(&project);
    for named in ["spec/a.spec", "./spec//a.spec"] {
        assert_eq!(view.source(named).unwrap(), "spec/a.spec", "{named}");
    }
    let missing = view.source("a.spec").unwrap_err();
    assert_eq!(
        kind_and_code(&missing),
        (specforge_ops::OpErrorKind::FileNotFound, "file_not_found")
    );
    assert_eq!(missing.suggestion, None);
    assert_eq!(view.source_place("spec/new.spec").unwrap(), "spec/new.spec");
    let outside = view.source_place("new.spec").unwrap_err();
    assert_eq!(outside.code, "not_a_source");
    assert_eq!(outside.suggestion, None);
}

/// A tempdir holding `proj/` whose spec root is `../shared` (or `../link`, a link to `shared`, when `link`), and
/// `shared/a.spec`, compiled.
fn outside_project(link: bool) -> (tempfile::TempDir, specforge_project::CompiledProject) {
    let dir = tempfile::tempdir().unwrap();
    let t = std::fs::canonicalize(dir.path()).unwrap();
    std::fs::create_dir_all(t.join("proj")).unwrap();
    std::fs::create_dir_all(t.join("shared")).unwrap();
    std::fs::write(t.join("shared/a.spec"), A).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(t.join("shared"), t.join("link")).unwrap();
    let spec_root = if link { "../link" } else { "../shared" };
    std::fs::write(
        t.join("proj/specforge.json"),
        serde_json::json!({"name": "n", "spec_root": spec_root, "extensions": ["@specforge/software"]})
            .to_string(),
    )
    .unwrap();
    let project = compile_at(
        &t.join("proj"),
        Some(specforge_project::testing::component_runtime()),
    );
    (dir, project)
}

#[cfg(unix)]
#[specforge_test_macros::test(
    behavior = "read_a_named_spec_file",
    verify = "a source under a spec root outside the project reads by its key and every spelling of it from the project root"
)]
fn the_view_reads_a_source_under_an_outside_spec_root_by_every_spelling() {
    for link in [false, true] {
        let (dir, project) = outside_project(link);
        let view = ProjectView::of(&project);
        let t = std::fs::canonicalize(dir.path()).unwrap();
        let key = t.join("shared/a.spec").to_str().unwrap().to_owned();
        assert_eq!(
            project.source_keys().into_iter().collect::<Vec<_>>(),
            [key.as_str()]
        );
        let mut spellings = vec![
            key.clone(),
            "../shared/a.spec".to_owned(),
            "./../shared//a.spec".to_owned(),
        ];
        if link {
            spellings.push("../link/a.spec".to_owned());
        }
        for named in &spellings {
            assert_eq!(view.source(named).unwrap(), key, "{named}");
            assert_eq!(view.source_place(named).unwrap(), key, "{named}");
        }
        assert_eq!(
            view.source_place("../shared/new.spec").unwrap(),
            t.join("shared/new.spec").to_str().unwrap()
        );
        let error = view.source("../other/x.spec").unwrap_err();
        assert_eq!(
            kind_and_code(&error),
            (specforge_ops::OpErrorKind::InvalidInput, "not_a_source")
        );
        let error = view.source("a.spec").unwrap_err();
        assert_eq!(error.code, "file_not_found");
    }
}

#[specforge_test_macros::test(
    behavior = "read_a_named_spec_file",
    verify = "a spelling with a .. reads as the file it lands on"
)]
fn a_dotdot_that_stays_inside_reads_as_its_key() {
    let (_dir, project) = spec_dir_project();
    assert_eq!(
        ProjectView::of(&project)
            .source("spec/sub/../a.spec")
            .unwrap(),
        "spec/a.spec"
    );
}
