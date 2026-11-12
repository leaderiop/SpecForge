//! The one test that gives the port's `locate` git's variables through the process's environment, as a hook git runs
//! is given them: its own binary, so no other test reads the environment while this one sets it.

use specforge_effects::git::{Git, Located};
use specforge_git::Gix;
use specforge_git::testing::Repo;

#[allow(
    unsafe_code,
    reason = "set_var in the one test of this binary, before it starts a thread or reads the environment"
)]
#[test]
fn a_git_dir_in_the_process_environment_naming_another_repository_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().canonicalize().unwrap();
    let repo = Repo::init(&real.join("repo"));
    repo.write("a.txt", "a\n");
    repo.commit_all("first");
    let other = Repo::init(&real.join("other"));
    other.write("b.txt", "b\n");
    other.commit_all("other");
    // SAFETY: this binary holds this one test, and nothing has read the environment on another thread.
    unsafe { std::env::set_var("GIT_DIR", real.join("other/.git")) };
    let located = Gix::default().locate(&real.join("repo"));
    assert!(
        matches!(&located, Located::Unavailable(why) if why.starts_with("GIT_DIR")),
        "{located:?}"
    );
}
