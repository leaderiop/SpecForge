//! The S2 spike (ADR 0339 D5): what the pinned gix answers for each git operation GSD's blocks run, on a repository
//! gix builds. A test that runs is an operation gix covers; one ignored names what gix lacks, and its body shows the
//! nearest piece gix has and where it stops. ADR 0339's spike table is this module, one row per test.

#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use gix::bstr::ByteSlice as _;
use gix::refs::transaction::PreviousValue;

use specforge_git::testing::Repo;

/// A repository with two commits on `main`: `a.txt` changed, `b.txt` added; `b.txt` binary in the second.
fn two_commits(dir: &std::path::Path) -> Repo {
    let repo = Repo::init(dir);
    repo.write("a.txt", "one\ntwo\n")
        .write("src/c.rs", "fn c() {}\n");
    repo.commit_all("first\n\nThe body.\n\nGsd-Task: 7\n");
    repo.write("a.txt", "one\nthree\n")
        .write("b.bin", b"\0\x01\x02");
    repo.commit_all("second\n");
    repo
}

#[test]
fn reads_locate() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let found = gix::discover(repo.dir().join("src")).unwrap();
    assert_eq!(
        found.workdir().unwrap().canonicalize().unwrap(),
        repo.dir().canonicalize().unwrap()
    );
    assert_eq!(found.git_dir(), found.common_dir());
    assert!(!found.is_shallow());
    let outside = tempfile::tempdir().unwrap();
    assert!(gix::discover(outside.path()).is_err());
}

#[test]
fn reads_rev_parse_forms() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let git = repo.open();
    let head = git.rev_parse_single("HEAD").unwrap().detach();
    assert_eq!(git.rev_parse_single("main").unwrap().detach(), head);
    let parent = git.rev_parse_single("HEAD~1").unwrap().detach();
    assert_eq!(git.rev_parse_single("HEAD^").unwrap().detach(), parent);
    let short = head.to_hex_with_len(7).to_string();
    assert_eq!(git.rev_parse_single(short.as_str()).unwrap().detach(), head);
    let blob = git.rev_parse_single("HEAD:a.txt").unwrap();
    assert_eq!(blob.object().unwrap().kind, gix::objs::Kind::Blob);
    assert!(git.rev_parse_single("nosuch").is_err());
    assert_eq!(git.head_name().unwrap().unwrap().shorten(), "main");
}

#[test]
fn reads_status() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.write("a.txt", "changed\n").write("new.txt", "n");
    let git = repo.open();
    let mut paths: Vec<String> = git
        .status(gix::progress::Discard)
        .unwrap()
        .untracked_files(gix::status::UntrackedFiles::Files)
        .into_iter(None)
        .unwrap()
        .map(|item| item.unwrap().location().to_string())
        .collect();
    paths.sort();
    assert_eq!(paths, ["a.txt", "new.txt"]);
    assert!(git.is_dirty().unwrap());
}

#[test]
fn reads_log_and_count() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let git = repo.open();
    let head = git.head_id().unwrap();
    let commits: Vec<_> = head
        .ancestors()
        .all()
        .unwrap()
        .map(|info| info.unwrap().id)
        .collect();
    assert_eq!(commits.len(), 2, "rev-list --count");
    let first = git.find_commit(commits[1]).unwrap();
    let message = first.message().unwrap();
    assert_eq!(message.title, "first");
    let trailers: Vec<_> = first
        .message_raw()
        .unwrap()
        .to_str()
        .unwrap()
        .lines()
        .filter_map(|line| line.split_once(": "))
        .collect();
    assert_eq!(trailers, [("Gsd-Task", "7")]);
}

