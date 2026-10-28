//! K11 (ADR 0293): a pass declares the project files it reads and its input carries their facts and capped text; a
//! read file is a check input.

use std::fs;
use std::sync::Arc;

use serde_json::{Value, json};
use specforge_project::testing::{
    GSD_FACTS_SPEC, LIB_RS, TestProject, gsd_facts_probe, gsd_facts_project, gsd_facts_runtime,
    pass_input,
};
use specforge_project::{SharedRuntime, UpdateKind};
use specforge_wasm::testing::InProcessRuntime;

fn facts<'a>(input: &'a Value, read: &str) -> &'a Vec<Value> {
    input["files"][read]["facts"]
        .as_array()
        .unwrap_or_else(|| panic!("no files[{read}] in {input}"))
}

fn fact<'a>(input: &'a Value, read: &str, path: &str) -> &'a Value {
    facts(input, read)
        .iter()
        .find(|f| f["path"] == path)
        .unwrap_or_else(|| panic!("no fact for {path} in {input}"))
}

/// A project of [`GSD_FACTS_SPEC`] whose plan names `files`, and a probe pass declaring `reads`; the input the probe
/// is sent at the first compile.
fn probed(project: TestProject, reads: Value) -> (TestProject, Arc<InProcessRuntime>, Value) {
    let runtime = Arc::new(gsd_facts_probe(reads));
    let _compiled = project.compile_in(runtime.clone());
    let input = pass_input(&runtime, "probe");
    (project, runtime, input)
}

fn plan_naming(files: &[&str]) -> String {
    let names: Vec<String> = files.iter().map(|f| format!("{f:?}")).collect();
    format!(
        "phase p1 \"One\" {{}}\n\nplan pl1 \"Plan\" {{\n  phase p1\n  files_modified [{}]\n}}\n",
        names.join(", ")
    )
}

fn project_of(spec: &str) -> TestProject {
    gsd_facts_project().file("a.spec", spec)
}

const MODIFIED: &str = r#"[{"name": "modified", "field": "plan.files_modified", "text": true}]"#;

fn modified() -> Value {
    serde_json::from_str(MODIFIED).unwrap()
}

#[test]
fn a_pass_reads_a_declared_file_fact_and_text() {
    let project = gsd_facts_project();
    let runtime = Arc::new(gsd_facts_runtime());
    let _compiled = project.compile_in(runtime.clone());

    let input = pass_input(&runtime, "files");
    let lib = fact(&input, "modified", "src/lib.rs");
    assert_eq!(lib["exists"], true);
    assert_eq!(lib["entity"], "pl1");
    assert_eq!(lib["bytes"], LIB_RS.len());
    assert_eq!(lib["lines"], 3);
    assert_eq!(lib["text"], LIB_RS);
    assert!(
        lib["digest"]
            .as_str()
            .is_some_and(|d| d.starts_with("sha256:")),
        "{lib}"
    );
    let gone = fact(&input, "modified", "docs/gone.md");
    assert_eq!(gone["exists"], false);
    assert!(
        gone.get("text").is_none() && gone.get("bytes").is_none(),
        "{gone}"
    );
    // The glob read gives facts and no text.
    let source = fact(&input, "sources", "src/lib.rs");
    assert!(
        source.get("text").is_none() && source.get("entity").is_none(),
        "{source}"
    );
}

#[test]
fn an_undeclared_path_is_absent() {
    // The pass `lists` declares no read, and the pass `files` reads only what it declares.
    let project = gsd_facts_project().file("src/other.rs", "x");
    let runtime = Arc::new(gsd_facts_runtime());
    let _compiled = project.compile_in(runtime.clone());

    assert!(pass_input(&runtime, "lists").get("files").is_none());
    assert!(pass_input(&runtime, "tests").get("files").is_none());
    let input = pass_input(&runtime, "files");
    assert_eq!(input["files"].as_object().unwrap().len(), 2);
    assert!(
        !facts(&input, "modified")
            .iter()
            .any(|f| f["path"] == "src/other.rs"),
        "{input}"
    );
}

