//! `Contained`: a written path in the root it names, lexically, for real, and against the listings.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use specforge_effects::contained::{Contained, Lookup, RealFs, Refused, withheld};
use specforge_effects::{Root, Roots};

/// A disk with no links, by directory listing; it records every directory it lists.
#[derive(Default)]
struct Listings {
    dirs: BTreeMap<PathBuf, Vec<String>>,
    asked: RefCell<Vec<PathBuf>>,
}

impl Listings {
    fn with(mut self, dir: &str, names: &[&str]) -> Self {
        self.dirs
            .insert(dir.into(), names.iter().map(|n| (*n).to_string()).collect());
        self
    }
}

impl Lookup for Listings {
    fn canonical(&self, _path: &Path) -> Option<PathBuf> {
        None
    }

    fn names(&self, dir: &Path) -> Option<Vec<String>> {
        self.asked.borrow_mut().push(dir.to_path_buf());
        self.dirs.get(dir).cloned()
    }
}

fn roots() -> Roots {
    Roots::new("/p", "/p/spec").with_named("vendor", "/v")
}

fn disk() -> Listings {
    Listings::default()
        .with("/p", &["spec", "README.md", "docs"])
        .with("/p/docs", &["guide.md"])
        .with("/p/spec", &["Readme.md", "a.spec", "sub"])
        .with("/p/spec/sub", &["deep.md"])
        .with("/v", &["lib.md"])
}

#[test]
fn contained_resolves_in_the_default_root_and_in_a_named_prefix() {
    let (roots, disk) = (roots(), disk());
    let contained = Contained::new(&roots, &disk);
    let spec = contained.resolve("sub/deep.md", Root::Spec).unwrap();
    assert_eq!(
        (spec.root.clone(), spec.rel.as_str()),
        (Root::Spec, "sub/deep.md")
    );
    assert_eq!(spec.at, Path::new("/p/spec/sub/deep.md"));
    assert!(spec.exists && spec.case.is_none() && !spec.backslashes && !spec.withheld);
    let project = contained
        .resolve("<project>/README.md", Root::Spec)
        .unwrap();
    assert_eq!(
        (project.root, project.at),
        (Root::Project, PathBuf::from("/p/README.md"))
    );
    let back = contained.resolve("<spec>/a.spec", Root::Project).unwrap();
    assert_eq!(back.at, Path::new("/p/spec/a.spec"));
    let named = contained.resolve("<vendor>/lib.md", Root::Spec).unwrap();
    assert!(named.exists && named.root == Root::Named("vendor".into()));
}

#[test]
fn contained_refuses_an_unknown_root() {
    let (roots, disk) = (roots(), disk());
    let contained = Contained::new(&roots, &disk);
    assert_eq!(
        contained.resolve("<nowhere>/a.md", Root::Spec),
        Err(Refused::UnknownRoot("nowhere".into()))
    );
}

#[test]
fn contained_refuses_an_absolute_path_a_climb_and_nothing() {
    let (roots, disk) = (roots(), disk());
    let contained = Contained::new(&roots, &disk);
    for absolute in [
        "/etc/passwd",
        "C:/Windows",
        "c:\\x",
        "//server/share/x",
        "<project>//etc",
    ] {
        assert_eq!(
            contained.resolve(absolute, Root::Spec),
            Err(Refused::Absolute),
            "{absolute}"
        );
    }
    for climbs in ["../README.md", "sub/../../x", "<project>/../outside"] {
        assert_eq!(
            contained.resolve(climbs, Root::Spec),
            Err(Refused::Climbs),
            "{climbs}"
        );
    }
    for empty in ["", ".", "./", "sub/.."] {
        assert_eq!(
            contained.resolve(empty, Root::Spec),
            Err(Refused::Empty),
            "{empty}"
        );
    }
    assert!(
        disk.asked.borrow().is_empty(),
        "a refused path lists nothing"
    );
}

#[test]
fn contained_keeps_a_climb_that_stays_inside() {
    let (roots, disk) = (roots(), disk());
    let contained = Contained::new(&roots, &disk);
    let resolved = contained.resolve("sub/./../a.spec", Root::Spec).unwrap();
    assert_eq!(resolved.rel, "a.spec");
    assert!(resolved.exists);
}

#[test]
fn contained_reads_a_backslash_as_a_slash() {
    let (roots, disk) = (roots(), disk());
    let contained = Contained::new(&roots, &disk);
    let resolved = contained.resolve("sub\\deep.md", Root::Spec).unwrap();
    assert_eq!(resolved.rel, "sub/deep.md");
    assert!(resolved.backslashes && resolved.exists);
}

