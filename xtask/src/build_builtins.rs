//! Builds the six builtin extension components (wasm32-wasip2) and keeps the
//! vendored copies under `extensions/<name>/wasm/` — the bytes
//! `specforge-component` embeds — in step with their sources.
//!
//! ```text
//! build-builtins             build any blob missing from target/
//! build-builtins --force     rebuild every blob into target/
//! build-builtins --install   rebuild, vendor into extensions/<name>/wasm/, record inputs.json
//! build-builtins --check     fail if a vendored blob or any recorded input changed (no build)
//! ```

use serde_json::{Value, json};
use specforge_wasm::hex_sha256;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const EXTENSIONS: &[(&str, &str)] = &[
    ("product", "specforge_ext_product.wasm"),
    ("software", "specforge_ext_software.wasm"),
    ("governance", "specforge_ext_governance.wasm"),
    ("formal", "specforge_ext_formal.wasm"),
    ("testing", "specforge_ext_testing.wasm"),
    ("cargo-test", "specforge_ext_cargo_test.wasm"),
    ("vitest", "specforge_ext_vitest.wasm"),
    ("rust", "specforge_ext_rust.wasm"),
    ("typescript", "specforge_ext_typescript.wasm"),
];

const TARGET: &str = "wasm32-wasip2";
const INPUTS_FILE: &str = "inputs.json";

/// The shared crates inherit edition/version/license and dependency specs from
/// the root manifest; only those tables can change what they compile to.
const WORKSPACE_INHERITED: &str = "Cargo.toml#workspace-inherited";
const INHERITED_TABLES: &[&str] = &["[workspace.package]", "[workspace.dependencies]"];

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Missing,
    Force,
    Install,
    Check,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => Mode::Missing,
        ["--force"] => Mode::Force,
        ["--install"] => Mode::Install,
        ["--check"] => Mode::Check,
        other => {
            eprintln!("usage: build-builtins [--force | --install | --check] (got {other:?})");
            std::process::exit(2);
        }
    };
    let root = std::env::var("CARGO_MANIFEST_DIR")
        .map(|m| {
            Path::new(&m)
                .join("..")
                .canonicalize()
                .expect("repo root resolves")
        })
        .expect("CARGO_MANIFEST_DIR is set when run via cargo xtask");

    let mut failed = false;
    for &(name, artifact) in EXTENSIONS {
        let ok = match mode {
            Mode::Check => check(&root, name, artifact),
            Mode::Missing | Mode::Force => {
                build(&root, name, artifact, mode == Mode::Force).is_some()
            }
            Mode::Install => build(&root, name, artifact, true)
                .is_some_and(|built| install(&root, name, artifact, &built)),
        };
        failed |= !ok;
    }
    if failed && mode == Mode::Check {
        eprintln!(
            "\nVendored builtin blobs are stale. Rebuild them with\n  \
             cargo run -p xtask --bin build-builtins -- --install\n\
             and commit extensions/*/wasm/."
        );
    }
    std::process::exit(i32::from(failed));
}

fn build(root: &Path, name: &str, artifact: &str, force: bool) -> Option<PathBuf> {
    let blob = root.join(format!(
        "extensions/{name}/target/{TARGET}/release/{artifact}"
    ));
    if blob.exists() && !force {
        println!("[skip]   {name}: {} exists", blob.display());
        return Some(blob);
    }
    println!(
        "[build]  {name} for {TARGET}{}",
        if force { " (forced)" } else { "" }
    );
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", TARGET])
        .current_dir(root.join(format!("extensions/{name}")))
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn cargo while building builtin '{name}': {e}"));
    if status.success() && blob.exists() {
        println!("[ok]     {name}");
        Some(blob)
    } else {
        eprintln!(
            "[fail]   {name}: build did not produce {}\nIf the target is missing, run: rustup target add {TARGET}",
            blob.display()
        );
        None
    }
}

fn install(root: &Path, name: &str, artifact: &str, built: &Path) -> bool {
    let vendor_dir = root.join(format!("extensions/{name}/wasm"));
    let result = (|| -> Result<usize, String> {
        let bytes = std::fs::read(built).map_err(|e| format!("read {}: {e}", built.display()))?;
        let inputs = collect_inputs(root, name, &built.with_extension("d"))?;
        let count = inputs.len();
        let record = json!({
            "artifact": artifact,
            "artifact_sha256": hex_sha256(&bytes),
            "built_with": rustc_version(),
            "inputs": inputs,
        });
        let mut text = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
        text.push('\n');
        std::fs::create_dir_all(&vendor_dir).map_err(|e| e.to_string())?;
        std::fs::write(vendor_dir.join(artifact), &bytes).map_err(|e| e.to_string())?;
        std::fs::write(vendor_dir.join(INPUTS_FILE), text).map_err(|e| e.to_string())?;
        Ok(count)
    })();
    match result {
        Ok(count) => {
            println!("[vendor] {name}: {count} inputs recorded");
            true
        }
        Err(e) => {
            eprintln!("[fail]   {name}: {e}");
            false
        }
    }
}

