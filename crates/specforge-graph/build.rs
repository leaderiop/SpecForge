use serde::Serialize;
use specforge_parser::{FieldValue, parse};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct GraphExport {
    entities: Vec<ExportedEntity>,
}

#[derive(Serialize)]
struct ExportedEntity {
    id: String,
    kind: String,
    verify: Vec<ExportedVerify>,
    testable: bool,
    /// BDD intent (C11-03): gherkin feature-file references. A pure
    /// Cucumber team's behaviors declare intent through this field with no
    /// `verify` statements — without exporting it they disappear from the
    /// coverage diff entirely.
    #[serde(skip_serializing_if = "Option::is_none")]
    gherkin: Option<Vec<String>>,
}

#[derive(Serialize)]
struct ExportedVerify {
    kind: String,
    description: String,
    slug: String,
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .replace(' ', "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "")
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let spec_dir = project_root.join("spec");

    // Rerun if any spec file changes
    println!("cargo::rerun-if-changed={}", spec_dir.display());

    if !spec_dir.exists() {
        return;
    }

    let spec_files = collect_spec_files(&spec_dir);
    let mut entities = Vec::new();

    for path in &spec_files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let result = parse(&source, path.to_str().unwrap_or("unknown.spec"));

        for entity in &result.entities {
            let verify = match entity.fields.get("verify") {
                Some(FieldValue::VerifyList(stmts)) => stmts
                    .iter()
                    .map(|v| ExportedVerify {
                        kind: v.kind.clone(),
                        description: v.description.clone(),
                        slug: slugify(&v.description),
                    })
                    .collect(),
                _ => vec![],
            };

            let gherkin = match entity.fields.get("gherkin") {
                Some(FieldValue::StringList(files)) if !files.is_empty() => {
                    Some(files.iter().map(|f| f.to_string()).collect::<Vec<_>>())
                }
                _ => None,
            };

            // C11-03: gherkin-referencing entities carry spec intent even
            // with zero verify statements — W004 accepts verify OR gherkin,
            // so the export must too.
            let testable = !verify.is_empty() || gherkin.is_some();
            entities.push(ExportedEntity {
                id: entity.id.raw.to_string(),
                kind: entity.kind.raw.to_string(),
                verify,
                testable,
                gherkin,
            });
        }
    }

    // C5-05: no wall-clock timestamp — it defeated incremental reuse of
    // graph.json by dirtying the file on every rebuild. Content is derived
    // solely from spec/, so the output is reproducible.
    let export = GraphExport { entities };

    // Write to target/specforge/graph.json
    let target_dir = find_target_dir();
    let specforge_dir = target_dir.join("specforge");
    if std::fs::create_dir_all(&specforge_dir).is_err() {
        println!("cargo::warning=specforge: could not create target/specforge/");
        return;
    }

    let json = match serde_json::to_string_pretty(&export) {
        Ok(j) => j,
        Err(e) => {
            println!("cargo::warning=specforge: failed to serialize graph: {e}");
            return;
        }
    };

    let out_path = specforge_dir.join("graph.json");
    if let Err(e) = std::fs::write(&out_path, json) {
        println!("cargo::warning=specforge: failed to write graph.json: {e}");
    }
}

fn collect_spec_files(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(collect_spec_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "spec") {
                results.push(path);
            }
        }
    }
    results
}

fn find_target_dir() -> PathBuf {
    // OUT_DIR is like target/debug/build/specforge-graph-XXX/out
    // Walk up to find target/
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let mut dir = Path::new(&out_dir);
        while let Some(parent) = dir.parent() {
            if dir.file_name().is_some_and(|n| n == "target") {
                return dir.to_path_buf();
            }
            dir = parent;
        }
    }
    PathBuf::from("target")
}
