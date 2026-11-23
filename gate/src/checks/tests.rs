use super::*;
use crate::repo::{outside_workspace_patterns, this_repo};

fn check(name: &str) -> &'static Check {
    CHECKS
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no check named {name}"))
}

fn selection(name: &str, mode: Mode, paths: &[&str]) -> Selection {
    check(name)
        .selection(mode, &this_repo(), &ChangeSet::of(paths.iter().copied()))
        .unwrap()
}

fn only(names: &[&str]) -> Selection {
    Selection::Only(names.iter().map(|n| n.to_string()).collect())
}

#[test]
fn every_check_has_a_unique_name_a_fix_and_known_after_names() {
    let mut seen = BTreeSet::new();
    for check in CHECKS {
        assert!(seen.insert(check.name), "duplicate check {}", check.name);
        assert!(
            check
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '-'),
            "{} is not kebab-case",
            check.name
        );
        assert!(
            !check.fix.is_empty() && !check.proves.is_empty(),
            "{} lacks a fix or a proof",
            check.name
        );
    }
    for check in CHECKS {
        for after in check.after {
            let position = CHECKS.iter().position(|c| c.name == *after);
            let own = CHECKS.iter().position(|c| c.name == check.name);
            assert!(
                position < own && position.is_some(),
                "{} runs after {after}, which does not precede it",
                check.name
            );
        }
    }
}

#[test]
fn ci_and_full_select_the_same_checks() {
    let repo = this_repo();
    let changes = ChangeSet::of(["README.md"]);
    for check in CHECKS {
        assert_eq!(
            check.selection(Mode::Ci, &repo, &changes).unwrap(),
            check.selection(Mode::Full, &repo, &changes).unwrap(),
            "{}",
            check.name
        );
    }
}

#[test]
fn the_hook_reaches_no_check_that_compiles_and_not_the_builtins() {
    let repo = this_repo();
    let everything = ChangeSet::of(["crates/surfaces/lsp/src/lib.rs", "a.spec", "Cargo.toml"]);
    for check in CHECKS {
        let selected = check.selection(Mode::Hook, &repo, &everything).unwrap() != Selection::Skip;
        if selected {
            assert!(
                !check.compiles,
                "the hook reaches {}, which compiles the workspace",
                check.name
            );
            assert!(
                check.reach == Reach::Hook,
                "the hook reaches {}",
                check.name
            );
        }
    }
    assert_eq!(
        selection("builtins", Mode::Hook, &["extensions/product/src/lib.rs"]),
        Selection::Skip
    );
}

#[test]
fn full_reaches_every_check_but_builtins_verify() {
    let repo = this_repo();
    let nothing = ChangeSet::default();
    for check in CHECKS {
        let selected = check.selection(Mode::Full, &repo, &nothing).unwrap();
        let expected = if check.name == "builtins-verify" {
            Selection::Skip
        } else {
            Selection::Everything
        };
        assert_eq!(selected, expected, "{}", check.name);
    }
}

#[test]
fn bootstrap_reaches_only_builtins_verify() {
    let repo = this_repo();
    let nothing = ChangeSet::default();
    for check in CHECKS {
        let selected =
            check.selection(Mode::Bootstrap, &repo, &nothing).unwrap() != Selection::Skip;
        assert_eq!(selected, check.name == "builtins-verify", "{}", check.name);
    }
}

#[test]
fn an_unformatted_guest_is_judged_by_the_hook() {
    // §3.2: the hook ran `cargo fmt --all`, which skips every crate outside the workspace.
    assert_eq!(
        selection("rustfmt", Mode::Hook, &["extensions/governance/src/lib.rs"]),
        Selection::Everything
    );
    assert_eq!(
        selection("rustfmt", Mode::Hook, &["docs/guide.md"]),
        Selection::Skip
    );
    let manifests = outside_workspace_patterns()
        .iter()
        .flat_map(|pattern| crate::repo::expand(&this_repo().root, pattern))
        .count();
    assert_eq!(
        manifests, 31,
        "fourteen guests, @rust/coverage (not vendored yet), the @code, @rust and @javascript \
         extension libraries (seven, one, one), four fixtures, the marketing example's extension, the @gsd \
         workspace and its Rust parity fixture (the ledger)"
    );
}

