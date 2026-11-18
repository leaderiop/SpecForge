//! Where the runtime keeps wasmtime's compiled code: the per-user compilation cache's directory and its setup.

use std::path::{Path, PathBuf};

use wasmtime::{Cache, CacheConfig, Config};

/// Per-user Wasmtime compilation cache directory.
///
/// `$SPECFORGE_WASMTIME_CACHE`, an absolute directory, overrides; `off`
/// disables the cache, and so does a relative directory, as a relative home
/// is no home. Set empty, it is unset. Default:
/// `<home>/.cache/specforge/wasmtime`, where the home is
/// `specforge_common::home`'s (`HOME`, else `USERPROFILE`, kept only when
/// absolute; ADR 0375 D5). No cache with no home.
/// The test drivers read it, so a child running under another `HOME` shares the cache of the test process
/// (a cache of its own would compile every builtin again).
pub fn user_compile_cache_dir() -> Option<PathBuf> {
    compile_cache_dir_from(
        std::env::var_os("SPECFORGE_WASMTIME_CACHE"),
        std::env::var_os("HOME"),
        std::env::var_os("USERPROFILE"),
    )
}

/// [`user_compile_cache_dir`] over the variables' values: `cache` (`SPECFORGE_WASMTIME_CACHE`), `home` (`HOME`)
/// and `profile` (`USERPROFILE`), the home read by [`specforge_common::home::home_from`]. A relative cache or home is
/// no cache, so the directory a run starts in never holds compiled code the runtime would load.
fn compile_cache_dir_from(
    cache: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
    profile: Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    match cache.filter(|v| !v.is_empty()) {
        Some(v) if v == "off" => None,
        Some(v) => Some(PathBuf::from(v)).filter(|dir| dir.is_absolute()),
        None => specforge_common::home::home_from(home, profile)
            .map(|home| home.join(".cache").join("specforge").join("wasmtime")),
    }
}

/// Configure wasmtime's native on-disk compilation cache (true AOT: compiled
/// machine code is cached and deserialized on later runs, keyed by input
/// bytes + engine config). Covers components — `compile_component` goes
/// through wasmtime's `ModuleCacheEntry` like modules do. The soft size cap
/// keeps the governance constraint (500 MB, size-based eviction) true.
pub(crate) fn enable_compile_cache(config: &mut Config, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut cache_config = CacheConfig::new();
    cache_config.with_directory(dir);
    cache_config.with_files_total_size_soft_limit(500 * 1024 * 1024);
    let cache = Cache::new(cache_config).map_err(|e| e.to_string())?;
    config.cache(Some(cache));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::compile_cache_dir_from;

    fn abs() -> OsString {
        std::env::temp_dir().into_os_string()
    }

    fn cache_under(home: OsString) -> PathBuf {
        PathBuf::from(home)
            .join(".cache")
            .join("specforge")
            .join("wasmtime")
    }

    #[test]
    fn a_user_compile_cache_lives_under_userprofile_when_home_is_unset() {
        let profile = abs();
        assert_eq!(
            compile_cache_dir_from(None, None, Some(profile.clone())),
            Some(cache_under(profile))
        );
    }

    #[test]
    fn the_user_compile_cache_takes_home_before_userprofile() {
        let home = abs();
        let profile = PathBuf::from(abs()).join("profile").into_os_string();
        assert_eq!(
            compile_cache_dir_from(None, Some(home.clone()), Some(profile)),
            Some(cache_under(home))
        );
    }

    #[test]
    fn a_relative_home_gives_no_user_compile_cache() {
        assert_eq!(
            compile_cache_dir_from(None, Some("relative".into()), Some(abs())),
            None
        );
    }

    #[test]
    fn the_cache_variable_still_overrides_the_user_compile_cache_first() {
        assert_eq!(
            compile_cache_dir_from(Some("off".into()), Some(abs()), Some(abs())),
            None
        );
        let dir = PathBuf::from(abs()).join("cache");
        assert_eq!(
            compile_cache_dir_from(Some(dir.clone().into_os_string()), None, None),
            Some(dir)
        );
    }

    #[test]
    fn an_empty_cache_variable_is_unset() {
        let home = abs();
        assert_eq!(
            compile_cache_dir_from(Some("".into()), Some(home.clone()), None),
            Some(cache_under(home))
        );
    }

    #[test]
    fn a_relative_cache_variable_gives_no_user_compile_cache() {
        for relative in ["cache", "./cache", "../cache"] {
            assert_eq!(
                compile_cache_dir_from(Some(relative.into()), Some(abs()), Some(abs())),
                None,
                "{relative}"
            );
        }
    }

    #[test]
    fn no_home_at_all_gives_no_user_compile_cache() {
        assert_eq!(compile_cache_dir_from(None, None, None), None);
    }

    #[cfg(windows)]
    #[test]
    fn a_user_compile_cache_lives_under_a_drive_path_userprofile() {
        assert_eq!(
            compile_cache_dir_from(None, None, Some(r"C:\Users\runner".into())),
            Some(PathBuf::from(r"C:\Users\runner\.cache\specforge\wasmtime"))
        );
    }
}
