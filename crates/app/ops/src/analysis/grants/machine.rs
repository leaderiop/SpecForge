//! Machine roots (ADR 0335's S1 amendment): a root the user names on this machine, by typing it, for every project's
//! verbs. No project is read, so `approve` and `revoke` work from any directory.

use std::path::{Component, Path};

use specforge_effects::{Approval, ApprovedAt, Consent, MachineRoot, RealFs, Revoke};
use specforge_project::RESERVED_ROOTS;
use specforge_protocol_types::is_scope_name;

use super::{GrantsChange, invalid, shown, store_refused};
use crate::OpError;
use crate::consent::{resolved, user_home};
use crate::settle::stamped;

/// Whether `root`'s stored target is still a directory that lands where it was approved: one swapped for a link is
/// not followed.
pub fn lands(root: &MachineRoot) -> bool {
    let target = Path::new(&root.target);
    target.is_dir() && RealFs::landing(target) == target
}

/// `specforge grants approve --root <name> --at <dir>`: name a root on this machine, for every project's verbs. No
/// project is read, so it works from any directory. `at` is read from `cwd`, a leading `~/` as the home directory, and
/// the directory is made when missing. Each refusal comes before anything is made.
pub fn approve_machine_root(
    consent: &dyn Consent,
    name: &str,
    at: &Path,
    cwd: &Path,
) -> Result<GrantsChange, OpError> {
    if RESERVED_ROOTS.contains(&name) {
        return Err(invalid(
            format!("`{}` is a root every project has", shown(name)),
            "name it something else",
        ));
    }
    if !is_scope_name(name) {
        return Err(invalid(
            format!("`{}` is not a word ([a-z][a-z0-9-]*)", shown(name)),
            "name the root with a word, as `workspaces`",
        ));
    }
    let home = user_home();
    let typed = from_home(at, home.as_deref())?;
    let wanted = resolved(&cwd.join(typed));
    let refuse_target = |target: &Path| not_a_machine_root(target, home.as_deref(), consent);
    refuse_target(&wanted)?;
    // A name that came to hold a link since `resolved` looked is judged again once it lands.
    std::fs::create_dir_all(&wanted).map_err(|e| {
        invalid(
            format!("{} cannot be made ({e})", shown(&wanted.to_string_lossy())),
            "name a directory, or a path where one can be made",
        )
    })?;
    let target = std::fs::canonicalize(&wanted).map_err(|e| {
        invalid(
            format!("{} cannot be read ({e})", shown(&wanted.to_string_lossy())),
            "name a directory, or a path where one can be made",
        )
    })?;
    refuse_target(&target)?;
    let approval = stamped(Approval::MachineRoot(MachineRoot {
        root: name.to_string(),
        target: target.display().to_string(),
        approved_at: ApprovedAt::default(),
    }));
    consent.approve(approval).map_err(|e| store_refused(&e))?;
    Ok(GrantsChange {
        changed: vec![shown(&format!("root {name} {}", target.display()))],
    })
}

/// `at` with a leading `~` read as `home`; `~user` is refused: this reads no other user's home.
fn from_home<'a>(at: &'a Path, home: Option<&Path>) -> Result<std::borrow::Cow<'a, Path>, OpError> {
    let mut parts = at.components();
    let Some(Component::Normal(first)) = parts.next() else {
        return Ok(at.into());
    };
    let first = first.to_string_lossy();
    if !first.starts_with('~') {
        return Ok(at.into());
    }
    if first != "~" {
        return Err(invalid(
            format!("{} names another user's home", shown(&at.to_string_lossy())),
            "write the directory out, or use ~/ for your own home",
        ));
    }
    let home = home.ok_or_else(|| {
        invalid(
            format!(
                "{} needs a home directory (HOME), and there is none",
                at.display()
            ),
            "write the directory out",
        )
    })?;
    Ok(home.join(parts.as_path()).into())
}

/// `Err` when a root at `target` would reach what a root must not: the home directory or what holds it, a filesystem
/// root, the host's own state, a credential directory or the store itself, which could then approve anything.
fn not_a_machine_root(
    target: &Path,
    home: Option<&Path>,
    consent: &dyn Consent,
) -> Result<(), OpError> {
    let refuse = |why: &str| {
        Err(invalid(
            format!("{} {why}", shown(&target.to_string_lossy())),
            "name a directory of its own, as ~/gsd-workspaces",
        ))
    };
    if target.parent().is_none() {
        return refuse("is a filesystem root");
    }
    if home.is_some_and(|home| resolved(home).starts_with(target)) {
        return refuse("is or holds the home directory");
    }
    let host_state = specforge_effects::contained::HOST_STATE;
    let inside_credentials = specforge_effects::contained::credential(&format!(
        "{}/x",
        target.to_string_lossy().replace('\\', "/")
    ));
    if inside_credentials
        || target
            .components()
            .any(|part| part.as_os_str().eq_ignore_ascii_case(host_state))
    {
        return refuse("lies in the host's own state or a credential directory");
    }
    let store = resolved(Path::new(&consent.store()));
    let holds_store = store.starts_with(target);
    if holds_store || store.parent().is_some_and(|dir| target.starts_with(dir)) {
        return refuse("is, holds or lies beside the consent store");
    }
    Ok(())
}

/// `specforge grants revoke --root <name> --machine`: forget the machine root `name`.
pub fn revoke_machine_root(consent: &dyn Consent, name: &str) -> Result<GrantsChange, OpError> {
    let held = consent.list().map_err(|e| store_refused(&e))?.machine_roots;
    if !held.iter().any(|a| a.root == name) {
        let names: Vec<String> = held.iter().map(|a| shown(&a.root)).collect();
        return Err(invalid(
            format!("this machine names no root `{}`", shown(name)),
            &if names.is_empty() {
                "name one with `specforge grants approve --root <name> --at <dir>`".to_string()
            } else {
                format!("its machine roots: {}", names.join(", "))
            },
        ));
    }
    consent
        .revoke(&Revoke::MachineRoot {
            root: name.to_string(),
        })
        .map_err(|e| store_refused(&e))?;
    Ok(GrantsChange {
        changed: vec![shown(&format!("root {name}"))],
    })
}