#[test]
fn editing_a_read_file_reruns_the_pass() {
    let project = gsd_facts_project();
    let runtime = Arc::new(gsd_facts_runtime());
    let mut session = project.session_in(runtime.clone() as SharedRuntime);
    runtime.clear_calls();

    let lib = project.root().join("src/lib.rs");
    fs::write(&lib, "pub fn two() -> u8 {\n    2\n}\n").unwrap();
    let update = session
        .apply(&session.changes([lib.as_path()]))
        .expect("a file a pass reads is a check input");
    assert_eq!(update.kind, UpdateKind::Checks);

    let input = pass_input(&runtime, "files");
    assert_eq!(
        fact(&input, "modified", "src/lib.rs")["text"],
        json!("pub fn two() -> u8 {\n    2\n}\n"),
        "{input}"
    );

    // The creation of a file the plan named, and had not yet, is an input too.
    runtime.clear_calls();
    let gone = project.root().join("docs/gone.md");
    fs::create_dir_all(gone.parent().unwrap()).unwrap();
    fs::write(&gone, "# gone\n").unwrap();
    session
        .apply(&session.changes([gone.as_path()]))
        .expect("the creation of a read file is a check input");
    assert_eq!(
        fact(&pass_input(&runtime, "files"), "modified", "docs/gone.md")["exists"],
        true
    );
}

#[test]
fn a_new_file_under_a_globbed_directory_reruns_the_pass() {
    let project = gsd_facts_project();
    fs::create_dir_all(project.root().join("src/deep")).unwrap();
    let runtime = Arc::new(gsd_facts_runtime());
    let mut session = project.session_in(runtime.clone() as SharedRuntime);

    // Beside the files the glob matched, and deeper: each listing of a walked directory is recorded.
    for new in ["src/new.rs", "src/deep/new.rs"] {
        runtime.clear_calls();
        let new = project.root().join(new);
        fs::write(&new, "fn n() {}\n").unwrap();
        let update = session
            .apply(&session.changes([new.as_path()]))
            .expect("a file under a globbed directory is a check input");
        assert_eq!(update.kind, UpdateKind::Checks);
        let input = pass_input(&runtime, "files");
        let name = new.strip_prefix(project.root()).unwrap().to_str().unwrap();
        assert_eq!(fact(&input, "sources", name)["exists"], true);
    }
}

#[test]
fn a_path_outside_the_project_is_outside_project() {
    let (_, _, input) = probed(
        project_of(&plan_naming(&[
            "../outside.txt",
            "/etc/hosts",
            "src/../src/lib.rs",
        ])),
        modified(),
    );

    for path in ["../outside.txt", "/etc/hosts"] {
        let f = fact(&input, "modified", path);
        assert_eq!(f["error"], "outside_project", "{f}");
        assert_eq!(f["exists"], false);
        assert!(f.get("digest").is_none());
    }
    // A path that climbs and returns is inside, and is shown normalised.
    assert_eq!(fact(&input, "modified", "src/lib.rs")["exists"], true);
}

#[test]
fn a_file_over_the_cap_has_facts_and_no_text() {
    let project = project_of(&plan_naming(&["big.txt"])).file("big.txt", "a".repeat(70_000));
    let (_, _, input) = probed(project, modified());

    let f = fact(&input, "modified", "big.txt");
    assert_eq!(
        (f["bytes"].as_u64(), f["error"].as_str()),
        (Some(70_000), Some("too_large"))
    );
    assert!(f.get("text").is_none() && f["digest"].is_string(), "{f}");
}

#[test]
fn a_non_utf8_file_has_facts_and_not_utf8() {
    let project = project_of(&plan_naming(&["bin.dat"])).file("bin.dat", [0xff_u8, 0xfe, 0x00]);
    let (_, _, input) = probed(project, modified());

    let f = fact(&input, "modified", "bin.dat");
    assert_eq!(
        (f["bytes"].as_u64(), f["error"].as_str()),
        (Some(3), Some("not_utf8"))
    );
    assert!(f.get("text").is_none());
}

#[test]
fn a_glob_over_the_file_cap_is_truncated() {
    let project = gsd_facts_project();
    for n in 0..2005 {
        project.write(&format!("many/f{n:04}.txt"), "x");
    }
    let (_, _, input) = probed(project, json!([{"name": "many", "glob": "many/*.txt"}]));

    assert_eq!(input["files"]["many"]["truncated"], true);
    let found = facts(&input, "many");
    assert_eq!(found.len(), 2000);
    assert_eq!(found[0]["path"], "many/f0000.txt");
}

