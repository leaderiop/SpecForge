//! Where a collect stores its facts (ADR 0327 D2): the one path rule every reader of a fact store shares. A served
//! fact's span names its store, so this rule is how a reader tells a fact from an authored block.

/// Where a collect stores its facts, from the project root (ADR 0327): `<FACTS_DIR>/<key>/<collector>.json`.
pub const FACTS_DIR: &str = ".specforge/facts";

/// The collector and the key the fact store `file` (a source key, from the project root) is filed under:
/// `.specforge/facts/<key>/<collector>.json`, the key a store key ([`is_store_key`]) and the collector a collector's
/// name ([`is_collector_name`]). `None` for any other file.
pub fn filed(file: &str) -> Option<(&str, &str)> {
    let (key, name) = file
        .strip_prefix(FACTS_DIR)?
        .strip_prefix('/')?
        .split_once('/')?;
    let collector = name.strip_suffix(".json")?;
    (is_store_key(key) && is_collector_name(collector)).then_some((collector, key))
}

/// Whether `key` is a key a collect files its facts under (ADR 0327 D3): `worktree`, or a commit's sha (40 or 64
/// lowercase hex characters) with an optional `-dirty`. What a ledger line names is made a path only when it is one.
pub fn is_store_key(key: &str) -> bool {
    let sha = key.strip_suffix("-dirty").unwrap_or(key);
    key == "worktree"
        || (matches!(sha.len(), 40 | 64)
            && sha.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

/// Whether `name` can name a collector's store file: not empty, not hidden (no leading `.`), and holding no `/`, `\`
/// or NUL, so `<collector>.json` is one file name under its key's directory.
pub fn is_collector_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', '\0'])
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
    fn a_store_key_is_worktree_or_a_sha_with_an_optional_dirty() {
        let sha1 = "0123456789abcdef0123456789abcdef01234567";
        let sha256 = "0123456789abcdef".repeat(4);
        for key in [
            "worktree",
            sha1,
            &format!("{sha1}-dirty"),
            &sha256,
            &format!("{sha256}-dirty"),
        ] {
            assert!(is_store_key(key), "{key}");
            assert!(
                is_fact_store(&format!("{FACTS_DIR}/{key}/node.json")),
                "{key}"
            );
        }
        for key in [
            "",
            "k",
            "..",
            "../../x",
            "worktree-dirty",
            "-dirty",
            &sha1.to_uppercase(),
            &sha1[1..],
            &format!("{sha1}0"),
            &format!("{sha1}-dirty-dirty"),
            &format!("{}g", &sha1[1..]),
        ] {
            assert!(!is_store_key(key), "{key}");
            assert!(
                filed(&format!("{FACTS_DIR}/{key}/node.json")).is_none(),
                "{key}"
            );
        }
    }

    #[test]
    fn a_collector_name_is_one_plain_file_name() {
        for name in ["node", "node-test", "@scope_x", "a.b"] {
            assert!(is_collector_name(name), "{name}");
        }
        for name in ["", ".c", "..", "../c", "a/b", "a\\b", "a\0b"] {
            assert!(!is_collector_name(name), "{name:?}");
            assert!(
                filed(&format!("{FACTS_DIR}/worktree/{name}.json")).is_none(),
                "{name:?}"
            );
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
