//! Where an extension to add comes from, as `specforge add` is given it.

use specforge_common::{Diagnostic, codes};
use std::path::PathBuf;

/// Parsed extension specifier from a project configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionSpecifier {
    Registry { name: String, version: String },
    Local { path: PathBuf },
    Git { url: String, rev: Option<String> },
}

/// Parse an extension specifier string into a structured type.
///
/// Formats:
/// - `name@version` → Registry { name, version }
/// - `./path/to/ext` or `/absolute/path` → Local { path }
/// - `git+https://...` optionally `#rev` → Git { url, rev }
pub fn parse_extension_specifier(input: &str) -> Result<ExtensionSpecifier, Diagnostic> {
    let input = input.trim();

    if input.is_empty() {
        return Err(
            Diagnostic::new(codes::E054, "empty extension specifier".to_string()).with_suggestion(
                "provide a specifier like '@scope/name@1.0.0', './local/path', or 'git+https://...'"
                    .to_string(),
            ),
        );
    }

    // Git specifier
    if let Some(rest) = input.strip_prefix("git+") {
        let (url, rev) = if let Some(hash_pos) = rest.rfind('#') {
            let url = &rest[..hash_pos];
            let rev = &rest[hash_pos + 1..];
            (url.to_string(), Some(rev.to_string()))
        } else {
            (rest.to_string(), None)
        };
        return Ok(ExtensionSpecifier::Git { url, rev });
    }

    // Local path specifier
    if input.starts_with("./") || input.starts_with("../") || input.starts_with('/') {
        return Ok(ExtensionSpecifier::Local {
            path: PathBuf::from(input),
        });
    }

    // Registry specifier: name@version
    if let Some(at_pos) = input.rfind('@') {
        // Handle scoped packages like @scope/name@version
        // Don't split at position 0 (that's the scope prefix @)
        if at_pos > 0 {
            let name = &input[..at_pos];
            let version = &input[at_pos + 1..];
            if !version.is_empty() {
                return Ok(ExtensionSpecifier::Registry {
                    name: name.to_string(),
                    version: version.to_string(),
                });
            }
        }
    }

    Err(Diagnostic::new(
        codes::E054,
        format!("invalid extension specifier: '{}'", input),
    )
    .with_suggestion(
        "use format: 'name@version', './local/path', or 'git+https://...'".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse_extension_specifier --

    // B:parse_extension_specifier — verify unit "parses name@version registry specifier"
    #[test]
    fn test_parses_registry_specifier() {
        let spec = parse_extension_specifier("@specforge/software@1.0.0").unwrap();
        assert_eq!(
            spec,
            ExtensionSpecifier::Registry {
                name: "@specforge/software".to_string(),
                version: "1.0.0".to_string(),
            }
        );
    }

    // B:parse_extension_specifier — verify unit "parses local path specifier"
    #[test]
    fn test_parses_local_path_specifier() {
        let spec = parse_extension_specifier("./extensions/custom").unwrap();
        assert_eq!(
            spec,
            ExtensionSpecifier::Local {
                path: PathBuf::from("./extensions/custom"),
            }
        );

        let abs = parse_extension_specifier("/absolute/path/ext").unwrap();
        assert_eq!(
            abs,
            ExtensionSpecifier::Local {
                path: PathBuf::from("/absolute/path/ext"),
            }
        );
    }

    // B:parse_extension_specifier — verify unit "parses git+https specifier with optional rev"
    #[test]
    fn test_parses_git_specifier() {
        let spec = parse_extension_specifier("git+https://github.com/org/ext").unwrap();
        assert_eq!(
            spec,
            ExtensionSpecifier::Git {
                url: "https://github.com/org/ext".to_string(),
                rev: None,
            }
        );

        let with_rev = parse_extension_specifier("git+https://github.com/org/ext#v1.0.0").unwrap();
        assert_eq!(
            with_rev,
            ExtensionSpecifier::Git {
                url: "https://github.com/org/ext".to_string(),
                rev: Some("v1.0.0".to_string()),
            }
        );
    }

    /// What P1 makes of every input of plan 12's table (§2.2) today: the
    /// module goes with T2.
    #[specforge_test_macros::test(
        behavior = "parse_extension_specifier",
        verify = "each add argument reads as one extension source"
    )]
    fn parse_extension_specifier_reads_each_input_as_today() {
        fn registry(name: &str, version: &str) -> Result<ExtensionSpecifier, String> {
            Ok(ExtensionSpecifier::Registry {
                name: name.into(),
                version: version.into(),
            })
        }
        let cases: Vec<(&str, Result<ExtensionSpecifier, String>)> = vec![
            ("@acme/tool", Err("E054".into())), // I1: P1 refuses a name with no version
            ("@acme/tool@", Err("E054".into())), // I2
            ("@acme/tool@1.2.0", registry("@acme/tool", "1.2.0")), // I3
            ("@acme/tool@^1.2", registry("@acme/tool", "^1.2")), // I4
            ("@acme/tool@1.x", registry("@acme/tool", "1.x")), // I5
            ("@acme/tool@1.2", registry("@acme/tool", "1.2")), // I6
            ("@acme/tool@1.0.0/x", registry("@acme/tool", "1.0.0/x")), // I7
            ("@acme/tool@1.0.0?x=1", registry("@acme/tool", "1.0.0?x=1")), // I8
            ("foo@/bar", registry("foo", "/bar")), // I9
            ("tool@1.0.0", registry("tool", "1.0.0")), // I10
            ("tool", Err("E054".into())),       // I11
            ("@acme/..", Err("E054".into())),   // I12
            ("@acme/aa/bb", Err("E054".into())), // I13
            ("@acme/a/b", Err("E054".into())),  // I14
            ("@a/x", Err("E054".into())),       // I15
            ("@acme/T ool", Err("E054".into())), // I16
            ("Acme@1", registry("Acme", "1")),  // I17
            ("@acme/tool@latest", registry("@acme/tool", "latest")), // I18
            ("@acme/tool@*", registry("@acme/tool", "*")), // I18
            ("@acme/tool@>=1, <2", registry("@acme/tool", ">=1, <2")), // I19
            ("@acme/tool@^bogus", registry("@acme/tool", "^bogus")), // I20
            (
                "@acme/tool@2.0.0+build.1",
                registry("@acme/tool", "2.0.0+build.1"),
            ), // I21
            ("@scope", Err("E054".into())),     // I22
            (
                "git+https://h/r#v",
                Ok(ExtensionSpecifier::Git {
                    url: "https://h/r".into(),
                    rev: Some("v".into()),
                }),
            ), // I25
            (" @acme/tool@1.2.0 ", registry("@acme/tool", "1.2.0")), // I26
        ];
        for (input, want) in cases {
            let got = parse_extension_specifier(input).map_err(|d| d.code.to_string());
            assert_eq!(got, want, "{input:?}");
        }
    }

    // B:parse_extension_specifier — verify unit "rejects invalid specifier"
    #[test]
    fn test_rejects_invalid_specifier() {
        let err = parse_extension_specifier("").unwrap_err();
        assert_eq!(err.code, "E054");

        let err2 = parse_extension_specifier("just-a-name").unwrap_err();
        assert_eq!(err2.code, "E054");
    }
}
