//! A repository that asks for signatures: specforge signs nothing, so every write that would make a commit or an
//! annotated tag refuses rather than make an unsigned one, as git's `commit.gpgSign` and `tag.gpgSign` ask.

use super::writes::side;
use super::{committed, git};

#[test]
fn a_repository_that_signs_every_commit_refuses() {
    // git reads a boolean as its config does: a non-zero integer is true
    for value in ["true", "2"] {
        let (project, repo, first) = committed();
        repo.config(&format!("[commit]\n\tgpgsign = {value}"));
        let refused = git(&project, &["commit", "--allow-empty", "-m", "x"]).exits(1);
        assert_eq!(
            refused.json()["code"],
            "signing_unsupported",
            "{value}: {}",
            refused.stdout
        );
        assert_eq!(repo.sha("HEAD"), first);
    }
}

#[test]
fn a_merge_commit_a_pick_and_a_revert_refuse_when_commits_are_signed() {
    let (project, repo, _first, _side) = side("s.txt", "side\n");
    repo.write("m.txt", "main\n");
    let main = repo.commit_all("on main");
    repo.config("[commit]\n\tgpgsign = true");
    for args in [
        &["merge", "side"][..],
        &["merge", "side", "--no-ff"],
        &["cherry-pick", "side"],
        &["revert", "HEAD"],
    ] {
        let refused = git(&project, args).exits(1);
        assert_eq!(
            refused.json()["code"],
            "signing_unsupported",
            "{args:?}: {}",
            refused.stdout
        );
    }
    assert_eq!(repo.sha("HEAD"), main);
}

#[test]
fn a_fast_forward_makes_no_commit_so_signing_does_not_stop_it() {
    let (project, repo, _first, side) = side("s.txt", "side\n");
    repo.config("[commit]\n\tgpgsign = true");
    git(&project, &["merge", "side"]).exits(0);
    assert_eq!(repo.sha("HEAD"), side);
}

#[test]
fn an_annotated_tag_refuses_when_tags_are_signed_and_a_light_one_does_not() {
    let (project, repo, first) = committed();
    repo.config("[tag]\n\tgpgSign = true");
    let refused = git(&project, &["tag", "v1", "-m", "release"]).exits(1);
    assert_eq!(
        refused.json()["code"],
        "signing_unsupported",
        "{}",
        refused.stdout
    );
    git(&project, &["tag", "v0"]).exits(0);
    assert_eq!(repo.sha("v0"), first);
}