#[test]
fn contained_compares_case_with_the_listing() {
    let roots = roots();
    let disk = disk().with("/p/spec/twins", &["NOTE.md", "note.md"]);
    let contained = Contained::new(&roots, &disk);
    let exact = contained.resolve("Readme.md", Root::Spec).unwrap();
    assert!(exact.exists && exact.case.is_none());
    let variant = contained.resolve("SUB/DEEP.MD", Root::Spec).unwrap();
    assert!(variant.exists, "{variant:?}");
    assert_eq!(variant.case.as_deref(), Some("sub/deep.md"));
    let single = contained.resolve("README.md", Root::Spec).unwrap();
    assert_eq!(single.case.as_deref(), Some("Readme.md"));
    let twins = contained.resolve("twins/Note.md", Root::Spec).unwrap();
    assert!(
        !twins.exists && twins.case.is_none(),
        "two variants are no match: {twins:?}"
    );
    let missing = contained.resolve("nope.md", Root::Spec).unwrap();
    assert!(!missing.exists);
}

#[test]
fn contained_withholds_a_credential_name() {
    let (roots, disk) = (roots(), disk());
    let contained = Contained::new(&roots, &disk);
    assert!(contained.resolve(".env", Root::Project).unwrap().withheld);
    assert!(
        contained
            .resolve("keys/id_rsa.pub", Root::Project)
            .unwrap()
            .withheld
    );
    assert!(withheld(".git/config") && withheld("a/.git/config") && !withheld("git/config"));
}

#[test]
fn contained_spec_root_outside_the_project_is_flagged() {
    assert!(!Roots::new("/p", "/p/spec").spec_outside());
    assert!(Roots::new("/p", "/q/spec").spec_outside());
}

#[test]
fn contained_suggests_only_from_a_held_parent() {
    let (roots, disk) = (roots(), disk());
    let contained = Contained::new(&roots, &disk);
    let typo = contained.resolve("sub/dep.md", Root::Spec).unwrap();
    assert!(!typo.exists);
    assert_eq!(contained.sibling(&typo).as_deref(), Some("sub/deep.md"));
    let top = contained.resolve("a.spek", Root::Spec).unwrap();
    assert_eq!(contained.sibling(&top).as_deref(), Some("a.spec"));
    let before = disk.asked.borrow().len();
    assert!(contained.resolve("../docs/guid.md", Root::Spec).is_err());
    assert_eq!(
        disk.asked.borrow().len(),
        before,
        "a refused path lists nothing"
    );
    assert!(!disk.asked.borrow().contains(&PathBuf::from("/p/docs")));
}

#[cfg(unix)]
mod real {
    use super::*;

    fn project() -> (tempfile::TempDir, Roots) {
        let dir = tempfile::TempDir::new().unwrap();
        let base = dir.path().canonicalize().unwrap();
        std::fs::create_dir_all(base.join("proj/spec")).unwrap();
        std::fs::create_dir_all(base.join("outside")).unwrap();
        std::fs::write(base.join("outside/secret.txt"), "s").unwrap();
        std::fs::write(base.join("proj/spec/a.md"), "a").unwrap();
        let roots = Roots::new(base.join("proj"), base.join("proj/spec"));
        (dir, roots)
    }

    #[test]
    fn contained_refuses_a_link_out_and_a_dangling_link_out() {
        let (dir, roots) = project();
        let base = dir.path().canonicalize().unwrap();
        let spec = base.join("proj/spec");
        std::os::unix::fs::symlink(base.join("outside/secret.txt"), spec.join("out.md")).unwrap();
        std::os::unix::fs::symlink(base.join("outside/none.txt"), spec.join("dangling.md"))
            .unwrap();
        std::os::unix::fs::symlink("a.md", spec.join("in.md")).unwrap();
        let contained = Contained::new(&roots, &RealFs);
        assert!(matches!(
            contained.resolve("out.md", Root::Spec),
            Err(Refused::LinksOut { .. })
        ));
        assert!(matches!(
            contained.resolve("dangling.md", Root::Spec),
            Err(Refused::LinksOut { landed }) if landed == base.join("outside/none.txt")
        ));
        let inside = contained.resolve("in.md", Root::Spec).unwrap();
        assert!(inside.exists);
        assert_eq!(inside.real.as_deref(), Some(spec.join("a.md").as_path()));
    }

    #[test]
    fn contained_ends_a_link_loop() {
        let (dir, roots) = project();
        let spec = dir.path().canonicalize().unwrap().join("proj/spec");
        std::os::unix::fs::symlink("b.md", spec.join("a-loop.md")).unwrap();
        std::os::unix::fs::symlink("a-loop.md", spec.join("b.md")).unwrap();
        let contained = Contained::new(&roots, &RealFs);
        let looped = contained.resolve("a-loop.md", Root::Spec).unwrap();
        assert!(!looped.exists, "{looped:?}");
    }
}
