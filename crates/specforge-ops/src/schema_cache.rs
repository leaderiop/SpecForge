//! The schema cache: the Graph Protocol schema the previous export wrote
//! to `.specforge/schema-cache.json`, which versions the next export's
//! schema and shows what changed since (W053, I016). The schema and its
//! diff are the emitter's; reading and writing the cache is here.

use std::io;
use std::path::Path;

use specforge_common::{Diagnostic, Severity};
use specforge_emitter::{
    GraphProtocolSchema, SchemaCacheEntry, SchemaMigration, compute_schema_version, content_hash,
    diff_schemas_optional,
};

const CACHE_FILE: &str = "schema-cache.json";

pub fn persist_schema_cache(schema: &GraphProtocolSchema, cache_dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(cache_dir)?;

    let entry = SchemaCacheEntry {
        content_hash: content_hash(schema),
        schema: schema.clone(),
    };

    let json = serde_json::to_string_pretty(&entry).map_err(io::Error::other)?;

    // Atomic write: temp file then rename
    let target = cache_dir.join(CACHE_FILE);
    let tmp = cache_dir.join(".schema-cache.tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, &target)?;

    Ok(())
}

pub fn load_schema_cache(cache_dir: &Path) -> io::Result<Option<SchemaCacheEntry>> {
    let path = cache_dir.join(CACHE_FILE);
    if !path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&path)?;
    let entry: SchemaCacheEntry = serde_json::from_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(Some(entry))
}

/// Compare `current` against the schema cached in `cache_dir` by the
/// previous export. Returns the migration and its diagnostics: a W053
/// warning per breaking change, or I016 when there is no cache although
/// `output_dir_has_exports` says the project was exported before. With no
/// previous schema every change is an addition and nothing is breaking.
pub fn detect_breaking_with_diagnostics(
    cache_dir: &Path,
    current: &GraphProtocolSchema,
    output_dir_has_exports: bool,
) -> (SchemaMigration, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    let cached = match load_schema_cache(cache_dir) {
        Ok(Some(entry)) => Some(entry.schema),
        Ok(None) => {
            if output_dir_has_exports {
                diagnostics.push(Diagnostic {
                    code: "I016".to_string(),
                    severity: Severity::Info,
                    message: "Schema cache not found; breaking change detection skipped. \
                              Prior exports exist but .specforge/schema-cache.json is missing."
                        .to_string(),
                    span: None,
                    suggestion: Some(
                        "Run a full compilation to regenerate the schema cache.".to_string(),
                    ),
                });
            }
            None
        }
        Err(_) => None,
    };

    let migration = diff_schemas_optional(cached.as_ref(), current);
    for change in migration.changes.iter().filter(|c| c.is_breaking()) {
        diagnostics.push(Diagnostic {
            code: "W053".to_string(),
            severity: Severity::Warning,
            message: format!("breaking schema change since the last export: {change}"),
            span: None,
            suggestion: Some(
                "Agents and tools that read the previous export may not accept this one: \
                 update them, or keep the extension versions that produced the old schema."
                    .to_string(),
            ),
        });
    }
    (migration, diagnostics)
}

/// Give `schema` its version: the version of the schema the previous export
/// cached in `cache_dir`, bumped by what changed since (major for a breaking
/// change, minor for an addition, patch for anything else; unchanged when
/// nothing changed). With no cache, or one that can't be read, it is 1.0.0.
/// Only reads the cache.
pub fn attach_schema_version(schema: &mut GraphProtocolSchema, cache_dir: &Path) {
    let previous = load_schema_cache(cache_dir)
        .ok()
        .flatten()
        .map(|e| e.schema);
    let migration = diff_schemas_optional(previous.as_ref(), schema);
    schema.schema_version =
        compute_schema_version(&migration, previous.as_ref().map(|p| &p.schema_version));
}
