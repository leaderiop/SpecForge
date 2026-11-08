//! Where a collect stores its facts (ADR 0327 D2): the one path rule every reader of a fact store shares. A served
//! fact's span names its store, so this rule is how a reader tells a fact from an authored block.

/// Where a collect stores its facts, from the project root (ADR 0327): `<FACTS_DIR>/<key>/<collector>.json`.
pub const FACTS_DIR: &str = ".specforge/facts";

/// The collector and the key the fact store `file` (a source key, from the project root) is filed under:
/// `.specforge/facts/<key>/<collector>.json`. `None` for any other file.
pub fn filed(file: &str) -> Option<(&str, &str)> {
    let (key, name) = file
        .strip_prefix(FACTS_DIR)?
        .strip_prefix('/')?
        .split_once('/')?;
    let collector = name
        .strip_suffix(".json")
        .filter(|name| !name.contains('/'))?;
    Some((collector, key))
}

/// Whether `file`, a source key, is a fact store ([`filed`]).
pub fn is_fact_store(file: &str) -> bool {
    filed(file).is_some()
}

/// Whether `written`, a name a collected fact holds, names the entity at `path` by the rule a fact's reference is read
/// with when nothing else answers it (ADR 0346 D4): it is `path`, or the end of it after a `/`.
pub fn names(path: &str, written: &str) -> bool {
    path == written
        || path
            .strip_suffix(written)
            .is_some_and(|head| head.ends_with('/'))
}

/// `path` written as `written` writes a name: as many trailing names as `written` has. What a rename writes where a
/// fact named the entity it moved.
pub fn respelled(path: &str, written: &str) -> String {
    let names: Vec<&str> = path.split('/').collect();
    let count = written.split('/').count().min(names.len());
    names[names.len() - count..].join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fact_store_is_filed_under_the_facts_dir_at_the_root_only() {
        assert_eq!(
            filed(".specforge/facts/worktree/node.json"),
            Some(("node", "worktree"))
        );
        for other in [
            "spec/.specforge/facts/worktree/node.json",
            ".specforge/facts/worktree/node.spec",
            ".specforge/facts/worktree/deep/node.json",
            ".specforge/facts/node.json",
        ] {
            assert!(!is_fact_store(other), "{other}");
        }
    }

    #[test]
    fn a_written_name_is_the_path_or_its_end_after_a_slash() {
        assert!(names("v1/02/TR1", "TR1") && names("v1/02/TR1", "02/TR1") && names("TR1", "TR1"));
        assert!(!names("v1/02/XTR1", "TR1") && !names("TR1", "v1/TR1"));
        assert_eq!(respelled("v1/02/TR2", "TR1"), "TR2");
        assert_eq!(respelled("v1/02/TR2", "02/TR1"), "02/TR2");
        assert_eq!(respelled("TR2", "a/b/TR1"), "TR2");
    }
}
