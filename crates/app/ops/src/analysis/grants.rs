//! `specforge grants` (ADR 0335 D2): what each loaded extension may make the host do here, this machine's approvals
//! of it, and `approve` and `revoke`. Typing `approve` is the consent: nothing is asked.

use serde::Serialize;
use specforge_common::DiagnosticList;
use specforge_common::shape::Shape;
use specforge_effects::grant::project_key;
use specforge_effects::{
    Approval, Approvals, ApprovedAt, Consent, ConsentError, Grant, Granted, KeyedBy, Provenance,
    Revoke, RootApproval, Roots, Uses,
};

use crate::analysis::collect::uses_here;
use crate::consent::unusable_store;
use crate::settle::stamped;
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind};

/// What one capability's grant is on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Shape)]
#[serde(rename_all = "kebab-case")]
pub enum GrantState {
    /// Using it needs no approval (a file or git read).
    NoneNeeded,
    /// This machine approved it, its granted scope covered.
    Approved,
    /// Using it needs an approval this machine has not given.
    NotApproved,
    /// The project's effects ceiling left nothing of it.
    NarrowedOut,
}

/// One declared capability as `specforge grants` and doctor list it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct CapabilityRow {
    /// Its scope name.
    pub name: String,
    /// `http`, `fs`, `git` or `process`.
    pub port: String,
    /// What the ceiling left of it, in one line; the declared scope when it left nothing.
    pub scope: String,
    pub why: String,
    /// The call kinds it is declared for; none: every kind the gate lets use its port.
    pub call_kinds: Vec<String>,
    /// What the project's effects ceiling removed (W171).
    pub narrowed: Vec<String>,
    /// A run here uses it: a detected collector's command.
    pub in_use: bool,
    pub state: GrantState,
    /// When this machine approved it (RFC 3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
}

/// One loaded extension's capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct ExtensionGrants {
    pub extension: String,
    /// `builtin`, or `sha256:<digest>` for a third-party blob: what an approval is keyed by.
    pub provenance: String,
    pub capabilities: Vec<CapabilityRow>,
}

/// One named root of `specforge.json` that resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct RootRow {
    pub name: String,
    /// Its canonical target.
    pub target: String,
    /// Under the project root: it needs no approval.
    pub inside: bool,
    /// An outside root this machine approved.
    pub approved: bool,
}

/// `specforge grants`'s listing of one project.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct GrantsListing {
    pub extensions: Vec<ExtensionGrants>,
    pub roots: Vec<RootRow>,
    /// W186 when the store cannot be used; W170 per extension a run here needs approvals of.
    pub diagnostics: DiagnosticList,
}

/// One stored capability approval, as `specforge grants --store` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct StoredCapability {
    pub project: String,
    pub extension: String,
    pub provenance: String,
    pub scope: String,
    /// The granted scope approved, in one line.
    pub granted: String,
    pub approved_at: String,
}

/// One stored root approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct StoredRoot {
    pub project: String,
    pub root: String,
    pub target: String,
    pub approved_at: String,
}

/// Every approval the store holds, for every project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct StoreListing {
    pub capabilities: Vec<StoredCapability>,
    pub roots: Vec<StoredRoot>,
}

/// What `approve` or `revoke` changed: `<extension> <scope>` or `root <name>`, one per approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct GrantsChange {
    pub changed: Vec<String>,
}

/// What `approve` or `revoke` names.
#[derive(Debug, Clone, Copy)]
pub enum Change<'a> {
    /// An extension's scopes; none named: every one (that needs approval, for `approve`).
    Scopes {
        extension: &'a str,
        scopes: &'a [String],
    },
    /// A named root of `specforge.json`.
    Root(&'a str),
    /// Every approval of the project (`revoke` only).
    Project,
}

/// Where `extension`'s code came from: as the runtime loaded it, else as the load recorded it. One loaded with
/// neither (an environment assembled in memory) is keyed as a third party by its name, so no builtin approval
/// meets it.
pub fn provenance_of(view: &ProjectView, extension: &str) -> Provenance {
    view.runtime()
        .and_then(|runtime| runtime.provenance(extension))
        .or_else(|| {
            view.env()
                .enabled()
                .iter()
                .find(|e| e.name == extension)
                .and_then(|e| e.provenance.clone())
        })
        .unwrap_or_else(|| Provenance::third_party(extension.as_bytes()))
}

