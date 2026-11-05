//! The arguments of `specforge git`'s actions.

use super::super::VerbArgs;

#[derive(clap::Args)]
pub(crate) struct MergeArgs {
    /// The branch or commit merged in
    pub(super) revision: String,

    /// Refuse unless HEAD can fast-forward
    #[arg(long, conflicts_with = "no_ff")]
    pub(super) ff_only: bool,

    /// Record a merge commit even when HEAD could fast-forward
    #[arg(long)]
    pub(super) no_ff: bool,

    /// The merge commit's message
    #[arg(short, long, value_name = "MESSAGE")]
    pub(super) message: Option<String>,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct ResetArgs {
    /// Where HEAD goes; HEAD when absent
    pub(super) revision: Option<String>,

    /// Move HEAD alone
    #[arg(long, conflicts_with_all = ["mixed", "hard"])]
    pub(super) soft: bool,

    /// Move HEAD and the index (the default)
    #[arg(long, conflicts_with = "hard")]
    pub(super) mixed: bool,

    /// Move HEAD, the index and the files: changes are lost
    #[arg(long)]
    pub(super) hard: bool,

    /// Discard the tree's changes with --hard
    #[arg(long)]
    pub(super) yes: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct RestoreArgs {
    /// The files, from the current directory
    #[arg(required = true)]
    pub(super) paths: Vec<std::path::PathBuf>,

    /// Take them from this commit instead
    #[arg(long, value_name = "REV")]
    pub(super) source: Option<String>,

    /// Restore their index entries (from HEAD, or --source), not the files
    #[arg(long)]
    pub(super) staged: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Subcommand)]
pub(crate) enum StashAction {
    /// Stash the changes of the index and the worktree, and clean the tree
    Push(StashPushArgs),
    /// Apply a stash (the newest when absent) and drop it
    Pop(StashPopArgs),
    /// The stashes, newest first
    List(Plain),
}

#[derive(clap::Args)]
pub(crate) struct StashPushArgs {
    #[arg(short, long, value_name = "MESSAGE")]
    pub(super) message: Option<String>,

    /// Stash the untracked files too
    #[arg(short = 'u', long)]
    pub(super) include_untracked: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct StashPopArgs {
    /// The stash's index (`0` the newest)
    #[arg(default_value_t = 0)]
    pub(super) index: usize,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct PickArgs {
    /// The commit
    #[arg(required_unless_present_any = ["quit", "abort"], conflicts_with_all = ["quit", "abort"])]
    pub(super) revision: Option<String>,

    /// Apply it onto the index as it stands and leave it staged, unrecorded; a second one stacks on the first
    #[arg(short, long)]
    pub(super) no_commit: bool,

    /// End the cherry-pick or revert in progress, the index and the files left as they are
    #[arg(long, conflicts_with_all = ["abort", "no_commit"])]
    pub(super) quit: bool,

    /// End the cherry-pick or revert in progress and put back as HEAD has them the paths it staged or conflicted
    #[arg(long, conflicts_with = "no_commit")]
    pub(super) abort: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Subcommand)]
pub(crate) enum BranchAction {
    /// Create a branch at a revision (HEAD when absent)
    Create(BranchCreateArgs),
    /// Delete a branch: never HEAD's, and one with commits HEAD does not hold only with --force
    Delete(BranchDeleteArgs),
}

#[derive(clap::Args)]
pub(crate) struct BranchCreateArgs {
    pub(super) name: String,