#[test]
fn an_examples_extension_is_format_checked() {
    assert!(
        crate::steps::outside_workspace(&this_repo())
            .iter()
            .any(|manifest| manifest == "examples/marketing/extension/Cargo.toml")
    );
}

#[test]
fn a_staged_starter_template_is_not_format_checked() {
    // §3.5: a template a blob embeds, and a parser fixture, are formatted or not by their tests' needs.
    for path in [
        "extensions/software/src/starter.spec",
        "crates/kernel/parser/tests/fixtures/lexemes.spec",
    ] {
        assert_eq!(
            selection("spec-format", Mode::Hook, &[path]),
            Selection::Skip,
            "{path}"
        );
    }
    assert_eq!(
        selection(
            "spec-format",
            Mode::Hook,
            &["crates/surfaces/cli/tests/fixtures/parity/clean/spec/a.spec"]
        ),
        Selection::Skip
    );
    assert_eq!(
        selection(
            "spec-format",
            Mode::Hook,
            &["spec/governance/invariants.spec"]
        ),
        only(&["spec/governance/invariants.spec"])
    );
    assert_eq!(
        selection(
            "spec-format",
            Mode::Hook,
            &[
                "examples/shop/spec/a.spec",
                "examples/todo-app/spec/b/c.spec"
            ]
        ),
        only(&[
            "examples/shop/spec/a.spec",
            "examples/todo-app/spec/b/c.spec"
        ])
    );
    assert_eq!(
        selection(
            "spec-format",
            Mode::Hook,
            &["crates/pipeline/formatter/src/lib.rs"]
        ),
        Selection::Everything
    );
}

#[test]
fn the_gsd_fixture_specs_are_formatted_too() {
    let paths = [
        "gsd/core/tests/projects/roadmap/spec/v1.0.spec",
        "gsd/parity/fixtures/taskboard/spec/project.spec",
    ];
    assert_eq!(selection("spec-format", Mode::Hook, &paths), only(&paths));
}

#[test]
fn a_grammar_edit_is_judged_by_the_hook() {
    assert_eq!(
        selection(
            "grammar",
            Mode::Hook,
            &["crates/kernel/tree-sitter/grammar.js"]
        ),
        Selection::Everything
    );
    assert_eq!(
        selection(
            "grammar",
            Mode::Hook,
            &["crates/kernel/tree-sitter/src/parser.c"]
        ),
        Selection::Everything
    );
    assert_eq!(
        selection("grammar", Mode::Hook, &["crates/surfaces/lsp/src/lib.rs"]),
        Selection::Skip
    );
}

#[test]
fn quick_runs_the_guests_and_the_specs_when_a_rule_crate_changes() {
    // gate.sh:227 ran `specforge check` only for the emitter, and no guest for an SDK edit.
    let path = "crates/pipeline/registry/src/checks/values.rs";
    assert_eq!(
        selection("guests", Mode::Quick, &[path]),
        Selection::Everything
    );
    assert_ne!(selection("specs", Mode::Quick, &[path]), Selection::Skip);
    assert_eq!(
        selection(
            "guests",
            Mode::Quick,
            &["crates/runtime/extension-sdk/src/lib.rs"]
        ),
        Selection::Everything
    );
    assert_eq!(
        selection("specs", Mode::Quick, &["docs/guide.md"]),
        Selection::Skip
    );
}

