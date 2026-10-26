//! A language server over a project that holds a named pipe where it expects a file ends its indexing
//! (ADR 0264); it never waits for a writer that never comes.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use crate::session::{Session, uri_of};
use serde_json::json;

/// A project with the software extension and `main.spec`, and a named pipe at `pipe_name`; the pipe is
/// released when this drops, so the blocking threads that wait on it end with the test.
struct PipeProject {
    dir: tempfile::TempDir,
    pipe: PathBuf,
}

impl PipeProject {
    fn new(pipe_name: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("specforge.json"),
            json!({"name": "t", "version": "0.1.0", "extensions": ["@specforge/software"]})
                .to_string(),
        )
        .unwrap();
        std::fs::write(dir.path().join("main.spec"), "\n").unwrap();
        let pipe = dir.path().join(pipe_name);
        let made = std::process::Command::new("mkfifo").arg(&pipe).status();
        assert!(made.unwrap().success());
        PipeProject { dir, pipe }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }
}

impl Drop for PipeProject {
    fn drop(&mut self) {
        let pipe = self.pipe.clone();
        // Opening a pipe for writing waits for a reader: off this thread, once more after each reader.
        std::thread::spawn(
            move || {
                while std::fs::OpenOptions::new().write(true).open(&pipe).is_ok() {}
            },
        );
    }
}

#[specforge_test_macros::test(
    behavior = "lsp_initialize",
    verify = "the language server ends indexing over a project whose specforge.lock or a .spec file is a named pipe"
)]
#[tokio::test]
async fn indexing_ends_though_a_source_is_a_named_pipe() {
    let project = PipeProject::new("blocker.spec");
    let (mut session, _) = Session::launch(project.root().to_str(), json!({})).await;
    session
        .notification("$/progress", |p| p["value"]["kind"] == "end")
        .await
        .expect("indexing never ended");
}

#[specforge_test_macros::test(
    behavior = "lsp_initialize",
    verify = "the language server ends indexing over a project whose specforge.lock or a .spec file is a named pipe"
)]
#[tokio::test]
async fn indexing_ends_though_the_lock_is_a_named_pipe() {
    let project = PipeProject::new("specforge.lock");
    let (mut session, _) = Session::launch(project.root().to_str(), json!({})).await;
    session
        .notification("$/progress", |p| p["value"]["kind"] == "end")
        .await
        .expect("indexing never ended");
    let uri = uri_of(&project.root().join("main.spec"));
    session.did_open(&uri, "specforge", "\n").await;
    let reply = session.completion(&uri, 0, 0).await;
    let labels = reply["result"].to_string();
    assert!(labels.contains("\"behavior\""), "{labels}");
}
