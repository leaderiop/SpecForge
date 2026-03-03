use crate::pipeline;
use clap::{Args, Subcommand};
use specforge_common::EntityKind;
use specforge_emitter::RenderOptions;
use std::path::PathBuf;

#[derive(Args)]
pub struct RenderArgs {
    #[command(subcommand)]
    pub format: RenderFormat,
}

#[derive(Subcommand)]
pub enum RenderFormat {
    /// Render JSON graph export
    Json(JsonArgs),
    /// Render Markdown documentation
    Markdown(MarkdownArgs),
}

#[derive(Args)]
pub struct JsonArgs {
    /// Output directory
    pub dir: PathBuf,

    /// Path to check (directory or file). Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Args)]
pub struct MarkdownArgs {
    /// Output directory
    pub dir: PathBuf,

    /// Path to check (directory or file). Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Only render entities of this kind (e.g. "behavior")
    #[arg(long)]
    pub only: Option<String>,
}

/// Parse the --only flag into RenderOptions, validating the kind keyword.
fn parse_render_options(only: Option<&str>) -> Result<RenderOptions, i32> {
    match only {
        None => Ok(RenderOptions::default()),
        Some(keyword) => match EntityKind::from_keyword(keyword) {
            Some(kind) => Ok(RenderOptions { only: Some(kind) }),
            None => {
                eprintln!("specforge: unknown entity kind: {keyword}");
                let valid: Vec<_> = EntityKind::ALL.iter().map(|k| k.keyword()).collect();
                eprintln!("  valid kinds: {}", valid.join(", "));
                Err(1)
            }
        },
    }
}

/// Run the render command. Returns exit code.
pub fn run(args: RenderArgs) -> i32 {
    match args.format {
        RenderFormat::Json(json_args) => run_json(json_args),
        RenderFormat::Markdown(md_args) => run_markdown(md_args),
    }
}

fn run_json(args: JsonArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let generated = specforge_emitter::render_json(&result.graph, &result.files, &result.config);

    // Ensure output directory exists
    if let Err(e) = std::fs::create_dir_all(&args.dir) {
        eprintln!("specforge: cannot create output directory: {e}");
        return 1;
    }

    let out_path = args.dir.join(&generated.path);
    if let Err(e) = std::fs::write(&out_path, &generated.content) {
        eprintln!("specforge: cannot write {}: {e}", out_path.display());
        return 1;
    }

    eprintln!("wrote {}", out_path.display());
    0
}

fn run_markdown(args: MarkdownArgs) -> i32 {
    let options = match parse_render_options(args.only.as_deref()) {
        Ok(o) => o,
        Err(code) => return code,
    };

    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let generated = specforge_emitter::render_markdown(
        &result.graph,
        &result.files,
        &result.config,
        &options,
    );

    // Ensure output directory exists
    if let Err(e) = std::fs::create_dir_all(&args.dir) {
        eprintln!("specforge: cannot create output directory: {e}");
        return 1;
    }

    for file in &generated {
        let out_path = args.dir.join(&file.path);
        if let Err(e) = std::fs::write(&out_path, &file.content) {
            eprintln!("specforge: cannot write {}: {e}", out_path.display());
            return 1;
        }
        eprintln!("wrote {}", out_path.display());
    }

    0
}
