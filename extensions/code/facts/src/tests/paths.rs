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
fn only_two_hex_digits_decode_and_a_run_that_is_no_utf8_stays_as_written() {
    for (uri, path) in [
        // `u8::from_str_radix` takes a sign: `%+f` is no escape.
        ("src/a%+fb.rs", "src/a%+fb.rs"),
        ("src/a%-1.rs", "src/a%-1.rs"),
        ("src/a%2", "src/a%2"),
        ("src/%%2e", "src/%."),
        // One run that is no UTF-8 keeps its spelling; every other escape still decodes.
        ("%ff/%2e%2e/a%20b", "%ff/../a b"),
        ("%C3%A9%C3/x", "é%C3/x"),
        ("%E2%82%AC%e2%82", "€%e2%82"),
    ] {
        assert_eq!(file_uri_path(uri).as_deref(), Some(path), "{uri}");
    }
    assert_eq!(
        relative(&file_uri_path("%FF/%2e%2e/%2e%2e/x").unwrap(), "/project"),
        None
    );
}

#[test]
fn the_outside_line_counts_the_paths_and_names_five() {
    let r = ".specforge/reports/x.lcov";
    assert_eq!(outside_line(r, &[]), None);
    assert_eq!(
        outside_line(r, &super::strings(&["/elsewhere/a.rs", "../../etc/passwd"])).as_deref(),
        Some(
            ".specforge/reports/x.lcov: skipped 2 file(s) outside the project: /elsewhere/a.rs, \
             ../../etc/passwd"
        )
    );
    let seven: Vec<String> = (1..=7).map(|n| format!("/o/{n}")).collect();
    assert_eq!(
        outside_line(r, &seven).as_deref(),
        Some(
            ".specforge/reports/x.lcov: skipped 7 file(s) outside the project: /o/1, /o/2, /o/3, /o/4, \
             /o/5, and 2 more"
        )
    );
    let five = &seven[..5];
    assert!(!outside_line(r, five).unwrap().contains("more"));
}

#[test]
fn a_path_resolving_to_the_root_or_a_pseudo_name_names_no_file_not_outside() {
    for empty in [".", "./", "a/..", "/project", "/project/", "/project/a/.."] {
        assert_eq!(place(empty, "/project"), Place::NoFile, "{empty}");
        assert_eq!(relative(empty, "/project"), None, "{empty}");
    }
    for pseudo in ["<std macros>", "<anon>", "<proc-macro source code>"] {
        assert_eq!(place(pseudo, "/project"), Place::NoFile, "{pseudo}");
    }
    for outside in ["..", "../x.rs", "/elsewhere/x.rs", "/project-two/x.rs"] {
        assert_eq!(place(outside, "/project"), Place::Outside, "{outside}");
    }
    assert_eq!(
        place("/project/src/lib.rs", "/project"),
        Place::File("src/lib.rs".to_string())
    );
    assert_eq!(
        place("<a>/b.rs", "/project"),
        Place::File("<a>/b.rs".to_string())
    );
}

#[test]
fn the_no_file_line_counts_the_paths_like_the_outside_line() {
    let r = ".specforge/reports/x.sarif";
    assert_eq!(no_file_line(r, &[]), None);
    assert_eq!(
        no_file_line(r, &super::strings(&["<std macros>", "."])).as_deref(),
        Some(".specforge/reports/x.sarif: skipped 2 path(s) naming no file: <std macros>, .")
    );
}

#[test]
fn a_decoded_dot_dot_is_judged_by_relative() {
    let path = file_uri_path("%2E%2E/etc/passwd").expect("a relative reference");
    assert_eq!(path, "../etc/passwd");
    assert_eq!(relative(&path, "/project"), None);
    let path = file_uri_path("file:///project/%2E%2E/etc/passwd").expect("a file URI");
    assert_eq!(relative(&path, "/project"), None);
}