    /// Where the branch starts; HEAD when absent
    #[arg(long, value_name = "REV")]
    pub(super) at: Option<String>,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct BranchDeleteArgs {
    pub(super) name: String,

    /// Delete it even when HEAD does not hold its commits
    #[arg(long)]
    pub(super) force: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct SwitchArgs {
    /// The branch
    pub(super) name: String,

    /// Create the branch first (at --at, or HEAD)
    #[arg(short, long)]
    pub(super) create: bool,

    /// Where a created branch starts; HEAD when absent
    #[arg(long, value_name = "REV", requires = "create")]
    pub(super) at: Option<String>,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct TagArgs {
    pub(super) name: String,

    /// The commit tagged; HEAD when absent
    #[arg(long, value_name = "REV")]
    pub(super) at: Option<String>,

    /// An annotated tag's message, by git's configured identity
    #[arg(short, long, value_name = "MESSAGE")]
    pub(super) message: Option<String>,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Subcommand)]
pub(crate) enum WorktreeAction {
    /// Add a linked worktree at a directory (absent or empty) on a branch: its name when no --branch, made when missing
    Add(WorktreeAddArgs),
    /// Remove a linked worktree and its directory: one with changes or untracked files only with --force
    Remove(WorktreeRemoveArgs),
    /// Drop the records of linked worktrees whose directories are gone
    Prune(Plain),
}

#[derive(clap::Args)]
pub(crate) struct WorktreeAddArgs {
    /// The worktree's directory, from the current directory
    #[arg(value_name = "DIR")]
    pub(super) dir: std::path::PathBuf,

    /// The branch it is on
    #[arg(long, value_name = "NAME")]
    pub(super) branch: Option<String>,

    /// Create the branch first (at --at, or HEAD)
    #[arg(short, long)]
    pub(super) create: bool,

    /// Where a created branch starts; HEAD when absent
    #[arg(long, value_name = "REV")]
    pub(super) at: Option<String>,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct WorktreeRemoveArgs {
    /// The worktree's directory, from the current directory
    #[arg(value_name = "DIR")]
    pub(super) dir: std::path::PathBuf,

    /// Remove it with its changes and untracked files
    #[arg(long)]
    pub(super) force: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct AddArgs {
    /// The files, from the current directory: each at or under it, as the worktree has it (a deleted one leaves
    /// the index)
    #[arg(required_unless_present = "all", value_name = "PATH")]
    pub(super) paths: Vec<std::path::PathBuf>,

    /// Every file under the repository's top when no path is named, untracked ones too (`git add -A`)
    #[arg(short = 'A', long)]
    pub(super) all: bool,

    /// Leave out the files at or under this path, whatever else names them (`':!PATH'`)
    #[arg(long, value_name = "PATH")]
    pub(super) except: Vec<std::path::PathBuf>,

    /// Stage an ignored path too; without it one is refused, as `git add` refuses it
    #[arg(short, long)]
    pub(super) force: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct CommitArgs {
    /// The message; given more than once, each is a paragraph of its own (as `git commit -m` takes them)
    #[arg(short, long = "message", value_name = "MESSAGE", required = true)]
    pub(super) message: Vec<String>,

    /// Stage the files at or under this path first, as the worktree has them (a deleted one leaves the index)
    #[arg(long, value_name = "PATH")]
    pub(super) add: Vec<std::path::PathBuf>,

    /// Take the files at or under this path out of the index first, leaving them on disk (`git rm --cached`)
    #[arg(long, value_name = "PATH")]
    pub(super) remove: Vec<std::path::PathBuf>,

    /// Stage every tracked file first as the worktree has it; untracked files stay out
    #[arg(short, long)]
    pub(super) all: bool,

    /// Stage an ignored path --add names too; without it one is refused, as `git add` refuses it
    #[arg(long, requires = "add")]
    pub(super) force: bool,

    /// Commit even when the index matches HEAD
    #[arg(long)]
    pub(super) allow_empty: bool,

    /// Run none of the repository's pre-commit, commit-msg and post-commit hooks; prepare-commit-msg still runs, as
    /// git runs it
    #[arg(long)]
    pub(super) no_verify: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct BranchesArgs {
    /// The remote-tracking branches instead (`origin/main`)
    #[arg(long)]
    pub(super) remote: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct Plain {
    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct RevArgs {
    /// The revisions: a sha, a branch, a tag, `HEAD~1`, `main:path`
    #[arg(required_unless_present_any = ["sorted", "stdin"])]
    pub(super) revs: Vec<String>,

    /// Each must name a commit: the commits newest first by committer date, as `git rev-list --no-walk=sorted`
    /// orders them
    #[arg(long)]
    pub(super) sorted: bool,

    /// With --sorted: oldest first
    #[arg(long, requires = "sorted")]
    pub(super) reverse: bool,

    /// Read more revisions from stdin, one per line
    #[arg(long)]
    pub(super) stdin: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct ConfigArgs {
    /// The key, `section.name` or `section.subsection.name`
    pub(super) key: String,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct LogArgs {
    /// `A..B` (what B reaches and A does not), `A...B` (what either reaches and not both) or one revision;
    /// `HEAD` when absent
    pub(super) range: Option<String>,

    /// At most this many commits listed (1000 when absent; `count` still counts them all, and `truncated` says so)
    #[arg(long, value_name = "N")]
    pub(super) max: Option<usize>,

    /// Only the commits that change these paths, from the repository's top (after `--`, as git takes them)
    #[arg(last = true, value_name = "PATH")]
    pub(super) paths: Vec<String>,

    /// Follow only each commit's first parent
    #[arg(long)]
    pub(super) first_parent: bool,

    /// List oldest first; --max still keeps the newest, as git applies -n before --reverse
    #[arg(long)]
    pub(super) reverse: bool,

    /// Only the commits a line of whose message this regex matches (Rust's regex syntax, ^ and $ at each line)
    #[arg(long, value_name = "REGEX")]
    pub(super) grep: Option<String>,

    /// Only the commits made at or after this date: ISO 8601, RFC 2822, a unix time, or `24 hours ago`, as git reads
    /// one
    #[arg(long, value_name = "DATE", visible_alias = "after")]
    pub(super) since: Option<String>,

    /// Only the commits made at or before this date
    #[arg(long, value_name = "DATE", visible_alias = "before")]
    pub(super) until: Option<String>,

    /// No commit with more than one parent
    #[arg(long)]
    pub(super) no_merges: bool,

    /// Only the commits that change how many times this text occurs in a file (of the paths), against their parent
    #[arg(short = 'S', value_name = "TEXT")]
    pub(super) pickaxe: Option<String>,

    /// -S names a regex, not a text
    #[arg(long, requires = "pickaxe")]
    pub(super) pickaxe_regex: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct DiffArgs {
    /// The commit to compare from; `HEAD` when absent
    #[arg(long, value_name = "REV")]
    pub(super) base: Option<String>,

    /// The commit to compare with; `HEAD` when `--base` is given, else the worktree
    #[arg(long, value_name = "REV", conflicts_with_all = ["staged", "worktree"])]
    pub(super) head: Option<String>,

    /// Compare with the index
    #[arg(long, conflicts_with = "worktree")]
    pub(super) staged: bool,

    /// Compare with the tracked files as they are on disk
    #[arg(long)]
    pub(super) worktree: bool,

    /// Only the files at or under these paths, from the repository's top (after `--`, as git takes them)
    #[arg(last = true, value_name = "PATH")]
    pub(super) paths: Vec<String>,

    /// Give each text file's unified hunks; with --format human, each file's patch as git prints it
    #[arg(long)]
    pub(super) patch: bool,

    /// Compare from where --base and the head meet (`git diff A...B`)
    #[arg(long, requires = "base")]
    pub(super) merge_base: bool,

    /// With --format human: end the file lines with git's summary of files changed, insertions and deletions
    #[arg(long, conflicts_with = "shortstat")]
    pub(super) stat: bool,

    /// With --format human: print only git's summary of files changed, insertions and deletions
    #[arg(long, conflicts_with = "patch")]
    pub(super) shortstat: bool,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct MergeBaseArgs {
    pub(super) a: String,
    pub(super) b: String,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct ShowArgs {
    /// `<rev>:<path>`, the path from the repository's top
    pub(super) object: String,

    #[command(flatten)]
    pub(super) verb: VerbArgs,
}