#[test]
fn reads_diff_tree_to_tree_with_patch() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let git = repo.open();
    let old = git
        .rev_parse_single("HEAD~1")
        .unwrap()
        .object()
        .unwrap()
        .peel_to_tree()
        .unwrap();
    let new = git.head_tree().unwrap();
    let changes = git.diff_tree_to_tree(&old, &new, None).unwrap();
    let mut paths: Vec<String> = changes.iter().map(|c| c.location().to_string()).collect();
    paths.sort();
    assert_eq!(paths, ["a.txt", "b.bin"]);
    // the patch text: imara-diff's unified diff of the two blobs
    let before = git
        .rev_parse_single("HEAD~1:a.txt")
        .unwrap()
        .object()
        .unwrap()
        .detach()
        .data;
    let after = git
        .rev_parse_single("HEAD:a.txt")
        .unwrap()
        .object()
        .unwrap()
        .detach()
        .data;
    let input = gix::diff::blob::InternedInput::new(before.as_slice(), after.as_slice());
    let diff = gix::diff::blob::Diff::compute(gix::diff::blob::Algorithm::Histogram, &input);
    assert_eq!((diff.count_additions(), diff.count_removals()), (1, 1));
    let patch = gix::diff::blob::UnifiedDiff::new(
        &diff,
        &input,
        gix::diff::blob::unified_diff::ConsumeBinaryHunk::new(String::new(), "\n"),
        gix::diff::blob::unified_diff::ContextSize::symmetrical(3),
    )
    .consume()
    .unwrap();
    assert!(patch.contains("-two\n+three\n"), "{patch}");
}

#[test]
fn reads_diff_tree_to_worktree() {
    // gix has tree→index and index→worktree; a base→worktree diff is the two composed (status with a head tree)
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.write("a.txt", "worktree\n");
    let git = repo.open();
    let base = git
        .rev_parse_single("HEAD~1")
        .unwrap()
        .object()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .id;
    let mut paths: Vec<String> = git
        .status(gix::progress::Discard)
        .unwrap()
        .head_tree(base)
        .untracked_files(gix::status::UntrackedFiles::None)
        .into_iter(None)
        .unwrap()
        .map(|item| item.unwrap().location().to_string())
        .collect();
    paths.sort();
    paths.dedup();
    assert_eq!(paths, ["a.txt", "b.bin"]);
}

#[test]
fn reads_merge_base() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let base = repo.sha("HEAD~1");
    repo.branch("side", &base).head_on("side");
    repo.write("side.txt", "s");
    let side = repo.commit_all("side\n");
    let git = repo.open();
    let found = git
        .merge_base(
            gix::ObjectId::from_hex(side.as_bytes()).unwrap(),
            git.rev_parse_single("main").unwrap(),
        )
        .unwrap();
    assert_eq!(found.to_string(), base);
}

#[test]
fn reads_show() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let git = repo.open();
    let blob = git
        .rev_parse_single("HEAD~1:a.txt")
        .unwrap()
        .object()
        .unwrap();
    assert_eq!(blob.data.as_slice(), b"one\ntwo\n");
}

#[test]
fn reads_branches() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.remote("origin", "https://example.com/acme/widgets.git");
    repo.branch("side", &repo.sha("HEAD~1"))
        .upstream("main", &repo.sha("HEAD~1"));
    let git = repo.open();
    let mut names: Vec<String> = git
        .references()
        .unwrap()
        .local_branches()
        .unwrap()
        .map(|r| r.unwrap().name().shorten().to_string())
        .collect();
    names.sort();
    assert_eq!(names, ["main", "side"]);
    let remote: Vec<String> = git
        .references()
        .unwrap()
        .remote_branches()
        .unwrap()
        .map(|r| r.unwrap().name().shorten().to_string())
        .collect();
    assert_eq!(remote, ["origin/main"]);
    let main = git.find_reference("refs/heads/main").unwrap();
    let tracking = main
        .remote_tracking_ref_name(gix::remote::Direction::Fetch)
        .unwrap()
        .unwrap();
    assert_eq!(tracking.as_bstr(), "refs/remotes/origin/main");
}

#[test]
fn reads_worktree_list() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(&dir.path().join("main"));
    let linked = repo.linked_worktree(&dir.path().join("wt"), "feature", &repo.sha("HEAD"));
    let git = repo.open();
    let worktrees = git.worktrees().unwrap();
    assert_eq!(worktrees.len(), 1);
    assert_eq!(
        worktrees[0].base().unwrap().canonicalize().unwrap(),
        linked.dir().canonicalize().unwrap()
    );
    let there = linked.open();
    assert_ne!(there.git_dir(), there.common_dir());
    assert_eq!(there.head_name().unwrap().unwrap().shorten(), "feature");
    assert!(!there.is_dirty().unwrap());
}

#[test]
fn reads_remotes() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.remote("origin", "git@github.com:acme/widgets.git");
    let git = repo.open();
    let names: Vec<String> = git.remote_names().iter().map(ToString::to_string).collect();
    assert_eq!(names, ["origin"]);
    let origin = git.find_remote("origin").unwrap();
    let url = origin.url(gix::remote::Direction::Fetch).unwrap();
    assert_eq!(url.host(), Some("github.com"));
    assert_eq!(url.path, "acme/widgets.git");
}