/// Every loaded extension's grant in `view`'s project that declares a capability, in declaration order.
pub fn grants_of(view: &ProjectView, consent: &dyn Consent) -> Result<Vec<Grant>, OpError> {
    let root = view.project_root()?;
    let env = view.env();
    let roots = env
        .effects_allowed()
        .with_named(Roots::new(root, env.spec_root()));
    Ok(view
        .registries()
        .declarations()
        .iter()
        .filter(|declaration| !declaration.sound_capabilities().is_empty())
        .map(|declaration| {
            let provenance = provenance_of(view, declaration.name());
            Grant::decide(
                declaration,
                &provenance,
                &env.effects_allowed().ceiling,
                &roots,
                consent,
            )
        })
        .collect())
}

/// `specforge grants`: each loaded extension's capabilities and their grant here, then the named roots; W186 when
/// the store cannot be used, and W170 per extension whose capabilities a run here uses unapproved.
pub fn list(view: &ProjectView, consent: &dyn Consent) -> Result<GrantsListing, OpError> {
    let root = view.project_root()?;
    let mut diagnostics = Vec::new();
    let held = consent.list().unwrap_or_else(|error| {
        diagnostics.push(unusable_store(&error));
        Approvals::default()
    });
    let uses = uses_here(view.registries().declarations(), root);
    let none = Uses::new();
    let extensions = grants_of(view, consent)?
        .iter()
        .map(|grant| {
            let used = uses
                .iter()
                .find(|(extension, _)| extension == grant.extension())
                .map_or(&none, |(_, uses)| uses);
            // Only what a run here uses is a warning (O11); the store's W186 says the rest.
            if grant.store_problem().is_none() {
                diagnostics.extend(grant.unapproved_summary(used));
            }
            ExtensionGrants {
                extension: grant.extension().to_string(),
                provenance: grant
                    .provenance()
                    .map_or_else(String::new, |p| keyed(&KeyedBy::of(p))),
                capabilities: grant
                    .capabilities()
                    .iter()
                    .map(|granted| row(grant, granted, used, &held))
                    .collect(),
            }
        })
        .collect();
    let project = project_key(root);
    let roots = view
        .env()
        .effects_allowed()
        .roots
        .iter()
        .filter_map(|named| {
            let target = named.target.as_ref()?.display().to_string();
            let approved = !named.inside
                && held.covers(&Approval::Root(RootApproval {
                    project: project.clone(),
                    root: named.name.clone(),
                    target: target.clone(),
                    approved_at: ApprovedAt::default(),
                }));
            Some(RootRow {
                name: named.name.clone(),
                target,
                inside: named.inside,
                approved,
            })
        })
        .collect();
    Ok(GrantsListing {
        extensions,
        roots,
        diagnostics: DiagnosticList::from(diagnostics),
    })
}

fn row(grant: &Grant, granted: &Granted, uses: &Uses, held: &Approvals) -> CapabilityRow {
    let declared = &granted.declared;
    let state = match (&granted.scope, granted.needs_approval, granted.approved) {
        (None, ..) => GrantState::NarrowedOut,
        (Some(_), false, _) => GrantState::NoneNeeded,
        (Some(_), true, true) => GrantState::Approved,
        (Some(_), true, false) => GrantState::NotApproved,
    };
    let approved_at = (state == GrantState::Approved)
        .then(|| grant.key(granted))
        .flatten()
        .and_then(|key| held.capabilities.iter().find(|a| a.covers(&key)))
        .map(|a| a.approved_at.0.clone());
    CapabilityRow {
        name: declared.name.clone(),
        port: declared.port.port().to_string(),
        scope: granted.summary(),
        why: declared.why.clone(),
        call_kinds: declared
            .call_kinds
            .iter()
            .map(ToString::to_string)
            .collect(),
        narrowed: granted.removed.iter().map(|r| r.what.clone()).collect(),
        in_use: uses.iter().any(|(_, name)| *name == declared.name),
        state,
        approved_at,
    }
}