#[test]
fn the_text_budget_of_a_read_is_one_mib() {
    let names: Vec<String> = (0..20).map(|n| format!("t{n:02}.txt")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut project = project_of(&plan_naming(&refs));
    for name in &names {
        project = project.file(name, "a".repeat(60_000));
    }
    let (_, _, input) = probed(project, modified());

    let found = facts(&input, "modified");
    let with_text = found.iter().filter(|f| f.get("text").is_some()).count();
    let over_budget = found.iter().filter(|f| f["error"] == "budget").count();
    // 17 x 60,000 bytes fit in 1 MiB; the 18th does not.
    assert_eq!((with_text, over_budget), (17, 3), "{found:?}");
}

#[test]
fn a_field_read_names_the_entity_and_dedupes_paths() {
    let spec = format!(
        "{}\nplan pl2 \"Two\" {{\n  phase p1\n  files_modified [\"src/lib.rs\"]\n}}\n",
        plan_naming(&["src/lib.rs", "./src/lib.rs"])
    );
    let (_, _, input) = probed(project_of(&spec), modified());

    let found: Vec<(&str, &str)> = facts(&input, "modified")
        .iter()
        .map(|f| (f["entity"].as_str().unwrap(), f["path"].as_str().unwrap()))
        .collect();
    assert_eq!(found, [("pl1", "src/lib.rs"), ("pl2", "src/lib.rs")]);
}

#[test]
fn a_glob_skips_build_and_vcs_directories_unless_it_names_them() {
    let project = gsd_facts_project()
        .file("target/out.rs", "x")
        .file("node_modules/pkg/a.rs", "x")
        .file(".git/x.rs", "x");
    let (_, _, input) = probed(
        project,
        json!([{"name": "all", "glob": "**/*.rs"}, {"name": "named", "glob": "target/*.rs"}]),
    );

    let all: Vec<&str> = facts(&input, "all")
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(all, ["src/lib.rs"]);
    assert_eq!(facts(&input, "named")[0]["path"], "target/out.rs");
}

#[test]
fn a_malformed_read_is_reported_and_served_empty() {
    let project = gsd_facts_project();
    let runtime = Arc::new(gsd_facts_probe(json!([
        {"name": "Bad Name", "glob": "src/*.rs"},
        {"name": "both", "field": "plan.files_modified", "glob": "src/*.rs"},
        {"name": "outside", "glob": "../x"},
        {"name": "absolute", "glob": "/etc/*"},
        {"name": "deep", "glob": "**/**/**/**/**/x"},
        {"name": "ghost", "field": "plan.no_such_field"},
        {"name": "fine", "glob": "src/*.rs"},
        {"name": "fine", "glob": "src/lib.rs"},
    ])));
    let compiled = project.compile_in(runtime.clone());

    let input = pass_input(&runtime, "probe");
    let served: Vec<&str> = input["files"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    // `ghost` is well formed (no entity writes it): served empty; the repeated `fine` is not served twice.
    assert_eq!(served, ["fine", "ghost"], "{input}");
    assert!(facts(&input, "ghost").is_empty());
    let messages: Vec<String> = compiled
        .diagnostics()
        .iter()
        .filter(|d| d.code == "W021" && d.message.contains("read"))
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(messages.len(), 7, "{messages:#?}");
    assert!(GSD_FACTS_SPEC.contains("plan"));
}

#[test]
fn an_absolute_path_is_outside_project_even_under_the_root() {
    let project = gsd_facts_project();
    let inner = project.root().join("src/lib.rs");
    let project = project.file("a.spec", plan_naming(&[inner.to_str().unwrap()]));
    let (_, _, input) = probed(project, modified());

    let f = &facts(&input, "modified")[0];
    assert_eq!(
        (f["error"].as_str(), f["exists"].as_bool()),
        (Some("outside_project"), Some(false)),
        "{f}"
    );
    assert!(f.get("text").is_none() && f.get("digest").is_none());
}

#[test]
fn a_glob_skips_the_usual_output_directories() {
    let mut project = gsd_facts_project();
    for dir in ["dist", "vendor", ".venv", ".next"] {
        project = project.file(&format!("{dir}/x.rs"), "x");
    }
    let (_, _, input) = probed(project, json!([{"name": "all", "glob": "**/*.rs"}]));

    let all: Vec<&str> = facts(&input, "all")
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(all, ["src/lib.rs"]);
}

#[test]
fn a_walk_that_visits_too_many_entries_is_truncated() {
    let project = gsd_facts_project();
    for n in 0..20_005 {
        project.write(&format!("many/f{n:05}.txt"), "x");
    }
    // Nothing matches, so the match cap never ends it: only the visit budget does.
    let (_, _, input) = probed(project, json!([{"name": "none", "glob": "many/*.zzz"}]));

    assert_eq!(input["files"]["none"]["truncated"], true);
    assert!(facts(&input, "none").is_empty());
}

#[test]
fn editing_a_file_the_glob_does_not_match_does_not_rerun_the_pass() {
    let project = gsd_facts_project()
        .file("src/README.txt", "one")
        .file("other.txt", "one");
    let runtime = Arc::new(gsd_facts_runtime());
    let mut session = project.session_in(runtime.clone() as SharedRuntime);

    for name in ["src/README.txt", "other.txt"] {
        let path = project.root().join(name);
        fs::write(&path, "two, longer").unwrap();
        assert!(
            session.apply(&session.changes([path.as_path()])).is_none(),
            "{name} matches no read and is not an input"
        );
    }
}

#[cfg(unix)]
mod links {
    use std::os::unix::fs::symlink;

    use super::*;

    fn outside() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("secret.txt"), "top secret").unwrap();
        dir
    }

    #[test]
    fn a_link_out_of_the_project_is_outside_project() {
        let out = outside();
        let project = project_of(&plan_naming(&["leak.txt", "dirlink/secret.txt"]));
        symlink(
            out.path().join("secret.txt"),
            project.root().join("leak.txt"),
        )
        .unwrap();
        symlink(out.path(), project.root().join("dirlink")).unwrap();
        let (_, _, input) = probed(
            project,
            json!([
                {"name": "modified", "field": "plan.files_modified", "text": true},
                {"name": "all", "glob": "**/*.txt", "text": true},
            ]),
        );

        for read in ["modified", "all"] {
            for f in facts(&input, read) {
                assert!(f.get("text").is_none() && f.get("digest").is_none(), "{f}");
                assert_eq!(f["error"], "outside_project", "{f}");
            }
        }
        assert!(!facts(&input, "modified").is_empty());
        // The linked directory is not entered, so only the linked file itself is named.
        let all: Vec<&str> = facts(&input, "all")
            .iter()
            .map(|f| f["path"].as_str().unwrap())
            .collect();
        assert_eq!(all, ["leak.txt"]);
    }

    #[test]
    fn a_link_to_a_directory_is_not_entered() {
        let project = gsd_facts_project();
        symlink(".", project.root().join("again")).unwrap();
        symlink("src", project.root().join("alias")).unwrap();
        let (_, _, input) = probed(project, json!([{"name": "all", "glob": "**/*.rs"}]));

        let all: Vec<&str> = facts(&input, "all")
            .iter()
            .map(|f| f["path"].as_str().unwrap())
            .collect();
        assert_eq!(all, ["src/lib.rs"]);
    }

    #[test]
    fn a_literal_prefix_that_is_a_link_out_is_not_walked() {
        let out = outside();
        let project = gsd_facts_project();
        symlink(out.path(), project.root().join("docs")).unwrap();
        let (_, _, input) = probed(project, json!([{"name": "all", "glob": "docs/*.txt"}]));

        assert!(facts(&input, "all").is_empty(), "{input}");
    }
}

#[test]
fn passes_with_equal_reads_share_one_walk() {
    use specforge_project::file_reads::{FileSource, Memo};
    use specforge_protocol_types::FileRead;
    use specforge_registry::{EntitySnapshot, MemoryDisk};

    let disk = MemoryDisk::new().with_file("/p/src/a.rs", b"a");
    let source = FileSource {
        disk: &disk,
        project_root: std::path::Path::new("/p"),
        spec_root: std::path::Path::new("/p"),
    };
    let read = |name: &str| FileRead {
        name: name.into(),
        glob: Some("src/*.rs".into()),
        ..FileRead::default()
    };
    let entities = EntitySnapshot::default();
    let mut memo = Memo::default();

    let first = memo.read(&[read("a")], &entities, &source);
    let walked = disk.asked().len();
    assert_eq!(memo.read(&[read("a")], &entities, &source), first);
    assert_eq!(disk.asked().len(), walked, "an equal read walks nothing");
    memo.read(&[read("b")], &entities, &source);
    assert!(disk.asked().len() > walked, "a different read does");
}