#[test]
fn reads_config() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.config("[user]\n\tname = Ada\n[core]\n\thooksPath = .githooks");
    let git = repo.open();
    let config = git.config_snapshot();
    assert_eq!(config.string("user.name").unwrap().as_ref(), "Ada");
    assert_eq!(
        config.string("core.hooksPath").unwrap().as_ref(),
        ".githooks"
    );
}

#[test]
fn writes_index_add_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.write("n.txt", "n").delete("a.txt");
    repo.stage(&["n.txt", "a.txt"]);
    let git = repo.open();
    let index = git.index().unwrap();
    let paths: Vec<String> = index
        .entries()
        .iter()
        .map(|e| e.path(&index).to_string())
        .collect();
    assert_eq!(paths, ["b.bin", "n.txt", "src/c.rs"]);
}

#[test]
fn writes_commit_with_identity() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.config("[user]\n\tname = Ada\n\temail = ada@example.com");
    let git = repo.open();
    let tree = git.head_tree_id().unwrap();
    let parent = git.head_id().unwrap();
    let id = git
        .commit("HEAD", "third\n\nGsd-Task: 9\n", tree, [parent])
        .unwrap();
    let commit = git.find_commit(id).unwrap();
    assert_eq!(commit.author().unwrap().name, "Ada");
    assert_eq!(commit.message_raw().unwrap(), "third\n\nGsd-Task: 9\n");
    assert_eq!(repo.sha("main"), id.to_string());
    // a branch commit writes its loose ref, the file gsd-git's detection reads
    assert!(repo.git_dir().join("refs/heads/main").is_file());
}

#[test]
fn writes_signing_is_detected() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.config("[commit]\n\tgpgsign = true");
    let git = repo.open();
    assert_eq!(git.config_snapshot().boolean("commit.gpgsign"), Some(true));
}

#[test]
#[ignore = "gix 0.84.0 lacks hooks: it runs none; the host runs them through the Process port"]
fn writes_hooks_invoked() {
    // gix finds where hooks live (core.hooksPath, else the common dir's `hooks`), and runs nothing there
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let git = repo.open();
    assert!(git.config_snapshot().string("core.hooksPath").is_none());
    assert!(git.common_dir().join("hooks").is_dir() || !git.common_dir().join("hooks").exists());
}

#[test]
fn writes_branch_create_and_delete() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let git = repo.open();
    let head = git.head_id().unwrap().detach();
    git.reference(
        "refs/heads/topic",
        head,
        PreviousValue::MustNotExist,
        "branch: Created from HEAD",
    )
    .unwrap();
    assert_eq!(repo.sha("topic"), head.to_string());
    let topic = git.find_reference("refs/heads/topic").unwrap();
    topic.delete().unwrap();
    assert!(
        git.try_find_reference("refs/heads/topic")
            .unwrap()
            .is_none()
    );
}

#[test]
fn writes_tag() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.config("[user]\n\tname = Ada\n\temail = ada@example.com");
    let git = repo.open();
    let head = git.head_id().unwrap();
    let tagger = git.committer().unwrap().unwrap();
    git.tag(
        "v1",
        head,
        gix::objs::Kind::Commit,
        Some(tagger),
        "release\n",
        PreviousValue::MustNotExist,
    )
    .unwrap();
    let tag = git
        .find_reference("refs/tags/v1")
        .unwrap()
        .id()
        .object()
        .unwrap();
    assert_eq!(tag.kind, gix::objs::Kind::Tag);
    assert_eq!(repo.sha("v1^{commit}"), head.to_string());
}

#[test]
#[ignore = "gix 0.84.0 lacks switch: it moves HEAD, but checks out only into an empty directory"]
fn writes_switch() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.branch("old", &repo.sha("HEAD~1"));
    repo.head_on("old");
    // HEAD moved; the working tree still holds the second commit's files
    assert!(repo.dir().join("b.bin").exists());
}

#[test]
#[ignore = "gix 0.84.0 lacks worktree add and remove: it lists and opens worktrees, and creates none"]
fn writes_worktree_add_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    assert!(repo.open().worktrees().unwrap().is_empty());
}

