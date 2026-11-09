//! `parse_all` over an in-process host whose answer holds at most `cap` files.

use std::cell::RefCell;
use std::rc::Rc;

use specforge_extension_sdk::host::in_process::{serve, InProcessHost};
use specforge_extension_sdk::host::{
    HostError, NotParsed, SyntaxAnswer, SyntaxFile, SyntaxFiles, SyntaxOutcome, SyntaxRequest,
    SyntaxSource,
};

use crate::parse_all;

/// The files a glob matches (`a.js` … in order), every answer capped after `cap` files; `short`, a glob's walk
/// stopped short. Each request's files are kept.
struct Host {
    globbed: Vec<&'static str>,
    cap: usize,
    short: bool,
    asked: RefCell<Vec<SyntaxFiles>>,
}

impl InProcessHost for Host {
    fn syntax_parse(&self, request: &SyntaxRequest) -> Result<SyntaxAnswer, HostError> {
        self.asked.borrow_mut().push(request.files.clone());
        let paths: Vec<String> = match &request.files {
            SyntaxFiles::Paths(paths) => paths.clone(),
            SyntaxFiles::Glob(_) => self.globbed.iter().map(|p| p.to_string()).collect(),
        };
        let files = paths
            .into_iter()
            .enumerate()
            .map(|(at, path)| SyntaxFile {
                outcome: if at < self.cap {
                    SyntaxOutcome::Parsed {
                        matches: Vec::new(),
                        errors: 0,
                    }
                } else {
                    SyntaxOutcome::NotParsed {
                        reason: NotParsed::Capped,
                    }
                },
                path,
                digest: None,
            })
            .collect();
        Ok(SyntaxAnswer {
            files,
            truncated: self.short && matches!(request.files, SyntaxFiles::Glob(_)),
            parsed: 0,
            cached: 0,
        })
    }
}

fn host(globbed: &[&'static str], cap: usize) -> Host {
    Host {
        globbed: globbed.to_vec(),
        cap,
        short: false,
        asked: RefCell::new(Vec::new()),
    }
}

fn request(files: SyntaxFiles) -> SyntaxRequest {
    SyntaxRequest {
        source: SyntaxSource::Worktree {
            root: "project".to_string(),
        },
        files,
        language: "tsx".to_string(),
        query: String::new(),
    }
}

/// What `parse_all` answers for a glob over `host`, keeping what `keep` keeps: the answered paths, and the requests.
fn globbed(
    host: Host,
    keep: impl Fn(&str) -> bool,
) -> (Result<Vec<String>, String>, Vec<SyntaxFiles>) {
    let host = Rc::new(host);
    let out = serve(host.clone(), || {
        parse_all("toy", request(SyntaxFiles::Glob("**/*.js".into())), keep)
    })
    .map(|files| files.into_iter().map(|f| f.path).collect());
    let asked = host.asked.borrow().clone();
    (out, asked)
}

fn paths(paths: &[&str]) -> SyntaxFiles {
    SyntaxFiles::Paths(paths.iter().map(|p| p.to_string()).collect())
}

#[test]
fn the_files_a_cap_left_are_asked_again_by_path_until_none_is() {
    let (out, asked) = globbed(host(&["a.js", "b.js", "c.js"], 1), |_| true);
    assert_eq!(out.unwrap(), ["a.js", "b.js", "c.js"]);
    assert_eq!(
        asked,
        [
            SyntaxFiles::Glob("**/*.js".into()),
            paths(&["b.js", "c.js"]),
            paths(&["c.js"])
        ]
    );
}

#[test]
fn a_file_the_pack_does_not_keep_is_never_answered_nor_asked_again() {
    let (out, asked) = globbed(host(&["a.js", "b.min.js", "c.js"], 1), |p| {
        !p.ends_with(".min.js")
    });
    assert_eq!(out.unwrap(), ["a.js", "c.js"]);
    assert_eq!(asked[1..], [paths(&["c.js"])]);
}

#[test]
fn a_glob_whose_walk_stopped_short_is_the_askers_error() {
    let short = Host {
        short: true,
        ..host(&["a.js"], 1)
    };
    let error = globbed(short, |_| true).0.unwrap_err();
    assert_eq!(
        error,
        "**/*.js: the project holds more files than the host walks; toy reads every file it keeps or none"
    );
}

#[test]
fn a_file_over_the_cap_alone_is_the_askers_error() {
    let (out, asked) = globbed(host(&["a.js", "b.js"], 0), |_| true);
    assert_eq!(
        out.unwrap_err(),
        "a.js: its matches alone pass the host's answer cap"
    );
    assert_eq!(asked.len(), 2, "the glob, then its files by path once");
}

#[test]
fn no_path_asks_nothing() {
    let host = Rc::new(host(&[], 1));
    let out = serve(host.clone(), || {
        parse_all("toy", request(paths(&[])), |_| true)
    });
    assert!(out.unwrap().is_empty());
    assert!(host.asked.borrow().is_empty());
}
