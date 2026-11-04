use super::*;

#[test]
fn a_bare_path_takes_the_default_root() {
    let bare = RootedPath::parse("docs/a.md", Root::Spec).unwrap();
    assert_eq!(bare.root, Root::Spec);
    assert_eq!(bare.path, "docs/a.md");
    assert_eq!(bare.written_root, None);
}

#[test]
fn a_prefix_names_spec_project_or_a_named_root() {
    for (text, root, at) in [
        ("<spec>/a.md", Root::Spec, 1..5),
        ("<project>/a.md", Root::Project, 1..8),
        ("<vendor-2>/a.md", Root::Named("vendor-2".into()), 1..9),
    ] {
        let parsed = RootedPath::parse(text, Root::Project).unwrap();
        assert_eq!(parsed.root, root);
        assert_eq!(parsed.path, "a.md");
        assert_eq!(parsed.written_root, Some(at));
    }
}

#[test]
fn a_root_alone_names_the_root_itself() {
    for text in ["<spec>", "<spec>/"] {
        let parsed = RootedPath::parse(text, Root::Project).unwrap();
        assert_eq!((parsed.root, parsed.path.as_str()), (Root::Spec, ""));
    }
}

#[test]
fn an_unclosed_or_misspelled_root_is_a_problem_with_its_range() {
    for text in ["<spec", "<spec>a.md", "<"] {
        assert_eq!(
            RootedPath::parse(text, Root::Project),
            Err(PrefixProblem::UnclosedRoot),
            "{text}"
        );
    }
    for (text, at) in [("<Spec>/a", 1..5), ("<>/a", 1..1), ("<a/b>/c", 1..4)] {
        assert_eq!(
            RootedPath::parse(text, Root::Project),
            Err(PrefixProblem::BadRootName { at }),
            "{text}"
        );
    }
}

#[test]
fn a_backslash_is_reported_not_rewritten() {
    assert_eq!(
        RootedPath::parse("docs\\a.md", Root::Project),
        Err(PrefixProblem::Backslash { at: 4 })
    );
}

#[test]
fn the_parser_never_normalises_dot_dot() {
    for text in ["../a.md", "/etc/passwd", "a/./b/../c"] {
        assert_eq!(RootedPath::parse(text, Root::Spec).unwrap().path, text);
    }
    assert_eq!(
        RootedPath::parse("<spec>/../a", Root::Project)
            .unwrap()
            .path,
        "../a"
    );
}