#[test]
fn writes_merge_fast_forward() {
    // a fast-forward is a ref move once the worktree is clean; gix moves the ref (the files are a checkout, below)
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let git = repo.open();
    let first = git.rev_parse_single("HEAD~1").unwrap().detach();
    let second = git.head_id().unwrap().detach();
    git.reference(
        "refs/heads/behind",
        first,
        PreviousValue::MustNotExist,
        "branch",
    )
    .unwrap();
    let base = git.merge_base(first, second).unwrap();
    assert_eq!(base.detach(), first, "behind is an ancestor: fast-forward");
    git.reference(
        "refs/heads/behind",
        second,
        PreviousValue::MustExistAndMatch(gix::refs::Target::Object(first)),
        "merge: Fast-forward",
    )
    .unwrap();
    assert_eq!(repo.sha("behind"), second.to_string());
}

#[test]
fn writes_merge_three_way_tree_and_conflict() {
    // gix merges two commits into a tree and names the conflicts; writing it into the index and worktree is not gix's
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let base = repo.sha("HEAD~1");
    repo.branch("side", &base).head_on("side");
    repo.write("a.txt", "one\nsides\n").write("side.txt", "s");
    repo.commit_all("side\n");
    let git = repo.open();
    let options = git.tree_merge_options().unwrap();
    let ours = git.rev_parse_single("main").unwrap();
    let theirs = git.rev_parse_single("side").unwrap();
    let outcome = git
        .merge_commits(ours, theirs, Default::default(), options.into())
        .unwrap();
    assert!(
        outcome
            .tree_merge
            .has_unresolved_conflicts(Default::default())
    );
    let conflicted: Vec<String> = outcome
        .tree_merge
        .conflicts
        .iter()
        .map(|c| c.ours.location().to_string())
        .collect();
    assert_eq!(conflicted, ["a.txt"]);
}

#[test]
#[ignore = "gix 0.84.0 lacks merge into the worktree: its merged tree and conflicts reach no index or file"]
fn writes_merge_into_worktree() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    assert!(!repo.open().is_dirty().unwrap());
}

#[test]
fn writes_reset_soft_and_mixed() {
    // soft moves the branch; mixed also writes the index from the commit's tree: both are gix
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let git = repo.open();
    let first = git.rev_parse_single("HEAD~1").unwrap().detach();
    git.reference(
        "refs/heads/main",
        first,
        PreviousValue::Any,
        "reset: moving to HEAD~1",
    )
    .unwrap();
    let tree = git.find_commit(first).unwrap().tree_id().unwrap();
    let mut index = git.index_from_tree(&tree).unwrap();
    index.write(gix::index::write::Options::default()).unwrap();
    assert_eq!(repo.sha("HEAD"), first.to_string());
    let git = repo.open();
    let index = git.index().unwrap();
    assert!(index.entry_by_path("b.bin".into()).is_none());
}

#[test]
#[ignore = "gix 0.84.0 lacks reset --hard and restore: no checkout over an existing worktree"]
fn writes_reset_hard_and_restore() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    repo.write("a.txt", "lost?\n");
    assert!(repo.open().is_dirty().unwrap());
}

#[test]
#[ignore = "gix 0.84.0 lacks stash: it reads no refs/stash and writes none"]
fn writes_stash() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    assert!(
        repo.open()
            .try_find_reference("refs/stash")
            .unwrap()
            .is_none()
    );
}

#[test]
fn writes_cherry_pick_and_revert_trees() {
    // a cherry-pick merges the commit against its parent onto HEAD; a revert the other way: gix's merge_trees
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    let git = repo.open();
    let first = git
        .rev_parse_single("HEAD~1")
        .unwrap()
        .object()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .id;
    let second = git.head_tree_id().unwrap().detach();
    let options = git.tree_merge_options().unwrap();
    let reverted = git
        .merge_trees(second, second, first, Default::default(), options)
        .unwrap();
    assert_eq!(reverted.conflicts.len(), 0);
    let mut tree = reverted.tree;
    assert_eq!(
        tree.write().unwrap().detach(),
        first,
        "reverting the tip gives its parent's tree"
    );
}

#[test]
#[ignore = "gix 0.84.0 lacks cherry-pick and revert into the worktree: the picked tree reaches no index or file"]
fn writes_cherry_pick_and_revert() {
    let dir = tempfile::tempdir().unwrap();
    let repo = two_commits(dir.path());
    assert!(!repo.open().is_dirty().unwrap());
}
