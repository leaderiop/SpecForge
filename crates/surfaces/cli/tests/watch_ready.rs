//! `specforge watch`'s ready event names the spec root as the path given names the project.

#![cfg(unix)]

use std::io::{BufRead, BufReader};
use std::time::{Duration, Instant};

use specforge_test::prelude::*;

use crate::support::{Cli, TestProject};

#[specforge_test(
    behavior = "watch_file_system_for_changes",
    verify = "the ready event names the spec root as the path given names the project"
)]
#[ignore = "flipped by T16"]
fn the_ready_event_names_the_spec_root_through_the_path_given() {
    let project = TestProject::new().raw_config(r#"{"spec_root":"spec"}"#);
    project.write("spec/a.spec", "entity one \"One\" {}\n");
    let links = tempfile::TempDir::new().unwrap();
    let link = links.path().join("lnk");
    std::os::unix::fs::symlink(project.root(), &link).unwrap();

    let mut child = Cli::on(&project).bare_spawn(
        &["watch", "--format", "json", "--path"]
            .iter()
            .copied()
            .chain([link.to_str().unwrap()])
            .collect::<Vec<_>>(),
    );
    let stdout = child.take_stdout().expect("stdout piped");
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut lines = BufReader::new(stdout).lines().map_while(Result::ok);
    let ready = lines
        .find(|line| line.contains("\"event\":\"ready\"") || Instant::now() > deadline)
        .expect("watch never reported ready");
    let event: serde_json::Value = serde_json::from_str(&ready).unwrap();
    assert_eq!(
        event["spec_root"],
        link.join("spec").to_str().unwrap(),
        "{ready}"
    );
}
