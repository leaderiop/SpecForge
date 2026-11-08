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
}