#[test]
fn a_test_file_or_a_spec_link_attribute_or_a_spec_runs_links() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("crates/a/src")).unwrap();
    for (file, text) in [
        (
            "linked.rs",
            "#[specforge_test(requirement = \"X-01\")]\nfn t() {}\n",
        ),
        (
            "spaced.rs",
            "#[ spec (requirement = \"X-01\")]\nfn t() {}\n",
        ),
        (
            "pathed.rs",
            "#[specforge_test_macros::test(requirement = \"X-01\")]\nfn t() {}\n",
        ),
        (
            "plain.rs",
            "#[test]\nfn specforge_test() {}\n#[spec_helper(x)]\nfn f() {}\nlet verify = 1;\n",
        ),
    ] {
        std::fs::write(dir.path().join("crates/a/src").join(file), text).unwrap();
    }
    let repo = Repo {
        root: dir.path().to_path_buf(),
        target_dir: dir.path().join("target"),
        members: vec![],
    };
    let run = |paths: &[&str]| {
        check("links")
            .selection(Mode::Hook, &repo, &ChangeSet::of(paths.iter().copied()))
            .unwrap()
    };
    for linked in ["linked.rs", "spaced.rs", "pathed.rs"] {
        assert_eq!(
            run(&[&format!("crates/a/src/{linked}")]),
            Selection::Everything,
            "{linked}"
        );
    }
    assert_eq!(run(&["crates/a/src/plain.rs"]), Selection::Skip);
    assert_eq!(run(&["crates/a/tests/t.rs"]), Selection::Everything);
    assert_eq!(run(&["tests/t.rs"]), Selection::Everything);
    assert_eq!(run(&["spec/glossary.spec"]), Selection::Everything);
    assert_eq!(run(&["README.md"]), Selection::Skip);
}

#[test]
fn packages_map_changed_paths_to_members_and_manifests_to_everything() {
    assert_eq!(
        selection(
            "tests",
            Mode::Quick,
            &[
                "crates/surfaces/lsp/src/lib.rs",
                "crates/surfaces/lsp/tests/a.rs",
                "xtask/src/x.rs"
            ]
        ),
        only(&["specforge-lsp", "xtask"])
    );
    assert_eq!(
        selection("build", Mode::Quick, &["Cargo.toml"]),
        Selection::Everything
    );
    assert_eq!(
        selection("build", Mode::Quick, &["Cargo.lock"]),
        Selection::Everything
    );
    assert_eq!(
        selection("tests", Mode::Quick, &[".config/nextest.toml"]),
        Selection::Everything
    );
    assert_eq!(
        selection(
            "build",
            Mode::Quick,
            &["extensions/governance/src/lib.rs", "README.md"]
        ),
        Selection::Skip
    );
}

#[test]
fn a_file_outside_the_workspace_that_a_member_tests_read_selects_that_member() {
    let golden = "gsd/core/tests/golden/kinds.json";
    // `gsd/` is its own workspace: the golden belongs to no member, yet two members' tests read it.
    assert!(this_repo().member_of(golden).is_none());
    for check in ["build", "tests"] {
        assert_eq!(
            selection(check, Mode::Quick, &[golden]),
            only(&["specforge-component", "xtask"]),
            "{check}"
        );
    }
    assert_eq!(
        selection(
            "tests",
            Mode::Quick,
            &[golden, "crates/surfaces/lsp/src/lib.rs"]
        ),
        only(&["specforge-component", "specforge-lsp", "xtask"])
    );
    // Another file of `gsd/` is read by no member's test.
    assert_eq!(
        selection("tests", Mode::Quick, &["gsd/core/src/lib.rs"]),
        Selection::Skip
    );
    // A fixture inside one member that other members' tests read (ADR 0374 D5's chain: the CLI's proof test and the
    // gate's JUnit merge) selects them beside its own member; another fixture of the same member selects only it.
    for fixture in [
        "xtask/tests/fixtures/requirement-links/spec/project.spec",
        "xtask/tests/fixtures/requirement-links/junit/part-1.xml",
    ] {
        assert_eq!(
            selection("tests", Mode::Quick, &[fixture]),
            only(&["gate", "specforge-cli", "xtask"]),
            "{fixture}"
        );
    }
    assert_eq!(
        selection(
            "tests",
            Mode::Quick,
            &["xtask/tests/fixtures/absorb/specforge.json"]
        ),
        only(&["xtask"])
    );
    // The file or directory exists and the readers it names are members of the real workspace (`this_repo` lists a
    // few).
    let repo = Repo::discover().unwrap();
    for (path, readers) in READ_FROM_OUTSIDE {
        let there = match path.strip_suffix("/**") {
            Some(dir) => repo.root.join(dir).is_dir(),
            None => repo.root.join(path).is_file(),
        };
        assert!(there, "{path}");
        for reader in *readers {
            assert!(repo.members.iter().any(|m| &m.name == reader), "{reader}");
        }
    }
}

