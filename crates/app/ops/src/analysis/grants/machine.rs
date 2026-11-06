//! Machine roots (ADR 0335's S1 amendment): a root the user names on this machine, by typing it, for every project's
//! verbs. No project is read, so `approve` and `revoke` work from any directory.

use std::path::{Component, Path};

use specforge_effects::{Approval, ApprovedAt, Consent, MachineRoot, Revoke};
use specforge_project::{ROOT_NAME_MAX, not_a_root_name};

use super::{GrantsChange, invalid, shown, store_refused};
use crate::OpError;
use crate::consent::{not_a_machine_root, resolved, user_home};
use crate::settle::stamped;

/// `specforge grants approve --root <name> --at <dir>`: name a root on this machine, for every project's verbs. No
/// project is read, so it works from any directory. `at` is read from `cwd`, a leading `~/` as the home directory, and
/// the directory is made when missing. Each refusal comes before anything is made.
pub fn approve_machine_root(
    consent: &dyn Consent,
    name: &str,
    at: &Path,
    cwd: &Path,
) -> Result<GrantsChange, OpError> {
    if let Some(why) = not_a_root_name(name) {
        let start: String = name.chars().take(ROOT_NAME_MAX).collect();
        return Err(invalid(
            format!("`{}` {why}", shown(&start)),
            "name the root with a word of its own, as `workspaces`",
        ));
    }
    // A store that would be refused is refused before the directory is made.
    consent.list().map_err(|e| store_refused(&e))?;
    let home = user_home();
    let typed = from_home(at, home.as_deref())?;
    let wanted = resolved(&cwd.join(typed));
    let store = consent.store();
    let refuse_target =
        |target: &Path| match not_a_machine_root(target, home.as_deref(), Path::new(&store)) {
            Some(why) => Err(invalid(
                format!("{} {why}", shown(&target.to_string_lossy())),
                "name a directory of its own, as ~/gsd-workspaces",
            )),
            None => Ok(()),
        };
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
