//! The schema cache: the Graph Protocol schema the previous export wrote
//! to `<root>/.specforge/schema-cache.json`, which versions the next
//! export's schema and shows what changed since (W053, I016). The schema
//! and its diff are the emitter's; reading and writing the cache is here.
//! A project view's cache is at the root the project was compiled from,
//! never an ancestor's (`ProjectView::schema_cache`).

use std::io;
use std::path::{Path, PathBuf};

use specforge_common::{Diagnostic, codes};
use specforge_emitter::{
    GraphProtocolSchema, SchemaCacheEntry, SchemaMigration, compute_schema_version, content_hash,
    diff_schemas_optional,
};

const CACHE_FILE: &str = "schema-cache.json";

/// The directory under a project root that holds the cache.
const CACHE_DIR: &str = ".specforge";

/// The schema cache in one directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaCache {
    dir: PathBuf,
}

impl SchemaCache {
    /// The cache of the project at `root`: `<root>/.specforge/`.
    pub fn of_root(root: &Path) -> Self {
        Self::in_dir(root.join(CACHE_DIR))
    }

    /// The cache in `dir`.
    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        SchemaCache { dir: dir.into() }
    }

    /// The directory holding `schema-cache.json`.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// What the previous export cached, if anything.
    pub fn load(&self) -> io::Result<Option<SchemaCacheEntry>> {
        load(&self.dir)
    }

    /// Give `schema` its version: the version of the schema the previous
    /// export cached, bumped by what changed since (major for a breaking
    /// change, minor for an addition, patch for anything else; unchanged
    /// when nothing changed). With no cache, or one that can't be read, it
    /// is 1.0.0. Only reads the cache.
    pub fn version(&self, schema: &mut GraphProtocolSchema) {
        let previous = self.load().ok().flatten().map(|e| e.schema);
        let migration = diff_schemas_optional(previous.as_ref(), schema);
        schema.schema_version =
            compute_schema_version(&migration, previous.as_ref().map(|p| &p.schema_version));
    }

    /// A W053 warning per breaking change since the cached schema, as
    /// `specforge export` reports them before it writes: its output goes to
    /// stdout, so nothing shows the project was exported before and the
    /// I016 branch of [`Self::detect_breaking`] is not taken.
    pub fn breaking_changes(&self, schema: &GraphProtocolSchema) -> Vec<Diagnostic> {
        self.detect_breaking(schema, false).1
    }

    /// Compare `current` against the cached schema. Returns the migration
    /// and its diagnostics: a W053 warning per breaking change, or I016 when
    /// there is no cache although `output_dir_has_exports` says the project
    /// was exported before. With no previous schema every change is an
    /// addition and nothing is breaking.
    pub fn detect_breaking(
        &self,
        current: &GraphProtocolSchema,
        output_dir_has_exports: bool,
    ) -> (SchemaMigration, Vec<Diagnostic>) {
        detect_breaking(&self.dir, current, output_dir_has_exports)
    }

    /// Replace the cache with `schema` (atomically: a temporary file,
    /// then a rename). Only `specforge export` records.
    pub fn record(&self, schema: &GraphProtocolSchema) -> io::Result<()> {
        persist(schema, &self.dir)
    }
}

fn persist(schema: &GraphProtocolSchema, cache_dir: &Path) -> io::Result<()> {
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

fn load(cache_dir: &Path) -> io::Result<Option<SchemaCacheEntry>> {
    let path = cache_dir.join(CACHE_FILE);
    if !path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&path)?;
    let entry: SchemaCacheEntry = serde_json::from_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(Some(entry))
}

fn detect_breaking(
    cache_dir: &Path,
    current: &GraphProtocolSchema,
    output_dir_has_exports: bool,
) -> (SchemaMigration, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    let cached = match load(cache_dir) {
        Ok(Some(entry)) => Some(entry.schema),
        Ok(None) => {
            if output_dir_has_exports {
                diagnostics.push(
                    Diagnostic::new(
                        codes::I016,
                        "Schema cache not found; breaking change detection skipped. \
                              Prior exports exist but .specforge/schema-cache.json is missing."
                            .to_string(),
                    )
                    .with_suggestion(
                        "Run a full compilation to regenerate the schema cache.".to_string(),
                    ),
                );
            }
            None
        }
        Err(_) => None,
    };

    let migration = diff_schemas_optional(cached.as_ref(), current);
    for change in migration.changes.iter().filter(|c| c.is_breaking()) {
        diagnostics.push(
            Diagnostic::new(
                codes::W053,
                format!("breaking schema change since the last export: {change}"),
            )
            .with_suggestion(
                "Agents and tools that read the previous export may not accept this one: \
                 update them, or keep the extension versions that produced the old schema."
                    .to_string(),
            ),
        );
    }
    (migration, diagnostics)
}

#[cfg(test)]
mod tests;
