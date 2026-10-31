//! The install plan (ADR 0292): what a rendered tree would do to the disk, file by file and entry by entry.
//!
//! | Disk | Marker | Plan |
//! |---|---|---|
//! | absent | | `create` |
//! | equal | | `unchanged` |
//! | differs, digest matches | ours, not edited | `update` |
//! | differs, digest does not match | ours, edited | refuse `edited_file` (`force`: `update`) |
//! | present | none | refuse `foreign_file` (always) |
//! | marked, no longer rendered | ours | `remove` (edited: refuse unless `force`) |
//! | a config file's entries | | merged by the adapter; its other keys untouched |

use std::collections::BTreeSet;

use super::marker::{self, Mark};
use super::port::{HostRenderer, HostTree, MergedEntry, NotRendered};
use super::{Host, owned_path};

/// What the plan reads of the disk.
pub trait HostDisk {
    /// The text at `path` (project-relative); `None` when nothing is there. Err: something is there that is not a
    /// text file this plan may read (a link, a directory, bytes that are no UTF-8): it is never install's.
    fn read(&self, path: &str) -> Result<Option<String>, String>;

    /// The files below the directory `dir`, project-relative, in path order.
    fn files_under(&self, dir: &str) -> Vec<String>;
}

/// What the plan does with a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Create,
    Update,
    Unchanged,
    Remove,
}

impl Action {
    /// The word the outcome document writes.
    pub fn as_str(self) -> &'static str {
        match self {
            Action::Create => "created",
            Action::Update => "updated",
            Action::Unchanged => "unchanged",
            Action::Remove => "removed",
        }
    }
}

/// One file of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    pub path: String,
    pub action: Action,
    /// What the disk holds now (`None`: nothing).
    pub before: Option<String>,
    /// What it will hold (`None`: removed).
    pub after: Option<String>,
}

/// Why a file is not written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    /// Something is at the path and install did not write it.
    ForeignFile,
    /// Install wrote it, and a hand edit has changed it since.
    EditedFile,
    /// A config file install merges into is no document of its format.
    UnreadableConfig,
}

impl RefusalCode {
    pub fn as_str(self) -> &'static str {
        match self {
            RefusalCode::ForeignFile => "foreign_file",
            RefusalCode::EditedFile => "edited_file",
            RefusalCode::UnreadableConfig => "unreadable_config",
        }
    }
}

/// A file the plan will not touch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub path: String,
    pub code: RefusalCode,
    pub message: String,
}

/// What one host's install would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPlan {
    pub host: Host,
    pub files: Vec<PlannedFile>,
    pub refused: Vec<Refusal>,
    pub not_rendered: Vec<NotRendered>,
}

impl HostPlan {
    /// The files whose action writes something.
    pub fn changes(&self) -> impl Iterator<Item = &PlannedFile> {
        self.files.iter().filter(|f| f.action != Action::Unchanged)
    }
}

fn refuse(path: &str, code: RefusalCode, message: String) -> Refusal {
    Refusal {
        path: path.to_string(),
        code,
        message,
    }
}

