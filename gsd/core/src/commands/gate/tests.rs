use super::*;

fn with<'a>(files: &'a [&'a str]) -> impl Fn(&str) -> bool + 'a {
    move |file| files.contains(&file)
}

fn line(text: &str) -> Vec<String> {
    text.split(' ').map(str::to_string).collect()
}

fn resolved(script: &str, json: Option<&str>, files: &[&str]) -> Result<String, String> {
    resolve(script, json, with(files)).map(|words| words.join(" "))
}

fn tree<'a>(files: &'a [&'a str]) -> Tree<'a, impl Fn(&str) -> bool + 'a> {
    Tree {
        has: with(files),
        makefile: None,
        package_json: None,
        python: Vec::new(),
        unread: Vec::new(),
    }
}

fn ask(target: &'static str) -> Ask<'static> {
    Ask {
        target,
        ..Ask::default()
    }
}

#[test]
fn each_lockfile_names_its_manager() {
    for (file, build, test) in [
        ("pnpm-lock.yaml", "pnpm run build", "pnpm test"),
        ("yarn.lock", "yarn run build", "yarn test"),
        ("bun.lock", "bun run build", "bun run test"),
        ("bun.lockb", "bun run build", "bun run test"),
    ] {
        assert_eq!(resolved("build", None, &[file]).as_deref(), Ok(build));
        assert_eq!(resolved("test", None, &[file]).as_deref(), Ok(test));
    }
    // pnpm's lockfile outranks yarn's.
    assert_eq!(
        resolved("build", None, &["yarn.lock", "pnpm-lock.yaml"]).as_deref(),
        Ok("pnpm run build")
    );
}

#[test]
fn the_package_manager_field_beats_a_lockfile() {
    let yarn = r#"{"packageManager": "yarn@4.1.0"}"#;
    assert_eq!(
        resolved("test", Some(yarn), &["pnpm-lock.yaml"]).as_deref(),
        Ok("yarn test")
    );
    // An unknown name, a non-string, a non-object or bad JSON falls through to the lockfiles, as upstream.
    for text in [
        r#"{"packageManager": "deno@2"}"#,
        r#"{"packageManager": 3}"#,
        "[]",
        "{not json",
    ] {
        assert_eq!(
            resolved("build", Some(text), &["bun.lock"]).as_deref(),
            Ok("bun run build"),
            "{text}"
        );
    }
}

#[test]
fn no_file_is_npm() {
    assert_eq!(resolved("build", None, &[]).as_deref(), Ok("npm run build"));
    assert_eq!(resolved("test", Some("{}"), &[]).as_deref(), Ok("npm test"));
    assert_eq!(
        resolved("test:unit", None, &[]).as_deref(),
        Ok("npm run test:unit")
    );
}

#[test]
fn a_script_name_that_is_not_plain_is_refused() {
    for bad in ["", "-x", ":x", "a b", "a;rm", "a$(x)", &"a".repeat(65)] {
        assert!(resolved(bad, None, &[]).is_err(), "{bad:?}");
    }
    assert!(resolved(&"a".repeat(64), None, &[]).is_ok());
    assert!(resolved("_x", None, &[]).is_ok());
}

#[test]
fn the_setting_runs_as_its_own_shell_command() {
    let asked = Ask {
        setting: Some("make -j4 && echo done"),
        ..ask("build")
    };
    let gate = choose(&asked, &tree(&["Cargo.toml"])).expect("an answer");
    assert_eq!(gate.source, "setting");
    assert_eq!(gate.argv, ["bash", "-c", "make -j4 && echo done"]);
    // An empty setting is no setting.
    let asked = Ask {
        setting: Some(""),
        ..ask("build")
    };
    assert_eq!(
        choose(&asked, &tree(&["Cargo.toml"]))
            .expect("an answer")
            .source,
        "cargo"
    );
}

#[test]
fn each_marker_picks_its_runner_in_upstreams_order() {
    let scripts = r#"{"scripts": {"build": "tsc"}}"#;
    for (target, order, other) in [
        (
            "build",
            [
                ("Justfile", "just build"),
                ("Cargo.toml", "cargo build"),
                ("go.mod", "go build ./..."),
                ("pyproject.toml", "python -m py_compile a.py"),
                ("package.json", "npm run build"),
            ],
            "test:\n",
        ),
        (
            "test",
            [
                ("Justfile", "just test"),
                ("package.json", "npm test"),
                ("Cargo.toml", "cargo test"),
                ("go.mod", "go test ./..."),
                ("pyproject.toml", "python -m pytest -x -q --tb=short"),
            ],
            "build:\n",
        ),
    ] {
        let markers: Vec<&str> = order.iter().map(|(marker, _)| *marker).collect();
        // Each runner is the answer once the ones before it in this target's order are gone.
        for (start, (_, want)) in order.iter().enumerate() {
            let t = Tree {
                has: with(&markers[start..]),
                makefile: None,
                package_json: Some(scripts),
                python: vec!["a.py"],
                unread: Vec::new(),
            };
            let gate = choose(&ask(target), &t).expect("an answer");
            assert_eq!(gate.argv, line(want), "{target} from {start}");
        }
        // A rule for this target outranks them all; a rule for the other one does not count.
        let rule = format!("{target}: deps\n");
        let t = Tree {
            has: with(&markers),
            makefile: Some(&rule),
            package_json: Some(scripts),
            python: vec!["a.py"],
            unread: Vec::new(),
        };
        assert_eq!(choose(&ask(target), &t).expect("an answer").source, "make");
        let t = Tree {
            makefile: Some(other),
            ..tree(&[])
        };
        assert_eq!(choose(&ask(target), &t).expect("an answer").source, "none");
    }
}

#[test]
fn an_unread_manifest_answers_unread_where_its_runner_decides() {
    let mut files = tree(&["Cargo.toml"]);
    files.unread = vec!["go.mod"];
    // The runner before it decides: the unread one is never reached.
    let gate = choose(&ask("build"), &files).expect("an answer");
    assert_eq!(gate.source, "cargo");
    // Nothing before it decides.
    let mut files = tree(&[]);
    files.unread = vec!["go.mod"];
    let gate = choose(&ask("build"), &files).expect("an answer");
    assert_eq!(gate.source, UNREAD);
    assert!(gate.argv.is_empty());
    assert!(gate.note.expect("a note").contains("go.mod"));
    // Each runner's manifests, and the lockfile that picks a manager.
    for file in [
        "Makefile",
        "justfile",
        "uv.lock",
        "package.json",
        "yarn.lock",
    ] {
        let mut files = tree(&[]);
        files.unread = vec![file];
        let gate = choose(&ask("test"), &files).expect("an answer");
        assert_eq!(gate.source, UNREAD, "{file}");
    }
}

#[test]
fn the_build_takes_the_package_arm_only_for_a_build_script() {
    for json in [
        r#"{"scripts": {"test": "x"}, "name": "build"}"#,
        r#"{"scripts": {"build": 3}}"#,
    ] {
        let t = Tree {
            has: with(&["package.json"]),
            makefile: None,
            package_json: Some(json),
            python: Vec::new(),
            unread: Vec::new(),
        };
        let gate = choose(&ask("build"), &t).expect("an answer");
        assert_eq!(gate.source, "none", "{json}");
        assert!(gate.argv.is_empty());
    }
    let t = Tree {
        has: with(&["package.json"]),
        makefile: None,
        package_json: Some(r#"{"scripts": {"build": "tsc"}}"#),
        python: Vec::new(),
        unread: Vec::new(),
    };
    let gate = choose(&ask("build"), &t).expect("an answer");
    assert_eq!(gate.argv, ["npm", "run", "build"]);
    assert!(gate.note.expect("a note").contains("(npm run build)"));
}

#[test]
fn python_sources_are_words_not_text() {
    let names: Vec<String> = (0..25).map(|i| format!("m{i};touch pwned.py")).collect();
    let mut t = tree(&["pyproject.toml"]);
    t.python = names.iter().map(String::as_str).collect();
    let gate = choose(&ask("build"), &t).expect("an answer");
    assert_eq!(gate.argv[..3], ["python", "-m", "py_compile"]);
    assert_eq!(gate.argv.len(), 3 + PY_COMPILE_MAX, "20 at most");
    assert_eq!(gate.argv[3], "m0;touch pwned.py", "one word");
    // A leading hyphen is a file, not an option; no source at all is no build.
    t.python = vec!["-x.py"];
    assert_eq!(
        choose(&ask("build"), &t).expect("an answer").argv[3],
        "./-x.py"
    );
    t.python = Vec::new();
    assert_eq!(choose(&ask("build"), &t).expect("an answer").source, "none");
}

#[test]
fn the_regression_scope_runs_pytest_on_the_named_files_or_skips() {
    let named = Ask {
        regression: true,
        files: vec!["tests/a b.py", "-c.py"],
        xcode_project: Some("App.xcodeproj"),
        ..ask("test")
    };
    let gate = choose(&named, &tree(&["requirements.txt"])).expect("an answer");
    assert_eq!(
        gate.argv,
        [
            "python",
            "-m",
            "pytest",
            "-q",
            "--tb=short",
            "--",
            "tests/a b.py",
            "-c.py"
        ],
        "no Xcode in the regression"
    );
    let none = Ask {
        regression: true,
        ..ask("test")
    };
    let gate = choose(&none, &tree(&["pyproject.toml"])).expect("an answer");
    assert!(gate.argv.is_empty());
    assert!(
        gate.note
            .expect("a note")
            .contains("no prior-phase test files")
    );
    // Nothing found says nothing, as upstream's `true` arm.
    let gate = choose(&none, &tree(&[])).expect("an answer");
    assert_eq!((gate.source, gate.note), ("none", None));
}

#[test]
fn uv_lock_runs_pytest_through_uv() {
    let gate = choose(&ask("test"), &tree(&["pyproject.toml", "uv.lock"])).expect("an answer");
    assert_eq!(gate.argv, line("uv run python -m pytest -x -q --tb=short"));
}

#[test]
fn xcode_takes_the_first_scheme_and_the_newest_ios_simulator() {
    let simulators = r#"{"devices": {
        "com.apple.CoreSimulator.SimRuntime.watchOS-99-0": [{"udid": "WATCH", "isAvailable": true}],
        "com.apple.CoreSimulator.SimRuntime.iOS-9-3": [{"udid": "OLD", "isAvailable": true}],
        "com.apple.CoreSimulator.SimRuntime.iOS-18-2": [
            {"udid": "GONE", "isAvailable": false},
            {"udid": "NEW", "isAvailable": true},
            {"udid": "NEXT", "isAvailable": true}
        ]
    }}"#;
    let asked = Ask {
        xcode_project: Some("My App.xcodeproj"),
        xcode_list: Some(r#"{"project": {"schemes": ["Main", "Other"]}}"#),
        simulators: Some(simulators),
        ..ask("build")
    };
    let gate = choose(&asked, &tree(&["Cargo.toml"])).expect("an answer");
    assert_eq!(gate.source, "xcode");
    assert_eq!(
        gate.argv,
        [
            "xcodebuild",
            "build",
            "-project",
            "My App.xcodeproj",
            "-scheme",
            "Main",
            "-destination",
            "id=NEW"
        ]
    );
    // A workspace's scheme serves; a listing that is no JSON drops the scheme, as upstream's empty scheme.
    assert_eq!(
        first_scheme(r#"{"workspace": {"schemes": ["W"]}}"#).as_deref(),
        Some("W")
    );
    let asked = Ask {
        xcode_project: Some("A.xcodeproj"),
        xcode_list: Some("xcodebuild: error"),
        simulators: Some(simulators),
        ..ask("test")
    };
    let gate = choose(&asked, &tree(&[])).expect("an answer");
    assert_eq!(
        gate.argv,
        line("xcodebuild test -project A.xcodeproj -destination id=NEW")
    );
    // No simulator skips the gate, loudly.
    let asked = Ask {
        xcode_project: Some("A.xcodeproj"),
        simulators: Some(
            r#"{"devices": {"com.apple.CoreSimulator.SimRuntime.watchOS-9-0": [{"udid": "W"}]}}"#,
        ),
        ..ask("build")
    };
    let gate = choose(&asked, &tree(&["Cargo.toml"])).expect("an answer");
    assert!(gate.argv.is_empty());
    assert!(
        gate.note
            .expect("a note")
            .contains("workflow.build_command")
    );
}

#[test]
fn a_word_with_a_line_break_is_refused() {
    for word in ["a\nb", "a\rb", ""] {
        let argv = vec!["python".to_string(), word.to_string()];
        assert!(refuse_unreadable(&argv).is_err(), "{word:?}");
    }
    assert!(refuse_unreadable(&line("cargo build")).is_ok());
}
