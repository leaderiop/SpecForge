//! What `specforge publish` uploads: an extension binary and the
//! declaration read from it (ADR 0012). Nothing here reaches a registry:
//! the binary is loaded and checked before any network call, and a binary
//! whose declaration has errors is refused.

use std::path::{Path, PathBuf};

use specforge_common::{Code, Diagnostic, Severity, codes};
use specforge_protocol_types::ExtensionDeclaration;

use crate::{OpError, OpErrorKind};

/// The diagnostic for an extension that can't be found or read.
const UNREADABLE: Code = codes::E040;

/// A binary ready to publish, with the declaration it is published as.
#[derive(Debug, Clone)]
pub struct Prepared {
    /// What the binary declares, loaded as every environment loads it: the
    /// package's stored manifest.
    pub declaration: ExtensionDeclaration,
    pub wasm: Vec<u8>,
    /// Warnings about the declaration (W153, W138, W021, ...), to show.
    pub diagnostics: Vec<Diagnostic>,
}

/// The binary `path` names: a `.wasm` component as given, or, for a
/// directory, the component its crate builds
/// (`target/wasm32-wasip2/release/<crate>.wasm`, the only `.wasm` there
/// when the crate's name can't be read).
pub fn binary_at(path: &Path) -> Result<PathBuf, OpError> {
    if !path.is_dir() {
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
        return Err(OpError::coded(
            OpErrorKind::PreconditionFailed,
            UNREADABLE,
            format!("no extension binary at {}", path.display()),
        )
        .with_suggestion("name a .wasm component, or the extension's crate directory"));
    }
    let release = path.join("target/wasm32-wasip2/release");
    let not_built = || {
        OpError::coded(
            OpErrorKind::PreconditionFailed,
            UNREADABLE,
            format!(
                "no built component in {}: build the extension first",
                release.display()
            ),
        )
        .with_suggestion("cargo build --release --target wasm32-wasip2")
    };
    if let Some(name) = crate_name(&path.join("Cargo.toml")) {
        let built = release.join(format!("{}.wasm", name.replace('-', "_")));
        if built.is_file() {
            return Ok(built);
        }
    }
    let mut built: Vec<PathBuf> = std::fs::read_dir(&release)
        .map_err(|_| not_built())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "wasm"))
        .collect();
    built.sort();
    match built.len() {
        0 => Err(not_built()),
        1 => Ok(built.remove(0)),
        _ => Err(OpError::coded(
            OpErrorKind::PreconditionFailed,
            UNREADABLE,
            format!(
                "{} holds several components ({}): name the one to publish",
                release.display(),
                built
                    .iter()
                    .filter_map(|p| p.file_name()?.to_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

/// The `[package]` name of the crate whose manifest is `cargo_toml`.
fn crate_name(cargo_toml: &Path) -> Option<String> {
    let text = std::fs::read_to_string(cargo_toml).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package
            && let Some(value) = line.strip_prefix("name")
            && let Some(value) = value.trim_start().strip_prefix('=')
        {
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// Load `wasm` and read its declaration, with what the registry build
/// alone reports of it (its load warnings, W153 and W138, first). A binary
/// that isn't a loadable extension is E028. Missing peers (E027) are left
/// out: they are installed beside the extension, not with it.
pub fn declare(wasm: &[u8]) -> Result<(ExtensionDeclaration, Vec<Diagnostic>), OpError> {
    let runtime = specforge_component::ComponentRuntime::new();
    let module = specforge_installed::Module::new(wasm.to_vec());
    let loaded = specforge_installed::declaration_of(&module, &runtime)?;
    let diagnostics = diagnostics_of(&loaded.declaration, loaded.warnings);
    Ok((loaded.declaration, diagnostics))
}

/// Load `wasm`, read its declaration and check it as the registry build
/// alone would ([`declare`], [`check`]), both before any network call.
pub fn prepare(wasm: Vec<u8>) -> Result<Prepared, OpError> {
    let (declaration, diagnostics) = declare(&wasm)?;
    refuse_errors(&declaration, &diagnostics)?;
    Ok(Prepared {
        declaration,
        wasm,
        diagnostics,
    })
}

/// `warnings`, then what the registry build of `declaration` alone reports
/// (its declaration, registry and surface diagnostics), without missing
/// peers (E027).
fn diagnostics_of(
    declaration: &ExtensionDeclaration,
    warnings: Vec<Diagnostic>,
) -> Vec<Diagnostic> {
    let build = specforge_registry::build_registries(vec![declaration.clone()]);
    warnings
        .into_iter()
        .chain(build.declaration_diagnostics)
        .chain(build.registry_diagnostics)
        .chain(build.surface_diagnostics)
        .filter(|d| !d.is(codes::E027))
        .collect()
}

/// Check `declaration` as the registry build alone checks it: an error
/// (E030, a refused tool schema, ...) refuses it, naming every error;
/// otherwise its warnings come back, after `warnings` (its load warnings,
/// W153 and W138).
pub fn check(
    declaration: &ExtensionDeclaration,
    warnings: Vec<Diagnostic>,
) -> Result<Vec<Diagnostic>, OpError> {
    let diagnostics = diagnostics_of(declaration, warnings);
    refuse_errors(declaration, &diagnostics)?;
    Ok(diagnostics)
}

fn refuse_errors(
    declaration: &ExtensionDeclaration,
    diagnostics: &[Diagnostic],
) -> Result<(), OpError> {
    let errors: Vec<&Diagnostic> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    let Some(first) = errors.first() else {
        return Ok(());
    };
    let detail: Vec<String> = errors
        .iter()
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect();
    Err(OpError::new(
        OpErrorKind::of_diagnostic(&first.code),
        first.code.clone(),
        format!(
            "{}@{} can't be published: its declaration has errors ({})",
            declaration.name(),
            declaration.version(),
            detail.join("; ")
        ),
    )
    .with_suggestion("fix the declaration in the extension's source and rebuild it"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_protocol_types::PeerDependency;
    use specforge_test_macros::test as specforge_test;

    fn greet() -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/greet-extension/greet.wasm"),
        )
        .expect("the greet fixture is vendored")
    }

    #[specforge_test(
        behavior = "publish_to_registry",
        verify = "the declaration is validated before publish"
    )]
    fn a_built_extension_is_prepared_with_its_declaration() {
        let prepared = prepare(greet()).unwrap();
        assert_eq!(prepared.declaration.name(), "@sdk/greet");
        assert_eq!(prepared.declaration.short(), "greet");
        assert!(
            prepared.diagnostics.is_empty(),
            "{:?}",
            prepared.diagnostics
        );
    }

    #[specforge_test(
        behavior = "publish_to_registry",
        verify = "publish refuses a binary whose declaration has errors before any network call"
    )]
    fn a_declaration_with_errors_is_refused_before_any_upload() {
        // `prepare` takes no registry: it decides before anything is sent.
        let mut declaration = prepare(greet()).unwrap().declaration;
        declaration.handshake.ext_short = Some("Friendly greetings".to_string());
        let error = check(&declaration, Vec::new()).unwrap_err();
        assert_eq!(error.code, "E030", "{error:?}");
        assert!(
            error
                .message
                .contains("@sdk/greet@0.1.0 can't be published"),
            "{error:?}"
        );
        assert!(error.message.contains("ext_short"), "{error:?}");

        // A required peer that isn't installed here is not an error.
        let mut with_peer = prepare(greet()).unwrap().declaration;
        with_peer.handshake.peer_dependencies.push(PeerDependency {
            name: "@acme/base".to_string(),
            version: "^1".to_string(),
            optional: false,
        });
        assert!(check(&with_peer, Vec::new()).is_ok());

        // A binary that isn't an extension never gets that far.
        let error = prepare(b"\0asm\x01\0\0\0".to_vec()).unwrap_err();
        assert_eq!(error.code, "E028", "{error:?}");
    }

    /// Pinned until T2 (ADR 0041): publish never sees a malformed peer range.
    #[test]
    fn pin_publish_accepts_a_malformed_peer_range() {
        let mut declaration = prepare(greet()).unwrap().declaration;
        declaration
            .handshake
            .peer_dependencies
            .push(PeerDependency {
                name: "@acme/base".to_string(),
                version: "one-ish".to_string(),
                optional: false,
            });
        assert!(check(&declaration, Vec::new()).is_ok());
    }

    #[test]
    fn a_crate_directory_names_its_built_component() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-ext\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let error = binary_at(dir.path()).unwrap_err();
        assert!(
            error.message.contains("build the extension first"),
            "{error:?}"
        );
        let release = dir.path().join("target/wasm32-wasip2/release");
        std::fs::create_dir_all(&release).unwrap();
        std::fs::write(release.join("other.wasm"), b"x").unwrap();
        std::fs::write(release.join("my_ext.wasm"), b"x").unwrap();
        assert_eq!(binary_at(dir.path()).unwrap(), release.join("my_ext.wasm"));
        let file = release.join("other.wasm");
        assert_eq!(binary_at(&file).unwrap(), file);
        assert!(binary_at(&dir.path().join("missing.wasm")).is_err());
    }
}