/// The plan for `tree` against `disk`, for the host `renderer` serves. `force` lets an edited file of install's be
/// rewritten or removed; a file that is not install's is refused whatever `force` says. `command` (`install` or
/// `uninstall`) is the one whose `--force` an edited-file refusal names (ADR 0306).
pub fn plan(
    renderer: &dyn HostRenderer,
    tree: &HostTree,
    disk: &dyn HostDisk,
    force: bool,
    command: &str,
) -> HostPlan {
    let host = renderer.host();
    let mut out = HostPlan {
        host,
        files: Vec::new(),
        refused: Vec::new(),
        not_rendered: tree.not_rendered.clone(),
    };
    let mut wanted = BTreeSet::new();
    for file in &tree.owned {
        wanted.insert(file.path.clone());
        if !owned_path(&file.path) {
            out.refused.push(refuse(
                &file.path,
                RefusalCode::ForeignFile,
                format!("{} is no file install may write", file.path),
            ));
            continue;
        }
        owned_file(
            &mut out,
            host,
            &file.path,
            Some(&file.text),
            disk,
            (force, command),
        );
    }
    for root in renderer.owned_roots() {
        for path in disk.files_under(&root) {
            if !wanted.contains(&path) && owned_path(&path) {
                owned_file(&mut out, host, &path, None, disk, (force, command));
            }
        }
    }
    for path in renderer.merged_paths() {
        let entries: Vec<MergedEntry> = tree
            .merged
            .iter()
            .filter(|m| m.path == path)
            .flat_map(|m| m.entries.clone())
            .collect();
        merged_file(&mut out, renderer, &path, &entries, disk);
    }
    out.files.sort_by(|a, b| a.path.cmp(&b.path));
    out.refused.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// One owned file: `want` is its text (`None`: no extension renders it any more).
fn owned_file(
    out: &mut HostPlan,
    host: Host,
    path: &str,
    want: Option<&str>,
    disk: &dyn HostDisk,
    (force, command): (bool, &str),
) {
    let now = match disk.read(path) {
        Ok(now) => now,
        Err(why) => {
            out.refused.push(refuse(
                path,
                RefusalCode::ForeignFile,
                format!("{path} is not a file install wrote: {why}"),
            ));
            return;
        }
    };
    let planned = |action, after: Option<&str>| PlannedFile {
        path: path.to_string(),
        action,
        before: now.clone(),
        after: after.map(str::to_string),
    };
    let Some(now_text) = now.as_deref() else {
        if let Some(want) = want {
            out.files.push(planned(Action::Create, Some(want)));
        }
        return;
    };
    let ours = match marker::inspect(now_text) {
        Mark::Ours {
            host: marked,
            intact,
        } if marked == host.as_str() => Some(intact),
        _ => None,
    };
    let Some(intact) = ours else {
        if want.is_some() {
            out.refused.push(refuse(
                path,
                RefusalCode::ForeignFile,
                format!(
                    "{path} exists and install did not write it: move it aside, then install again"
                ),
            ));
        }
        // A foreign file install no longer renders is none of its business.
        return;
    };
    if want == Some(now_text) {
        out.files.push(planned(Action::Unchanged, want));
        return;
    }
    if !intact && !force {
        out.refused.push(refuse(
            path,
            RefusalCode::EditedFile,
            if command == "uninstall" {
                format!(
                    "{path} was edited since install wrote it: {} removes it anyway",
                    specforge_common::remedy::command(
                        "`specforge uninstall {} --force`",
                        &[host.as_str()]
                    )
                )
            } else {
                format!(
                    "{path} was edited since install wrote it: {} overwrites the edit",
                    specforge_common::remedy::command(
                        "`specforge install {} --force`",
                        &[host.as_str()]
                    )
                )
            },
        ));
        return;
    }
    match want {
        Some(want) => out.files.push(planned(Action::Update, Some(want))),
        None => out.files.push(planned(Action::Remove, None)),
    }
}

/// One config file: `entries` are what install owns in it now (none: strip).
fn merged_file(
    out: &mut HostPlan,
    renderer: &dyn HostRenderer,
    path: &str,
    entries: &[MergedEntry],
    disk: &dyn HostDisk,
) {
    let now = match disk.read(path) {
        Ok(now) => now,
        Err(why) => {
            out.refused.push(refuse(
                path,
                RefusalCode::UnreadableConfig,
                format!("{path} cannot be read: {why}"),
            ));
            return;
        }
    };
    let merged = match renderer.merge(path, now.as_deref(), entries) {
        Ok(merged) => merged,
        Err(why) => {
            out.refused.push(refuse(
                path,
                RefusalCode::UnreadableConfig,
                format!("{path} is not a document install can merge into: {why}"),
            ));
            return;
        }
    };
    out.not_rendered.extend(merged.not_rendered);
    let action = match (&now, &merged.text) {
        (None, None) => return,
        (None, Some(_)) => Action::Create,
        (Some(_), None) => Action::Remove,
        (Some(a), Some(b)) if a == b => Action::Unchanged,
        (Some(_), Some(_)) => Action::Update,
    };
    out.files.push(PlannedFile {
        path: path.to_string(),
        action,
        before: now,
        after: merged.text,
    });
}

/// A disk in memory, for tests.
#[cfg(test)]
pub(crate) mod memory {
    use std::collections::BTreeMap;

    use super::HostDisk;

    #[derive(Default, Clone)]
    pub struct Memory(pub BTreeMap<String, String>);

    impl HostDisk for Memory {
        fn read(&self, path: &str) -> Result<Option<String>, String> {
            Ok(self.0.get(path).cloned())
        }
        fn files_under(&self, dir: &str) -> Vec<String> {
            self.0
                .keys()
                .filter(|p| p.starts_with(&format!("{dir}/")))
                .cloned()
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use specforge_protocol_types::HookEvent;

    use super::memory::Memory;
    use super::*;
    use crate::host::marker::{PLACEHOLDER, stamp};
    use crate::host::port::{
        HookAnswer, HookReadError, HostHookEvent, HostReply, Merged, MergedFile, OwnedFile,
        Rendered, Source,
    };

    /// An adapter of two directories and one JSON config that owns the key `ours`.
    struct Fake;

    impl HostRenderer for Fake {
        fn host(&self) -> Host {
            Host::ClaudeCode
        }
        fn render(&self, _: &Rendered) -> HostTree {
            HostTree::default()
        }
        fn owned_roots(&self) -> Vec<String> {
            vec![".claude/agents".into()]
        }
        fn merged_paths(&self) -> Vec<String> {
            vec![".mcp.json".into()]
        }
        fn merge(
            &self,
            _: &str,
            existing: Option<&str>,
            entries: &[MergedEntry],
        ) -> Result<Merged, String> {
            let mut doc: serde_json::Map<String, serde_json::Value> = match existing {
                Some(text) => serde_json::from_str(text).map_err(|e| e.to_string())?,
                None => Default::default(),
            };
            doc.remove("ours");
            if let Some(entry) = entries.first() {
                doc.insert("ours".into(), entry.value.clone());
            }
            let text = (!doc.is_empty()).then(|| format!("{}\n", serde_json::Value::Object(doc)));
            Ok(Merged {
                text,
                not_rendered: Vec::new(),
            })
        }
        fn event_name(&self, _: HookEvent) -> Option<&'static str> {
            None
        }
        fn read_event(&self, _: &[u8]) -> Result<HostHookEvent, HookReadError> {
            Err(HookReadError("unused".into()))
        }
        fn write_answer(&self, _: &HostHookEvent, _: &HookAnswer) -> HostReply {
            HostReply::default()
        }
    }

    fn source(id: &str) -> Source {
        Source {
            extension: "@t/core".into(),
            version: "1".into(),
            kind: "agent",
            id: id.into(),
        }
    }

    fn marked(id: &str, body: &str) -> String {
        stamp(
            Host::ClaudeCode,
            &source(id),
            &format!("---\n{PLACEHOLDER}\n{body}\n"),
        )
    }

    fn tree(files: &[(&str, String)]) -> HostTree {
        HostTree {
            owned: files
                .iter()
                .map(|(path, text)| OwnedFile {
                    path: path.to_string(),
                    text: text.clone(),
                })
                .collect(),
            merged: vec![MergedFile {
                path: ".mcp.json".into(),
                entries: vec![MergedEntry {
                    keys: vec![],
                    name: Some("ours".into()),
                    value: json!(1),
                }],
            }],
            not_rendered: Vec::new(),
        }
    }

    fn action(plan: &HostPlan, path: &str) -> Option<Action> {
        plan.files.iter().find(|f| f.path == path).map(|f| f.action)
    }

    #[test]
    fn drift_classifies_every_row() {
        let a = ".claude/agents/a.md";
        let (b, c, d, e) = (
            ".claude/agents/b.md",
            ".claude/agents/c.md",
            ".claude/agents/d.md",
            ".claude/agents/e.md",
        );
        let mut disk = Memory::default();
        disk.0.insert(a.into(), marked("a", "same"));
        disk.0.insert(b.into(), marked("b", "old"));
        disk.0
            .insert(c.into(), format!("{}\nmine", marked("c", "x")));
        disk.0.insert(d.into(), "mine\n".into());
        disk.0.insert(e.into(), marked("e", "gone"));
        let wanted = tree(&[
            (a, marked("a", "same")),
            (b, marked("b", "new")),
            (c, marked("c", "x")),
            (d, marked("d", "y")),
            (".claude/agents/f.md", marked("f", "fresh")),
        ]);
        let plan = plan(&Fake, &wanted, &disk, false, "install");
        assert_eq!(action(&plan, a), Some(Action::Unchanged));
        assert_eq!(action(&plan, b), Some(Action::Update));
        assert_eq!(action(&plan, e), Some(Action::Remove));
        assert_eq!(action(&plan, ".claude/agents/f.md"), Some(Action::Create));
        assert_eq!(action(&plan, ".mcp.json"), Some(Action::Create));
        let refused: Vec<(&str, RefusalCode)> = plan
            .refused
            .iter()
            .map(|r| (r.path.as_str(), r.code))
            .collect();
        assert_eq!(
            refused,
            [(c, RefusalCode::EditedFile), (d, RefusalCode::ForeignFile)]
        );
        // `force` rewrites an edit of install's and never a foreign file.
        let forced = super::plan(&Fake, &wanted, &disk, true, "install");
        assert_eq!(action(&forced, c), Some(Action::Update));
        assert_eq!(forced.refused.len(), 1);
        assert_eq!(forced.refused[0].code, RefusalCode::ForeignFile);
    }

    #[test]
    fn a_config_file_that_is_no_document_is_refused_and_one_of_ours_alone_goes() {
        let mut disk = Memory::default();
        disk.0.insert(".mcp.json".into(), "{ nope".into());
        let plan = plan(&Fake, &tree(&[]), &disk, false, "install");
        assert_eq!(plan.refused[0].code, RefusalCode::UnreadableConfig);
        let mut disk = Memory::default();
        disk.0.insert(".mcp.json".into(), "{\"ours\":1}\n".into());
        let none = HostTree::default();
        assert_eq!(
            action(
                &super::plan(&Fake, &none, &disk, false, "install"),
                ".mcp.json"
            ),
            Some(Action::Remove)
        );
    }

    #[test]
    fn applying_a_plan_leaves_a_plan_of_only_unchanged() {
        use proptest::prelude::*;
        proptest!(ProptestConfig::with_cases(64), |(
            bodies in proptest::collection::vec("[a-z]{0,8}", 1..5),
            stale in proptest::collection::vec(proptest::option::of("[a-z]{0,8}"), 1..5),
        )| {
            let files: Vec<(String, String)> = bodies
                .iter()
                .enumerate()
                .map(|(n, body)| (format!(".claude/agents/f{n}.md"), marked(&format!("f{n}"), body)))
                .collect();
            let mut disk = Memory::default();
            for (n, old) in stale.iter().enumerate().take(files.len()) {
                if let Some(old) = old {
                    disk.0.insert(format!(".claude/agents/f{n}.md"), marked(&format!("f{n}"), old));
                }
            }
            let wanted = tree(&files.iter().map(|(p, t)| (p.as_str(), t.clone())).collect::<Vec<_>>());
            let first = plan(&Fake, &wanted, &disk, false, "install");
            prop_assert!(first.refused.is_empty());
            for file in &first.files {
                match &file.after {
                    Some(text) => { disk.0.insert(file.path.clone(), text.clone()); }
                    None => { disk.0.remove(&file.path); }
                }
            }
            let second = plan(&Fake, &wanted, &disk, false, "install");
            prop_assert!(second.files.iter().all(|f| f.action == Action::Unchanged), "{:?}", second.files);
        });
    }
}
