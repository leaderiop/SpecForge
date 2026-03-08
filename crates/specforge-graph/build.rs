use serde::Serialize;
use specforge_parser::{parse, FieldValue};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct GraphExport {
    entities: Vec<ExportedEntity>,
    timestamp: String,
}

#[derive(Serialize)]
struct ExportedEntity {
    id: String,
    kind: String,
    verify: Vec<ExportedVerify>,
    testable: bool,
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

    // Testable entity kinds (behavior, invariant, event)
    let testable_kinds = ["behavior", "invariant", "event"];

    for path in &spec_files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let result = parse(&source, path.to_str().unwrap_or("unknown.spec"));

        for entity in &result.entities {
            let testable = testable_kinds.contains(&entity.kind.raw.as_str());
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

            entities.push(ExportedEntity {
                id: entity.id.raw.clone(),
                kind: entity.kind.raw.clone(),
                verify,
                testable,
            });
        }
    }

    let export = GraphExport {
        entities,
        timestamp: chrono_lite_now(),
    };

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

fn chrono_lite_now() -> String {
    // ISO 8601 UTC timestamp without pulling in chrono
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let days = secs / 86400;
    let time = secs % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;

    // Days since epoch → year/month/day (simplified Gregorian)
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let year_days = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if remaining < year_days {
            break;
        }
        remaining -= year_days;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 0;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining < md as i64 {
            mo = i + 1;
            break;
        }
        remaining -= md as i64;
    }
    let d = remaining + 1;

    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}