/// The gate's tests read the workflows (their jobs' timeouts and budgets, ADR 0375 D3): a pull request that only
/// edits a workflow runs them.
#[test]
fn a_change_to_a_workflow_selects_the_gates_tests() {
    assert_eq!(
        selection("tests", Mode::Quick, &[".github/workflows/ci.yml"]),
        only(&["gate"])
    );
}

/// The root spec, its project file and the frozen pre-swap spec (ADR 0374 D8) belong to no member: a change to one
/// selects the members whose tests read it, and another project's spec selects none.
#[test]
fn a_change_to_the_root_spec_or_its_frozen_copy_selects_the_tests_that_read_it() {
    let (live, frozen) = (
        "spec/specforge.spec",
        "fixtures/old-spec/spec/glossary.spec",
    );
    for path in [live, "specforge.json", frozen] {
        assert!(this_repo().member_of(path).is_none(), "{path}");
    }
    assert_eq!(
        selection("tests", Mode::Quick, &[live]),
        only(&[
            "specforge-cli",
            "specforge-formatter",
            "specforge-ops",
            "specforge-parser",
            "specforge-project",
        ])
    );
    assert_eq!(
        selection("tests", Mode::Quick, &["specforge.json"]),
        only(&["specforge-cli", "specforge-ops"])
    );
    assert_eq!(
        selection("tests", Mode::Quick, &[frozen]),
        only(&[
            "specforge-component",
            "specforge-formatter",
            "specforge-lsp",
            "specforge-ops",
            "specforge-parser",
            "specforge-project",
            "tree-sitter-specforge",
        ])
    );
    assert_eq!(
        selection("tests", Mode::Quick, &["integrations/rust/spec/a.spec"]),
        Selection::Skip,
        "another project's spec is not the root's"
    );
}

#[test]
fn doctests_run_for_the_changed_members_that_hold_one() {
    assert_eq!(
        selection(
            "doctests",
            Mode::Quick,
            &["crates/leaf/diagnostics/src/lib.rs"]
        ),
        only(&["specforge-diagnostics"])
    );
    assert_eq!(
        selection("doctests", Mode::Quick, &["crates/surfaces/lsp/src/lib.rs"]),
        Selection::Skip
    );
}

#[test]
fn quick_runs_the_builtins_check_when_a_recorded_input_changes() {
    // §3.1: quick passed all four of these edits while `build-blobs --check` failed on them.
    for path in [
        "crates/leaf/coverage/src/lib.rs",
        "crates/leaf/protocol-types/src/lib.rs",
        "Cargo.toml",
        "crates/runtime/extension-sdk/src/lib.rs",
    ] {
        assert_ne!(
            selection("builtins", Mode::Quick, &[path]),
            Selection::Skip,
            "{path}"
        );
    }
    assert_eq!(
        selection("builtins", Mode::Quick, &["crates/surfaces/lsp/src/lib.rs"]),
        Selection::Skip
    );
    assert_eq!(
        selection(
            "builtins",
            Mode::Quick,
            &["crates/leaf/coverage/src/lib.rs"]
        ),
        only(&[
            "extensions/formal",
            "extensions/product",
            "extensions/testing"
        ])
    );
}

#[test]
fn quick_runs_the_guests_when_an_examples_extension_changes() {
    assert_ne!(
        selection(
            "guests",
            Mode::Quick,
            &["examples/marketing/extension/src/lib.rs"]
        ),
        Selection::Skip
    );
    assert_eq!(
        selection(
            "guests",
            Mode::Quick,
            &["examples/marketing/spec/brand.spec"]
        ),
        Selection::Skip
    );
}

