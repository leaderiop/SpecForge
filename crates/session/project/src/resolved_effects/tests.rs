use super::*;
use serde_json::json;

fn resolve(config: Value) -> ResolvedEffects {
    ResolvedEffects::resolve(Some(&serde_json::to_string_pretty(&config).unwrap()), &[])
}

fn messages(resolved: &ResolvedEffects) -> Vec<&str> {
    resolved
        .diagnostics
        .iter()
        .map(|d| d.message.as_str())
        .collect()
}

fn declaring(capabilities: Value) -> ExtensionDeclaration {
    let mut declaration = ExtensionDeclaration::default();
    declaration.handshake.name = "@acme/x".to_string();
    declaration.capabilities = serde_json::from_value(capabilities).unwrap();
    declaration
}

#[test]
fn an_absent_port_is_open_off_removes_it_and_an_object_keeps_what_it_lists() {
    let resolved = resolve(json!({"effects": {
        "http": {"hosts": ["*.example.com"], "schemes": ["https"], "methods": ["head", "GET"]},
        "git": {"ops": ["read"]},
        "process": "off",
    }}));
    assert!(resolved.diagnostics.is_empty(), "{:?}", messages(&resolved));
    let ceiling = &resolved.ceiling;
    assert_eq!(ceiling.fs, Narrow::Open);
    assert_eq!(ceiling.process, Narrow::Off);
    assert_eq!(
        ceiling.http,
        Narrow::Only(HttpCeiling {
            hosts: Some(vec!["*.example.com".into()]),
            schemes: Some(vec!["https".into()]),
            methods: Some(vec![HttpMethod::Head, HttpMethod::Get]),
        })
    );
    assert_eq!(
        ceiling.git,
        Narrow::Only(GitCeiling {
            ops: Some(vec![GitOp::Read])
        })
    );
}

#[test]
fn each_member_or_item_not_used_as_written_is_e109_at_its_key_and_the_rest_is_kept() {
    let resolved = resolve(json!({
        "effects": {
            "smtp": "off",
            "http": {"hosts": ["crates.io", "not a host"], "verbs": ["GET"], "methods": ["FETCH"]},
            "fs": {"ops": ["read", "delete"], "roots": "spec"},
            "git": {"ops": ["push"]},
            "process": {"argv": ["make"]},
        },
        "roots": {"spec": "docs", "Vendor": "../v", "empty": " ", "vendor": "../vendor"},
    }));
    let found = messages(&resolved);
    for part in [
        "`effects.smtp` in specforge.json is not used: no port is named 'smtp'",
        "`effects.http.verbs` in specforge.json is not used: 'http' takes hosts, schemes, methods",
        "`effects.http.hosts` in specforge.json is not used: 'not a host' is not a host",
        "`effects.http.methods` in specforge.json is not used: 'FETCH' is no HTTP method",
        "`effects.fs.ops` in specforge.json is not used: 'delete' is no file op",
        "`effects.fs.roots` in specforge.json is not used: write a list of strings",
        "`effects.git.ops` in specforge.json is not used: 'push' is no git op",
        "`effects.process` in specforge.json is not used: a command runs as declared or not at all",
        "`roots.spec` in specforge.json is not used: 'spec' is built in",
        "`roots.Vendor` in specforge.json is not used: 'Vendor' is not a word",
        "`roots.empty` in specforge.json is not used: a root is a non-empty path",
    ] {
        assert!(
            found.iter().any(|m| m.starts_with(part)),
            "{part}\n{found:#?}"
        );
    }
    assert_eq!(found.len(), 11, "{found:#?}");
    assert!(
        resolved
            .diagnostics
            .iter()
            .all(|d| d.code == "E109" && d.span.is_some())
    );
    // What is used as written is kept.
    let Narrow::Only(http) = &resolved.ceiling.http else {
        panic!("{:?}", resolved.ceiling.http)
    };
    assert_eq!(http.hosts.as_deref(), Some(&["crates.io".to_string()][..]));
    assert_eq!(http.methods.as_deref(), Some(&[][..]));
    assert_eq!(
        resolved.ceiling.fs,
        Narrow::Only(FsCeiling {
            roots: None,
            ops: Some(vec![FsAccess::Read])
        })
    );
    assert_eq!(resolved.ceiling.process, Narrow::Open);
    let names: Vec<&str> = resolved.roots.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["vendor"]);
}

