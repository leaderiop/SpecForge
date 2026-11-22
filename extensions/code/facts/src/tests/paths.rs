//! A report's file path made project-relative, and a report's `file:` URI decoded to a path.

use crate::*;

#[test]
fn a_windows_file_under_a_verbatim_cwd_is_relative() {
    let file = r"C:\work\app\test\a.test.js";
    assert_eq!(
        relative(file, r"\\?\C:\work\app").as_deref(),
        Some("test/a.test.js")
    );
    assert_eq!(
        relative(file, r"\\?\c:\work\app\").as_deref(),
        Some("test/a.test.js")
    );
    assert_eq!(
        relative(r"\\server\share\app\t.js", r"\\?\UNC\server\share\app").as_deref(),
        Some("t.js")
    );
    assert_eq!(relative(file, r"\\?\D:\work\app"), None);
}

#[test]
fn a_private_var_alias_is_relative() {
    let file = "/var/folders/x/T/p/test/a.test.js";
    assert_eq!(
        relative(file, "/private/var/folders/x/T/p").as_deref(),
        Some("test/a.test.js")
    );
    assert_eq!(
        relative("/private/tmp/p/test/a.test.js", "/tmp/p").as_deref(),
        Some("test/a.test.js")
    );
    assert_eq!(relative("/variable/p/a.js", "/private/variable/p"), None);
}

#[test]
fn a_file_outside_cwd_is_none() {
    assert_eq!(relative("/elsewhere/test/a.test.js", "/project"), None);
    assert_eq!(relative("/project-two/a.test.js", "/project"), None);
    assert_eq!(relative("/project", "/project"), None);
    assert_eq!(
        relative("./test/a.test.js", "/project").as_deref(),
        Some("test/a.test.js")
    );
    assert_eq!(
        relative("/project/a.js", "/").as_deref(),
        Some("project/a.js")
    );
}

#[test]
fn a_file_that_climbs_out_through_dot_dot_is_outside() {
    for escaping in [
        "../x.test.js",
        "./test/../../x.test.js",
        r"..\x.test.js",
        "/project/../x.test.js",
        "/project/test/../../elsewhere/x.test.js",
    ] {
        assert_eq!(relative(escaping, "/project"), None, "{escaping}");
    }
    for (inside, expected) in [
        ("test/../src/a.test.js", "src/a.test.js"),
        ("/project/test/./unit/../a.test.js", "test/a.test.js"),
    ] {
        assert_eq!(
            relative(inside, "/project").as_deref(),
            Some(expected),
            "{inside}"
        );
    }
}

#[test]
fn a_file_uri_decodes_to_its_path() {
    for (uri, path) in [
        ("file:///project/src/a%20b.js", "/project/src/a b.js"),
        ("file:///C:/work/app/x.ts", "C:/work/app/x.ts"),
        ("file://localhost/p/x", "/p/x"),
        ("FILE://LocalHost/p/x", "/p/x"),
        ("file:/p/x", "/p/x"),
        ("file://server/share/x.js", "//server/share/x.js"),
        ("file:///p/x.js#L3", "/p/x.js"),
        ("src/a.rs", "src/a.rs"),
        ("src/a%3Ab.rs", "src/a:b.rs"),
        ("C:/work/x.ts", "C:/work/x.ts"),
    ] {
        assert_eq!(file_uri_path(uri).as_deref(), Some(path), "{uri}");
    }
    assert_eq!(
        relative(
            &file_uri_path("file:///C:/work/app/x.ts").unwrap(),
            r"C:\work\app"
        )
        .as_deref(),
        Some("x.ts")
    );
}

#[test]
fn a_uri_of_another_scheme_is_none() {
    for uri in [
        "http://example.com/x",
        "https://e.com/a.js",
        "git+ssh://h/r",
        "urn:x",
    ] {
        assert_eq!(file_uri_path(uri), None, "{uri}");
    }
}

#[test]
fn a_decoded_dot_dot_is_judged_by_relative() {
    let path = file_uri_path("%2E%2E/etc/passwd").expect("a relative reference");
    assert_eq!(path, "../etc/passwd");
    assert_eq!(relative(&path, "/project"), None);
    let path = file_uri_path("file:///project/%2E%2E/etc/passwd").expect("a file URI");
    assert_eq!(relative(&path, "/project"), None);
}