fn keyed(keyed: &KeyedBy) -> String {
    match keyed {
        KeyedBy::Builtin => "builtin".to_string(),
        KeyedBy::Digest(digest) => format!("sha256:{digest}"),
    }
}

/// `specforge grants --store`: every approval the store holds, for every project.
pub fn list_store(consent: &dyn Consent) -> Result<StoreListing, OpError> {
    let held = consent.list().map_err(|e| store_refused(&e))?;
    Ok(StoreListing {
        capabilities: held
            .capabilities
            .into_iter()
            .map(|a| StoredCapability {
                granted: a.summary(),
                provenance: keyed(&a.provenance),
                project: a.project.0,
                extension: a.extension,
                scope: a.scope,
                approved_at: a.approved_at.0,
            })
            .collect(),
        roots: held
            .roots
            .into_iter()
            .map(|a| StoredRoot {
                project: a.project.0,
                root: a.root,
                target: a.target,
                approved_at: a.approved_at.0,
            })
            .collect(),
    })
}

/// `specforge grants approve`: store `which`'s approvals for `view`'s project, dated now. An extension with no
/// scope named approves every scope of it that needs approval; a named scope that needs none stores nothing.
pub fn approve(
    view: &ProjectView,
    consent: &dyn Consent,
    which: Change<'_>,
) -> Result<GrantsChange, OpError> {
    let root = view.project_root()?;
    let mut approvals = Vec::new();
    let mut changed = Vec::new();
    match which {
        Change::Scopes { extension, scopes } => {
            let grant = grants_of(view, consent)?
                .into_iter()
                .find(|grant| grant.extension() == extension)
                .ok_or_else(|| unknown_extension(view, extension))?;
            let declared: Vec<&str> = grant
                .capabilities()
                .iter()
                .map(|g| g.declared.name.as_str())
                .collect();
            if let Some(scope) = scopes.iter().find(|s| !declared.contains(&s.as_str())) {
                return Err(unknown_scope(extension, scope, &declared));
            }
            for granted in grant.capabilities() {
                let name = &granted.declared.name;
                if !(scopes.is_empty() || scopes.contains(name)) || !granted.needs_approval {
                    continue;
                }
                let Some(key) = grant.key(granted) else {
                    if scopes.is_empty() {
                        continue;
                    }
                    return Err(invalid(
                        format!(
                            "the effects ceiling in specforge.json leaves nothing of {extension}'s \
                             `{name}` to approve"
                        ),
                        "widen `effects` in specforge.json first",
                    ));
                };
                approvals.push(Approval::Capability(key));
                changed.push(format!("{extension} {name}"));
            }
        }
        Change::Root(name) => {
            let named = view
                .env()
                .effects_allowed()
                .roots
                .iter()
                .find(|r| r.name == name)
                .ok_or_else(|| unknown_root(view, name))?;
            let Some(target) = &named.target else {
                return Err(invalid(
                    format!("root `{name}` names no directory (E109)"),
                    "fix its path in specforge.json",
                ));
            };
            if !named.inside {
                approvals.push(Approval::Root(RootApproval {
                    project: project_key(root),
                    root: name.to_string(),
                    target: target.display().to_string(),
                    approved_at: ApprovedAt::default(),
                }));
                changed.push(format!("root {name}"));
            }
        }
        Change::Project => {
            return Err(invalid(
                "approve names an extension or a root".to_string(),
                "run `specforge grants` to see what may be approved",
            ));
        }
    }
    if !approvals.is_empty() {
        let mut approvals = Some(approvals);
        consent
            .update(&mut |held| {
                for approval in approvals.take().into_iter().flatten() {
                    held.add(stamped(approval));
                }
            })
            .map_err(|e| store_refused(&e))?;
    }
    Ok(GrantsChange { changed })
}

