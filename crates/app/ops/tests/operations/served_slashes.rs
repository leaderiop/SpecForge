//! The slash commands a project serves (ADR 0316): the list a prompt that reads them is sent is the list an install
//! writes.

use serde_json::json;
use specforge_ops::host::{Host, render};
use specforge_ops::view::ProjectView;
use specforge_project::testing::GsdHosts;
use specforge_test_macros::test as specforge_test;

use crate::host_support::{hosted, hosts_project};

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "the slash commands a prompt is sent are the ones an install writes"
)]
fn served_slashes_are_what_an_install_writes() {
    let hosted = hosted(hosts_project(json!({}), json!({})), GsdHosts::new());
    let view = ProjectView::of(&hosted.compiled);
    let mut written: Vec<String> = render::rendered(&view, Host::ClaudeCode)
        .expect("the fixture renders")
        .slashes
        .into_iter()
        .map(|s| s.name)
        .collect();
    written.sort();
    let served = render::served_slashes(&view);
    let names: Vec<&str> = served.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, written);
    assert!(
        names.contains(&"t-plan-phase") && names.contains(&"t-stats"),
        "{names:?}"
    );
    let stats = served.iter().find(|s| s.name == "t-stats").unwrap();
    assert_eq!(stats.tool.as_deref(), Some("specforge.t.stats"));
    assert_eq!(stats.prompt, None);
    assert_eq!(stats.cli, "specforge t stats");
}