#[test]
fn a_narrowed_capability_is_w171_at_the_ceiling_key_and_one_with_nothing_left_grants_nothing() {
    let text = serde_json::to_string_pretty(&json!({"effects": {
        "http": {"hosts": ["crates.io"]},
        "process": "off",
    }}))
    .unwrap();
    let declaration = declaring(json!([
        {"name": "links", "why": "check links", "port": "http", "hosts": {"list": ["*.example.com", "crates.io"]}},
        {"name": "tests", "why": "run tests", "call_kinds": ["collect"], "port": "process", "argv": ["make"]},
        {"name": "docs", "why": "read docs", "port": "fs", "ops": ["read"]},
    ]));
    let resolved = ResolvedEffects::resolve(Some(&text), &[declaration]);
    let found = messages(&resolved);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(
        found[0].starts_with("capability 'links' of '@acme/x' is narrowed"),
        "{}",
        found[0]
    );
    assert!(!found[0].ends_with("it grants nothing"), "{}", found[0]);
    assert!(
        found[1].starts_with("capability 'tests' of '@acme/x' is narrowed"),
        "{}",
        found[1]
    );
    assert!(found[1].ends_with("; it grants nothing"), "{}", found[1]);
    let line_of = |key: &str| text.lines().position(|l| l.contains(key)).unwrap() + 1;
    let lines: Vec<usize> = resolved
        .diagnostics
        .iter()
        .map(|d| d.span.as_ref().unwrap().start_line)
        .collect();
    assert_eq!(lines, [line_of("\"hosts\""), line_of("\"process\"")]);
    assert!(resolved.diagnostics.iter().all(|d| d.code == "W171"));
}

#[test]
fn a_root_resolves_inside_or_outside_the_project_and_one_that_is_no_directory_is_e109() {
    let dir = tempfile::TempDir::new().unwrap();
    let project = dir.path().join("p");
    std::fs::create_dir_all(project.join("docs")).unwrap();
    std::fs::create_dir_all(dir.path().join("vendor")).unwrap();
    let text =
        json!({"roots": {"docs": "docs", "vendor": "../vendor", "gone": "../gone"}}).to_string();
    let mut resolved = ResolvedEffects::resolve(Some(&text), &[]);
    resolved.locate(Some(&text), &project, &project);
    let found: Vec<(&str, bool)> = resolved
        .roots
        .iter()
        .map(|r| (r.name.as_str(), r.inside))
        .collect();
    assert_eq!(found, [("docs", true), ("vendor", false)]);
    assert_eq!(
        messages(&resolved),
        ["`roots.gone` in specforge.json is not used: '../gone' is no directory"]
    );
    assert!(resolved.diagnostics[0].span.is_some());
    let roots = resolved.with_named(Roots::new(&project, &project));
    let named: Vec<&str> = roots.named().map(|(name, _)| name).collect();
    assert_eq!(named, ["docs", "vendor"]);
}

#[test]
fn a_ceiling_that_narrows_nothing_declared_reports_nothing() {
    let declaration = declaring(json!([
        {"name": "links", "why": "check links", "port": "http", "hosts": {"list": ["crates.io"]}},
    ]));
    let text = json!({"effects": {"http": {"hosts": ["crates.io", "*.example.com"]}}}).to_string();
    let resolved = ResolvedEffects::resolve(Some(&text), &[declaration]);
    assert!(resolved.diagnostics.is_empty(), "{:?}", messages(&resolved));
}
