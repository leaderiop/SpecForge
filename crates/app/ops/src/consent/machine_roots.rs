//! Which directories a machine root may be, and what a store's machine roots are to one project's verbs: judged when one
//! is approved and again whenever the store is read, by one check.

use std::path::{Component, Path, PathBuf};

use specforge_common::remedy::printable;
use specforge_common::{Diagnostic, codes};
use specforge_effects::{Consent, MachineRoot};
use specforge_project::not_a_root_name;

use super::{resolved, user_home};

/// The directories under the top of a filesystem that are the system's. `/etc` and `/var` land in `/private` on macOS.
#[cfg(not(windows))]
const SYSTEM_DIRS: [&str; 16] = [
    "etc", "usr", "bin", "sbin", "var", "private", "System", "Library", "opt", "lib", "lib64",
    "boot", "dev", "proc", "sys", "run",
];
#[cfg(windows)]
const SYSTEM_DIRS: [&str; 4] = [
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
];

/// A home's directories that hold what its hidden ones hold elsewhere: keychains and tokens on macOS and Windows.
const PRIVATE_IN_HOME: [&str; 2] = ["Library", "AppData"];

/// Why no root may be at `target`: it would reach the home directory or what holds it, a filesystem root, a hidden
/// directory of the home (where credentials live, under any name), a directory of the system, the host's own state, or
/// the store itself, which could then approve anything. A home can lie anywhere (`/var/lib/ci`, `/root`), so what is
/// under it is judged as the home's and never as the system's.
pub(crate) fn not_a_machine_root(
    target: &Path,
    home: Option<&Path>,
    store: &Path,
) -> Option<&'static str> {
    if target.parent().is_none() {
        return Some("is a filesystem root");
    }
    let home = home.map(resolved);
    if home.as_deref().is_some_and(|home| home.starts_with(target)) {
        return Some("is or holds the home directory");
    }
    let first = |path: &Path| {
        path.components().find_map(|part| match part {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
    };
    let named = |list: &[&str], name: &str| list.iter().any(|dir| name.eq_ignore_ascii_case(dir));
    match home
        .as_deref()
        .and_then(|home| target.strip_prefix(home).ok())
    {
        Some(rel) => {
            if first(rel).is_some_and(|dir| dir.starts_with('.') || named(&PRIVATE_IN_HOME, &dir)) {
                return Some("is or lies in a hidden directory of the home");
            }
        }
        None => {
            if first(target).is_some_and(|dir| named(&SYSTEM_DIRS, &dir)) {
                return Some("is or lies in a directory of the system");
            }
        }
    }
    if specforge_effects::contained::withheld(&target.to_string_lossy().replace('\\', "/")) {
        return Some(
            "is named as the host's own state or a credential is, so it is no directory of its own",
        );
    }
    let store = resolved(store);
    if store.starts_with(target) || store.parent().is_some_and(|dir| target.starts_with(dir)) {
        return Some("is, holds or lies beside the consent store");
    }
    None
}

/// Why a store's entry names no machine root, whoever wrote it: approving judged it, and the store is a file.
fn refused_entry(root: &MachineRoot, home: Option<&Path>, store: &Path) -> Option<String> {
    if let Some(why) = not_a_root_name(&root.root) {
        return Some(why.to_string());
    }
    not_a_machine_root(Path::new(&root.target), home, store)
        .map(|why| format!("at its target {why}"))
}

/// What a store's machine roots are to the verbs of one project.
pub(crate) struct MachineRootsHere {
    /// The ones they reach: each stored entry judged as `approve` judges one, whose target still lands where it was
    /// approved, and whose name `taken` leaves free.
    pub(crate) reached: Vec<MachineRoot>,
    /// W186 per entry no `approve` would have taken; it is ignored.
    pub(crate) refused: Vec<Diagnostic>,
}

/// [`MachineRootsHere`] of `held`, in `consent`'s store, beside the roots `taken` that the project keeps the names of.
/// The verbs and the listing both ask here, so what a listing says is approved is what a verb reaches.
pub(crate) fn machine_roots_here(
    consent: &dyn Consent,
    held: &[MachineRoot],
    taken: &[(String, PathBuf)],
) -> MachineRootsHere {
    let home = user_home();
    let store = consent.store();
    let mut here = MachineRootsHere {
        reached: Vec::new(),
        refused: Vec::new(),
    };
    for root in held {
        if let Some(why) = refused_entry(root, home.as_deref(), Path::new(&store)) {
            here.refused.push(Diagnostic::new(
                codes::W186,
                printable(&format!(
                    "consent store {store}: the machine root `{}` {why}, so it is ignored; drop it with `specforge \
                     grants revoke --root {} --machine`",
                    root.root, root.root
                ))
                .into_owned(),
            ));
            continue;
        }
        let free = !taken.iter().any(|(name, _)| *name == root.root)
            && !here.reached.iter().any(|other| other.root == root.root);
        if free && root.lands() {
            here.reached.push(root.clone());
        }
    }
    here
}
