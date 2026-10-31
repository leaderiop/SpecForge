//! `resolve_package_manager <script>` (#5346): the command that runs a package script with the project's package
//! manager, for the build and test gates that used to say `npm` whatever the project used. Read-only: it reads
//! `package.json` and the lockfiles through declared file reads (ADR 0293) and prints the command, nothing else.
//!
//! A port of `resolvePackageManagerCommand` (`src/normalize-test-command.cts`, gsd-core v1.19.0, MIT, `LICENSE-GSD`):
//! `package.json`'s `packageManager` field (`name@version`, a known name) wins, then the first lockfile in
//! [`LOCKFILES`]' order, else npm. Only the project root is looked in, as upstream looks only in the gate's directory.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, required, run};

const MANAGERS: &[&str] = &["npm", "pnpm", "yarn", "bun"];

/// `(read name, lockfile, manager)`, in upstream's order: it only matters when a project carries several.
const LOCKFILES: &[(&str, &str, &str)] = &[
    ("pnpm_lock", "pnpm-lock.yaml", "pnpm"),
    ("yarn_lock", "yarn.lock", "yarn"),
    ("bun_lock", "bun.lock", "bun"),
    ("bun_lockb", "bun.lockb", "bun"),
];

/// `^[A-Za-z0-9_][A-Za-z0-9:_-]{0,63}$`: the caller splices the name into a shell command.
fn plain_script(script: &str) -> bool {
    let mut bytes = script.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        && script.len() <= 64
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'_' | b'-'))
}

/// The manager `package.json`'s text declares, when it is an object whose `packageManager` names a known one.
fn declared(package_json: &str) -> Option<&'static str> {
    let value: serde_json::Value = serde_json::from_str(package_json).ok()?;
    let name = value.get("packageManager")?.as_str()?.split('@').next()?;
    MANAGERS.iter().copied().find(|m| *m == name)
}

/// The command that runs `script`, from `package.json`'s text and which lockfiles exist. `test` is the manager's short
/// form, except bun's (`bun test` is bun's own runner, not the script).
pub fn resolve(
    script: &str,
    package_json: Option<&str>,
    has: impl Fn(&str) -> bool,
) -> Result<String, String> {
    if !plain_script(script) {
        return Err(format!(
            "'{script}' is not a script name: a letter, digit or `_`, then up to 63 of those, `:` or `-`"
        ));
    }
    let manager = package_json
        .and_then(declared)
        .or_else(|| LOCKFILES.iter().find(|(_, file, _)| has(file)).map(|l| l.2))
        .unwrap_or("npm");
    Ok(if script == "test" && manager != "bun" {
        format!("{manager} test")
    } else {
        format!("{manager} run {script}")
    })
}

fn resolve_package_manager(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let script = required(call, "script")?;
    // shortcut: the host sends text up to 64 KiB (upstream parses up to 1 MiB); a larger package.json reads as one
    // without the field. Upgrade if a real manifest is ever that large.
    let package_json = call
        .files("package_json")
        .and_then(|f| f.facts.first())
        .and_then(|f| f.text.as_deref());
    let has = |file: &str| {
        LOCKFILES.iter().any(|(name, lockfile, _)| {
            *lockfile == file
                && call
                    .files(name)
                    .is_some_and(|f| f.facts.iter().any(|fact| fact.exists))
        })
    };
    let command = resolve(script, package_json, has)?;
    Ok(answer(
        call,
        json!({ "command": command }),
        command.clone(),
        None,
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("resolve_package_manager", |cmd| {
        cmd.description("Print the command that runs a package script (build, test ...) with the project's package manager: package.json's packageManager, else its lockfile, else npm")
            .arg("script", |a| {
                a.required()
                    .description("the package script: build, test, lint ...");
            })
            .reads("package_json", FileReadSpec::glob("package.json").text());
        for (name, file, _) in LOCKFILES {
            cmd.reads(name, FileReadSpec::glob(file));
        }
        cmd.handler(|call| run(call, resolve_package_manager));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with<'a>(files: &'a [&'a str]) -> impl Fn(&str) -> bool + 'a {
        move |file| files.contains(&file)
    }

    #[test]
    fn each_lockfile_names_its_manager() {
        for (file, build, test) in [
            ("pnpm-lock.yaml", "pnpm run build", "pnpm test"),
            ("yarn.lock", "yarn run build", "yarn test"),
            ("bun.lock", "bun run build", "bun run test"),
            ("bun.lockb", "bun run build", "bun run test"),
        ] {
            assert_eq!(resolve("build", None, with(&[file])).as_deref(), Ok(build));
            assert_eq!(resolve("test", None, with(&[file])).as_deref(), Ok(test));
        }
        // pnpm's lockfile outranks yarn's.
        assert_eq!(
            resolve("build", None, with(&["yarn.lock", "pnpm-lock.yaml"])).as_deref(),
            Ok("pnpm run build")
        );
    }

    #[test]
    fn the_package_manager_field_beats_a_lockfile() {
        let yarn = r#"{"packageManager": "yarn@4.1.0"}"#;
        assert_eq!(
            resolve("test", Some(yarn), with(&["pnpm-lock.yaml"])).as_deref(),
            Ok("yarn test")
        );
        // An unknown name, a non-string, a non-object or bad JSON falls through to the lockfiles, as upstream.
        for text in [
            r#"{"packageManager": "deno@2"}"#,
            r#"{"packageManager": 3}"#,
            "[]",
            "{not json",
        ] {
            assert_eq!(
                resolve("build", Some(text), with(&["bun.lock"])).as_deref(),
                Ok("bun run build"),
                "{text}"
            );
        }
    }

    #[test]
    fn no_file_is_npm() {
        assert_eq!(
            resolve("build", None, with(&[])).as_deref(),
            Ok("npm run build")
        );
        assert_eq!(
            resolve("test", Some("{}"), with(&[])).as_deref(),
            Ok("npm test")
        );
        assert_eq!(
            resolve("test:unit", None, with(&[])).as_deref(),
            Ok("npm run test:unit")
        );
    }

    #[test]
    fn a_script_name_that_is_not_plain_is_refused() {
        for bad in ["", "-x", ":x", "a b", "a;rm", "a$(x)", &"a".repeat(65)] {
            assert!(resolve(bad, None, with(&[])).is_err(), "{bad:?}");
        }
        assert!(resolve(&"a".repeat(64), None, with(&[])).is_ok());
        assert!(resolve("_x", None, with(&[])).is_ok());
    }
}