#[test]
fn the_list_is_pinned() {
    insta::assert_snapshot!(render_list());
}

#[test]
fn published_schemas_run_when_a_writer_or_a_declaration_changes() {
    for path in [
        "crates/pipeline/emitter/src/publish.rs",
        "extensions/formal/src/declaration.rs",
        "spec/behaviors/output-schema.spec",
    ] {
        assert_eq!(
            selection("published-schemas", Mode::Quick, &[path]),
            Selection::Everything,
            "{path}"
        );
    }
    assert_eq!(
        selection("published-schemas", Mode::Quick, &["README.md"]),
        Selection::Skip
    );
    assert_eq!(
        selection("published-schemas", Mode::Hook, &["spec/x.spec"]),
        Selection::Skip,
        "a quick check is not a hook one"
    );
}

#[test]
fn no_trigger_assumes_how_deep_a_crate_sits() {
    for check in CHECKS {
        if let Trigger::Paths(patterns) = &check.trigger {
            for pattern in *patterns {
                assert!(
                    !pattern.starts_with("crates/*/"),
                    "{}: `{pattern}` reaches only a crate directly under crates/; write `crates/**/`",
                    check.name
                );
            }
        }
    }
    for nested in [
        "crates/kernel/parser/src/lib.rs",
        "crates/kernel/parser/Cargo.toml",
    ] {
        assert_ne!(
            selection("specs", Mode::Quick, &[nested]),
            Selection::Skip,
            "{nested}"
        );
    }
    assert_eq!(
        selection(
            "zero-entity",
            Mode::Hook,
            &["crates/kernel/parser/src/lib.rs"]
        ),
        Selection::Everything
    );
}

#[test]
fn the_layer_check_runs_when_a_manifest_or_the_rule_changes() {
    for path in [
        "Cargo.toml",
        "crates/app/ops/Cargo.toml",
        "crates/kernel/tree-sitter/Cargo.toml",
        "gate/src/layers.rs",
    ] {
        assert_eq!(
            selection("layers", Mode::Hook, &[path]),
            Selection::Everything,
            "{path}"
        );
    }
    for path in [
        "crates/app/ops/src/lib.rs",
        "extensions/product/Cargo.toml",
        "gate/src/checks.rs",
    ] {
        assert_eq!(
            selection("layers", Mode::Hook, &[path]),
            Selection::Skip,
            "{path}"
        );
    }
}

#[test]
fn file_size_runs_in_the_hook_for_any_rust_file_or_the_baseline() {
    for path in [
        "crates/app/ops/src/analyze.rs",
        "extensions/product/src/queries.rs",
        "gate/src/steps.rs",
        "gate/baseline.json",
    ] {
        assert_eq!(
            selection(crate::size_cap::CHECK, Mode::Hook, &[path]),
            Selection::Everything,
            "{path}"
        );
    }
    assert_eq!(
        selection(
            crate::size_cap::CHECK,
            Mode::Hook,
            &["README.md", "spec/a.spec"]
        ),
        Selection::Skip
    );
    assert!(
        check(crate::size_cap::CHECK)
            .proves
            .contains(&format!("over {} lines", crate::size_cap::CAP))
    );
}

#[test]
fn the_crate_map_runs_when_what_it_is_drawn_from_changes() {
    let repo = this_repo();
    let manifest = crate::crate_map::manifest_of(&repo.members[0]);
    for path in [
        "ARCHITECTURE.md",
        "crates/README.md",
        "Cargo.toml",
        manifest.as_str(),
    ] {
        assert_eq!(
            selection("crate-map", Mode::Hook, &[path]),
            Selection::Everything,
            "{path}"
        );
    }
    assert_eq!(
        selection("crate-map", Mode::Hook, &["docs/guide.md"]),
        Selection::Skip
    );
}
