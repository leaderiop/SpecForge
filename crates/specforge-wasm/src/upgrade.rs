use crate::cache::invalidate_entry;
use crate::install::install_extension;
use crate::lock_file::LockFile;
use specforge_common::{Diagnostic, Severity};
use specforge_registry::ManifestV2;
use std::path::Path;

/// Result of an upgrade operation.
#[derive(Debug)]
pub struct UpgradeResult {
    pub name: String,
    pub old_version: String,
    pub new_version: String,
    pub aot_recompiled: bool,
}

/// Check if a newer version is available (compare version strings).
/// Parses as semver (major.minor.patch) tuples. Falls back to string comparison.
pub fn check_newer_version(current: &str, available: &str) -> bool {
    fn parse_semver(s: &str) -> Option<(u64, u64, u64)> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse::<u64>().ok()?;
        let minor = parts[1].parse::<u64>().ok()?;
        let patch = parts[2].parse::<u64>().ok()?;
        Some((major, minor, patch))
    }

    match (parse_semver(current), parse_semver(available)) {
        (Some(cur), Some(avail)) => avail > cur,
        _ => available > current,
    }
}

/// Upgrade an extension: validate peer compat -> install new -> invalidate old AOT -> recompile.
#[allow(clippy::too_many_arguments)]
pub fn upgrade_extension(
    name: &str,
    new_version: &str,
    new_wasm_bytes: &[u8],
    expected_sha256: &str,
    peer_manifests: &[ManifestV2],
    new_manifest: &ManifestV2,
    extensions_dir: &Path,
    cache_dir: &Path,
    lock: &mut LockFile,
    force: bool,
) -> Result<UpgradeResult, Diagnostic> {
    // 1. Find old version from lock file
    let old_version = lock
        .entries
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.version.clone())
        .unwrap_or_default();

    let old_hash = lock
        .entries
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.wasm_hash.clone())
        .unwrap_or_default();

    // 2. Validate peer dependency compatibility
    // Build a manifest set with the new manifest swapped in
    let mut manifests_for_check: Vec<ManifestV2> = peer_manifests
        .iter()
        .filter(|m| m.name != name)
        .cloned()
        .collect();
    manifests_for_check.push(new_manifest.clone());

    let peer_diags = specforge_registry::validate_peer_dependencies(&manifests_for_check);
    let relevant_diags: Vec<&Diagnostic> = peer_diags
        .iter()
        .filter(|d| d.message.contains(name))
        .collect();

    if !relevant_diags.is_empty() && !force {
        return Err(Diagnostic {
            code: "E027".to_string(),
            severity: Severity::Error,
            message: format!(
                "upgrade of '{}' to {} breaks peer dependencies: {}",
                name,
                new_version,
                relevant_diags
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            span: None,
            suggestion: Some("use --force to override peer dependency checks".to_string()),
        });
    }

    // 3. Invalidate old AOT cache
    if !old_hash.is_empty() {
        invalidate_entry(cache_dir, &old_hash);
    }

    // 4. Install new version
    let install_result = install_extension(
        name,
        new_version,
        new_wasm_bytes,
        expected_sha256,
        extensions_dir,
        cache_dir,
        lock,
        false, // always AOT compile on upgrade
    )?;

    Ok(UpgradeResult {
        name: name.to_string(),
        old_version,
        new_version: new_version.to_string(),
        aot_recompiled: install_result.aot_compiled,
    })
}
