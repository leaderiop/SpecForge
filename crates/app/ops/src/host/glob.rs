//! The path globs a hook declares (ADR 0292): the one function a read's glob also uses (ADR 0293).

pub use specforge_common::path_glob::matches;

#[cfg(test)]
mod tests {
    #[test]
    fn the_hook_glob_is_the_common_glob() {
        // A hook's glob and a file read's glob are one function: they can never disagree on a path.
        let same: fn(&str, &str) -> bool = specforge_common::path_glob::matches;
        for (pattern, path) in [
            ("specs/**/*.spec", "specs/x/a.spec"),
            ("*.md", "a/b.md"),
            ("a?", "ab"),
        ] {
            assert_eq!(super::matches(pattern, path), same(pattern, path));
        }
    }
}