/// `specforge grants revoke`: drop `which`'s approvals of `view`'s project. An extension or root is known when
/// the project declares it or the store holds an approval of it here.
pub fn revoke(
    view: &ProjectView,
    consent: &dyn Consent,
    which: Change<'_>,
) -> Result<GrantsChange, OpError> {
    let project = project_key(view.project_root()?);
    let held = consent.list().map_err(|e| store_refused(&e))?;
    let revoke = match which {
        Change::Scopes { extension, scopes } => {
            let mut known: Vec<&str> = held
                .capabilities
                .iter()
                .filter(|a| a.project == project && a.extension == extension)
                .map(|a| a.scope.as_str())
                .collect();
            let declaration = view
                .registries()
                .declarations()
                .iter()
                .find(|d| d.name() == extension);
            if declaration.is_none() && known.is_empty() {
                return Err(unknown_extension(view, extension));
            }
            if let Some(declaration) = declaration {
                known.extend(declaration.capabilities.iter().map(|c| c.name.as_str()));
            }
            if let Some(scope) = scopes.iter().find(|s| !known.contains(&s.as_str())) {
                return Err(unknown_scope(extension, scope, &known));
            }
            Revoke::Scopes {
                project: project.clone(),
                extension: extension.to_string(),
                scopes: scopes.to_vec(),
            }
        }
        Change::Root(name) => {
            let stored = held
                .roots
                .iter()
                .any(|a| a.project == project && a.root == name);
            if !stored
                && !view
                    .env()
                    .effects_allowed()
                    .roots
                    .iter()
                    .any(|r| r.name == name)
            {
                return Err(unknown_root(view, name));
            }
            Revoke::Root {
                project: project.clone(),
                root: name.to_string(),
            }
        }
        Change::Project => Revoke::Project {
            project: project.clone(),
        },
    };
    let mut changed = Vec::new();
    consent
        .update(&mut |held| {
            changed = revoked(held, &revoke);
            held.remove(&revoke);
        })
        .map_err(|e| store_refused(&e))?;
    Ok(GrantsChange { changed })
}

/// What `revoke` drops of `held`, as `GrantsChange` names it.
fn revoked(held: &Approvals, revoke: &Revoke) -> Vec<String> {
    let mut kept = held.clone();
    kept.remove(revoke);
    let capabilities = held
        .capabilities
        .iter()
        .filter(|a| !kept.capabilities.contains(a))
        .map(|a| format!("{} {}", a.extension, a.scope));
    let roots = held
        .roots
        .iter()
        .filter(|a| !kept.roots.contains(a))
        .map(|a| format!("root {}", a.root));
    capabilities.chain(roots).collect()
}

fn invalid(message: String, suggestion: &str) -> OpError {
    OpError::new(OpErrorKind::InvalidInput, "invalid_input", message).with_suggestion(suggestion)
}

fn unknown_extension(view: &ProjectView, extension: &str) -> OpError {
    let declaring: Vec<&str> = view
        .registries()
        .declarations()
        .iter()
        .filter(|d| !d.sound_capabilities().is_empty())
        .map(|d| d.name())
        .collect();
    OpError::new(
        OpErrorKind::ExtensionNotFound,
        "extension_not_found",
        format!("'{extension}' is not a loaded extension that declares capabilities"),
    )
    .with_suggestion(if declaring.is_empty() {
        "no loaded extension declares a capability".to_string()
    } else {
        format!("these do: {}", declaring.join(", "))
    })
}

fn unknown_scope(extension: &str, scope: &str, known: &[&str]) -> OpError {
    invalid(
        format!("{extension} declares no capability `{scope}`"),
        &format!("its scopes: {}", known.join(", ")),
    )
}

fn unknown_root(view: &ProjectView, name: &str) -> OpError {
    let names: Vec<&str> = view
        .env()
        .effects_allowed()
        .roots
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    invalid(
        format!("specforge.json names no root `{name}`"),
        &if names.is_empty() {
            "name it under `roots` in specforge.json".to_string()
        } else {
            format!("its roots: {}", names.join(", "))
        },
    )
}

/// The consent store could not be read or written: nothing was changed.
fn store_refused(error: &ConsentError) -> OpError {
    let diagnostic = unusable_store(error);
    OpError::new(
        OpErrorKind::PreconditionFailed,
        "consent_store_unusable",
        diagnostic.message,
    )
    .with_suggestion("fix or move the store it names; nothing was changed")
}