fn check(root: &Path, name: &str, artifact: &str) -> bool {
    let vendor_dir = root.join(format!("extensions/{name}/wasm"));
    let record: Option<Value> = std::fs::read(vendor_dir.join(INPUTS_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(record) = record else {
        eprintln!("[stale]  {name}: extensions/{name}/wasm/{INPUTS_FILE} is missing or unreadable");
        return false;
    };
    let Some(inputs) = record["inputs"].as_object() else {
        eprintln!("[stale]  {name}: {INPUTS_FILE} has no \"inputs\" object");
        return false;
    };

    let mut problems = Vec::new();
    match std::fs::read(vendor_dir.join(artifact)) {
        Ok(bytes) if record["artifact_sha256"].as_str() == Some(hex_sha256(&bytes).as_str()) => {}
        Ok(_) => problems.push(format!(
            "{artifact} differs from the blob --install recorded"
        )),
        Err(e) => problems.push(format!("{artifact}: {e}")),
    }
    for (key, recorded) in inputs {
        match input_hash(root, key) {
            Ok(current) if recorded.as_str() == Some(current.as_str()) => {}
            Ok(_) => problems.push(format!("changed: {key}")),
            Err(_) => problems.push(format!("missing: {key}")),
        }
    }

    if problems.is_empty() {
        println!("[fresh]  {name}: blob and {} inputs match", inputs.len());
        true
    } else {
        eprintln!("[stale]  {name}:");
        for problem in problems {
            eprintln!("           {problem}");
        }
        false
    }
}

/// Every repo-local file rustc read for the artifact (from cargo's dep-info),
/// the manifest owning each, the extension's lockfile (pins registry crates),
/// and the root manifest tables the shared crates inherit from.
fn collect_inputs(
    root: &Path,
    name: &str,
    dep_info: &Path,
) -> Result<BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(dep_info)
        .map_err(|e| format!("read dep-info {}: {e}", dep_info.display()))?;
    let mut keys = BTreeSet::new();
    for dep in parse_dep_info(&text) {
        let dep = std::fs::canonicalize(&dep).unwrap_or(dep);
        let Ok(rel) = dep.strip_prefix(root) else {
            continue;
        };
        keys.insert(repo_key(rel));
        if let Some(manifest) = owning_manifest(root, &dep) {
            keys.insert(manifest);
        }
    }
    let own_lib = format!("extensions/{name}/src/lib.rs");
    if !keys.contains(&own_lib) {
        return Err(format!(
            "dep-info {} does not list {own_lib}; refusing to record an incomplete input set",
            dep_info.display()
        ));
    }
    keys.insert(format!("extensions/{name}/Cargo.lock"));
    keys.insert(WORKSPACE_INHERITED.to_string());

    keys.into_iter()
        .map(|key| {
            let hash = input_hash(root, &key).map_err(|e| format!("hash {key}: {e}"))?;
            Ok((key, hash))
        })
        .collect()
}

fn input_hash(root: &Path, key: &str) -> std::io::Result<String> {
    if key == WORKSPACE_INHERITED {
        let manifest = std::fs::read_to_string(root.join("Cargo.toml"))?;
        return Ok(hex_sha256(inherited_tables(&manifest).as_bytes()));
    }
    Ok(hex_sha256(&std::fs::read(root.join(key))?))
}

fn inherited_tables(manifest: &str) -> String {
    let mut out = String::new();
    let mut keep = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            keep = INHERITED_TABLES.contains(&trimmed);
        }
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn owning_manifest(root: &Path, file: &Path) -> Option<String> {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if !d.starts_with(root) {
            return None;
        }
        let manifest = d.join("Cargo.toml");
        if manifest.is_file() {
            return manifest.strip_prefix(root).ok().map(repo_key);
        }
        dir = d.parent();
    }
    None
}

fn repo_key(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Cargo's dep-info is one Makefile rule, `target: dep dep ...`, with spaces
/// inside paths escaped as `\ `.
fn parse_dep_info(text: &str) -> Vec<PathBuf> {
    let Some((_, deps)) = text.lines().next().and_then(|line| line.split_once(": ")) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    let mut current = String::new();
    let mut chars = deps.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&' ') => {
                chars.next();
                current.push(' ');
            }
            ' ' => {
                if !current.is_empty() {
                    paths.push(PathBuf::from(std::mem::take(&mut current)));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        paths.push(PathBuf::from(current));
    }
    paths
}

fn rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dep_info_splits_on_unescaped_spaces_only() {
        let text = "/r/out.wasm: /r/a.rs /r/dir\\ with\\ space/b.rs /r/c.json\n";
        assert_eq!(
            parse_dep_info(text),
            vec![
                PathBuf::from("/r/a.rs"),
                PathBuf::from("/r/dir with space/b.rs"),
                PathBuf::from("/r/c.json"),
            ]
        );
    }

    #[test]
    fn inherited_tables_ignore_members_and_profiles() {
        let a = "[workspace]\nmembers = [\"x\"]\n[workspace.package]\nedition = \"2024\"\n\
                 [workspace.dependencies]\nserde = \"1\"\n[profile.release]\nlto = true\n";
        let b = "[workspace]\nmembers = [\"x\", \"y\"]\n[workspace.package]\nedition = \"2024\"\n\
                 [workspace.dependencies]\nserde = \"1\"\n[profile.release]\nlto = false\n";
        assert_eq!(inherited_tables(a), inherited_tables(b));

        let c = a.replace(
            "serde = \"1\"",
            "serde = { version = \"1\", features = [\"rc\"] }",
        );
        assert_ne!(inherited_tables(a), inherited_tables(&c));
    }
}
