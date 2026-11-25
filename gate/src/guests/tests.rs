#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use super::*;

#[test]
fn a_guests_config_is_the_store_the_base_and_its_junit_path() {
    let base = "[profile.gate]\nretries = 0\n";
    assert_eq!(
        config(base, Path::new("/r/target/nextest"), "gate", "rust-test").unwrap(),
        "[store]\ndir = \"/r/target/nextest\"\n\n[profile.gate]\nretries = 0\n\
         [profile.gate.junit]\npath = \"junit-guests/rust-test/junit.xml\"\n"
    );
}

/// The store is a TOML basic string: an apostrophe needs nothing, a backslash and a double quote are escaped.
#[test]
fn any_store_path_is_written_as_an_escaped_basic_string() {
    for (store, written) in [
        ("/Users/o'neil/r", "/Users/o'neil/r"),
        (r"C:\r\target", r"C:\\r\\target"),
        ("/r/a\"b", "/r/a\\\"b"),
    ] {
        let text = config("", Path::new(store), "gate", "checks").unwrap();
        assert!(
            text.starts_with(&format!("[store]\ndir = \"{written}\"\n")),
            "{text}"
        );
    }
}

#[test]
fn a_store_with_a_control_character_is_refused() {
    for store in ["/r/a\nb", "/r/a\rb", "/r/a\tb"] {
        let err = config("", Path::new(store), "gate", "checks").unwrap_err();
        assert!(err.contains(&format!("{store:?}")), "{err}");
    }
}

#[test]
fn a_slug_outside_lowercase_digits_and_dashes_is_refused() {
    for slug in ["", "Rust-test", "rust_test", "rust/test", "a b", "x\"y"] {
        let err = config("", Path::new("/r"), "gate", slug).unwrap_err();
        assert!(err.contains(&format!("{slug:?}")), "{err}");
    }
}

#[test]
fn a_guests_slug_is_its_directory_under_extensions() {
    assert_eq!(slug("extensions/rust/test/Cargo.toml"), "rust-test");
    assert_eq!(slug("extensions/checks/Cargo.toml"), "checks");
}

/// The committed base holds the gate and ci profiles and leaves the store, the JUnit path and the root's package
/// overrides out: the gate adds the first two, and nextest refuses the overrides against a guest manifest.
#[test]
fn the_committed_base_holds_the_profiles_and_no_store_junit_or_override() {
    let path = specforge_repo_root::repo_root().join(BASE);
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .collect();
    for table in ["[profile.gate]", "[profile.ci]"] {
        assert!(lines.contains(&table), "{}: no {table}", path.display());
    }
    for line in &lines {
        assert!(
            !line.starts_with("[store]") && !line.contains("junit") && !line.contains("overrides"),
            "{}: {line}",
            path.display()
        );
    }
}
