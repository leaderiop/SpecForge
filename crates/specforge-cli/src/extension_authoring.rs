//! `specforge extension init|build|validate`: author an extension with the
//! SDK (ADR 0012). `init` scaffolds an SDK crate declaring the extension,
//! `build` builds its `wasm32-wasip2` component, and `validate` loads the
//! built component and reports its declaration as the registry build sees
//! it. No manifest file is written or read: the binary declares the
//! extension.

use crate::OutputFormat;
use crate::outcome::{Exit, Refusal};
use serde_json::json;
use specforge_common::{Code, Diagnostic, Severity, codes};
use std::path::Path;

/// The diagnostic for an extension project that isn't there or isn't
/// built.
const NOT_BUILT: Code = codes::E040;

pub fn run_init(path: &Path, name: Option<&str>, format: OutputFormat) -> Exit {
    let ext_name = name.unwrap_or("my-extension");
    let ext_dir = path.join(ext_name.rsplit('/').next().unwrap_or(ext_name));
    let declared = if ext_name.starts_with('@') {
        ext_name.to_string()
    } else {
        format!("@local/{ext_name}")
    };

    if ext_dir.exists() {
        return Refusal::of(format).coded(
            codes::E065,
            format!("directory '{}' already exists", ext_dir.display()),
        );
    }
    if let Err(message) = crate::new::scaffold(&ext_dir, &declared) {
        return Refusal::of(format).coded(codes::E066, message);
    }

    match format {
        OutputFormat::Json => {
            let output = json!({
                "status": "created",
                "name": declared,
                "short": crate::new::short_name(&declared),
                "path": ext_dir.display().to_string(),
                "files": crate::new::SCAFFOLDED,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => {
            println!("Created extension {} at {}", declared, ext_dir.display());
            for file in crate::new::SCAFFOLDED {
                println!("  {file}");
            }
            println!();
            println!("next steps:");
            println!("  specforge extension build --path {}", ext_dir.display());
            println!(
                "  specforge extension validate --path {}",
                ext_dir.display()
            );
        }
    }
    Exit::Passed
}

pub fn run_build(path: &Path, format: OutputFormat) -> Exit {
    if !path.join("Cargo.toml").exists() {
        return Refusal::of(format).coded(
            NOT_BUILT,
            format!("no Cargo.toml found at {}", path.display()),
        );
    }

    // The component the host loads: release, wasm32-wasip2, built into the
    // crate's own `target/` (whatever CARGO_TARGET_DIR says), where
    // `validate` and `publish` look for it.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = match std::process::Command::new(cargo)
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip2",
            "--target-dir",
        ])
        .arg(path.join("target"))
        .current_dir(path)
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            return Refusal::of(format).coded(NOT_BUILT, format!("cannot run cargo: {e}"));
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first_error = stderr.lines().find(|l| l.starts_with("error"));
        let message = format!(
            "cargo build --release --target wasm32-wasip2 failed in {}{}",
            path.display(),
            first_error.map(|l| format!(": {l}")).unwrap_or_default()
        );
        if format == OutputFormat::Human {
            eprint!("{stderr}");
        }
        return Refusal::of(format).coded(NOT_BUILT, message);
    }
    let binary = match specforge_ops::publish::binary_at(path) {
        Ok(binary) => binary,
        Err(error) => {
            return Refusal::of(format).report(&error);
        }
    };

    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": "built",
                "path": path.display().to_string(),
                "component": binary.display().to_string(),
            }))
            .expect("serialize JSON output")
        ),
        OutputFormat::Human => println!("Built {}", binary.display()),
    }
    Exit::Passed
}

pub fn run_validate(path: &Path, format: OutputFormat) -> Exit {
    let binary = match specforge_ops::publish::binary_at(path) {
        Ok(binary) => binary,
        Err(error) => {
            return Refusal::of(format).report(&error);
        }
    };
    let wasm = match std::fs::read(&binary) {
        Ok(wasm) => wasm,
        Err(e) => {
            return Refusal::of(format)
                .coded(NOT_BUILT, format!("cannot read {}: {e}", binary.display()));
        }
    };
    let (declaration, diagnostics) = match specforge_ops::publish::declare(&wasm) {
        Ok(declared) => declared,
        Err(error) => {
            return Refusal::of(format).report(&error);
        }
    };
    let valid = !diagnostics.iter().any(|d| d.severity == Severity::Error);

    match format {
        OutputFormat::Json => {
            let output = json!({
                "valid": valid,
                "name": declaration.name(),
                "version": declaration.version(),
                "short": declaration.short(),
                "component": binary.display().to_string(),
                "diagnostics": diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
                "declaration": declaration,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => {
            for d in &diagnostics {
                eprintln!("{}[{}]: {}", level(d), d.code, d.message);
            }
            if valid {
                println!(
                    "{} v{} declares a valid extension ({})",
                    declaration.name(),
                    declaration.version(),
                    binary.display()
                );
            } else {
                eprintln!(
                    "{} v{}: the declaration has errors",
                    declaration.name(),
                    declaration.version()
                );
            }
        }
    }
    Exit::of_verdict(valid)
}

fn level(d: &Diagnostic) -> &'static str {
    match d.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        _ => "info",
    }
}

fn diagnostic_json(d: &Diagnostic) -> serde_json::Value {
    json!({ "code": d.code, "severity": level(d), "message": d.message })
}
