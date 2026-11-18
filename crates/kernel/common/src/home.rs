//! The user's home directory: `HOME`, else `USERPROFILE` (Windows), kept only when absolute (ADR 0375 D5). The one
//! home of that rule: a loose root's effect journal and the consent store (`specforge_effects::journal::state_base`)
//! and the compile cache (`specforge_component::user_compile_cache_dir`) all read it here. A relative home is no
//! home, so the directory a run starts in never holds what belongs under the user's.

use std::ffi::OsString;
use std::path::PathBuf;

/// The user's home directory; none when `HOME` and `USERPROFILE` are unset or the one read is relative.
pub fn user_home() -> Option<PathBuf> {
    home_from(std::env::var_os("HOME"), std::env::var_os("USERPROFILE"))
}

/// [`user_home`] over the variables' values: `home` (`HOME`), else `profile` (`USERPROFILE`), absolute only. A
/// caller's tests pass values here rather than set the process environment, which every other test in the process
/// reads.
pub fn home_from(home: Option<OsString>, profile: Option<OsString>) -> Option<PathBuf> {
    home.or(profile)
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
}

#[cfg(test)]
mod tests {
    use super::home_from;

    #[test]
    fn user_home_falls_back_to_userprofile_when_home_is_unset() {
        let profile = std::env::temp_dir();
        assert_eq!(
            home_from(None, Some(profile.clone().into_os_string())),
            Some(profile)
        );
    }

    #[test]
    fn user_home_takes_home_before_userprofile() {
        let home = std::env::temp_dir();
        let profile = home.join("profile");
        assert_eq!(
            home_from(
                Some(home.clone().into_os_string()),
                Some(profile.into_os_string())
            ),
            Some(home)
        );
    }

    #[test]
    fn a_relative_home_is_no_user_home() {
        let profile = std::env::temp_dir().into_os_string();
        assert_eq!(home_from(Some("relative".into()), Some(profile)), None);
    }

    #[test]
    fn no_variable_is_no_user_home() {
        assert_eq!(home_from(None, None), None);
    }

    #[cfg(windows)]
    #[test]
    fn user_home_reads_a_drive_path_userprofile() {
        assert_eq!(
            home_from(None, Some(r"C:\Users\runner".into())),
            Some(std::path::PathBuf::from(r"C:\Users\runner"))
        );
    }
}
