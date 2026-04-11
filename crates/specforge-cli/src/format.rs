use specforge_common::find_project_root;
use specforge_formatter::{
    FormatConfig, discover_targets, format_source, load_config, unified_diff,
};
use std::io::{self, Read as IoRead, Write as IoWrite};
use std::path::Path;

/// Run the `specforge format` command.
///
/// Returns the process exit code:
/// - 0: all files already formatted (or successfully formatted)
/// - 1: in `--check` mode, some files would change
pub fn run(
    path: &Path,
    check: bool,
    diff: bool,
    stdin: bool,
    explicit_paths: &[String],
) -> i32 {
    // Find project root
    let project_root = find_project_root(path).unwrap_or_else(|| path.to_path_buf());

    // Load config
    let (config, config_diags) = load_config(path, &project_root);
    for d in &config_diags {
        eprintln!("warning: {}", d.message);
    }

    if stdin {
        return run_stdin(&config);
    }

    // Discover targets
    let explicit: Vec<std::path::PathBuf> = explicit_paths.iter().map(Into::into).collect();
    let spec_root = project_root.join("spec");
    let search_root = if spec_root.exists() { &spec_root } else { &project_root };
    let targets = discover_targets(search_root, &explicit, &[]);

    if targets.is_empty() {
        eprintln!("No .spec files found");
        return 0;
    }

    let mut files_checked = 0;
    let mut files_changed = 0;

    for target in &targets {
        let source = match std::fs::read_to_string(target) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to read {}: {}", target.display(), e);
                continue;
            }
        };

        let result = format_source(&source, &config);

        for d in &result.diagnostics {
            eprintln!("{}: {}", target.display(), d.message);
        }

        files_checked += 1;

        if result.formatted == source {
            continue;
        }

        files_changed += 1;

        if diff {
            let d = unified_diff(&target.display().to_string(), &source, &result.formatted);
            print!("{}", d.diff_text);
        } else if check {
            println!("{}", target.display());
        } else {
            // Write formatted output
            if let Err(e) = std::fs::write(target, &result.formatted) {
                eprintln!("error: failed to write {}: {}", target.display(), e);
                continue;
            }
            println!("{}", target.display());
        }
    }

    if !check && !diff {
        eprintln!(
            "Formatted {} file(s), {} changed",
            files_checked, files_changed
        );
    }

    if check && files_changed > 0 {
        1
    } else {
        0
    }
}

/// Format from stdin, write to stdout.
fn run_stdin(config: &FormatConfig) -> i32 {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("error: failed to read stdin: {e}");
        return 1;
    }

    let result = format_source(&input, config);

    for d in &result.diagnostics {
        eprintln!("{}", d.message);
    }

    print!("{}", result.formatted);
    io::stdout().flush().ok();

    0
}
