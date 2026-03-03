use crate::pipeline;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct ProvidersArgs {
    /// Path to project root (default: current directory)
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

/// Run the providers command. Returns exit code.
pub fn run(args: ProvidersArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    if result.config.provider_schemes.is_empty() {
        println!("No providers configured.");
        return 0;
    }

    // Try to get provider details from JSON config if available
    let json_details = load_json_provider_details(&args.path);

    for scheme in &result.config.provider_schemes {
        let kinds = result
            .config
            .provider_kinds
            .get(scheme)
            .map(|k| format!("[{}]", k.join(", ")))
            .unwrap_or_default();

        if let Some(details) = json_details.as_ref().and_then(|d| d.get(scheme)) {
            println!(
                "  {}  {}  {}  kinds={}",
                scheme, details.package, details.extra_info, kinds
            );
        } else {
            println!("  {}  kinds={}", scheme, kinds);
        }
    }

    0
}

struct ProviderDetail {
    package: String,
    extra_info: String,
}

/// Load provider details from specforge.json if available.
fn load_json_provider_details(
    path: &std::path::Path,
) -> Option<std::collections::HashMap<String, ProviderDetail>> {
    let project_root = pipeline::find_project_root(path)?;
    let json_path = match project_root {
        pipeline::ProjectRoot::Json(p) => p,
        pipeline::ProjectRoot::Spec(_) => return None,
    };
    let config = pipeline::load_json_config(&json_path).ok()?;

    let mut details = std::collections::HashMap::new();
    for (scheme, cfg) in &config.providers {
        let extra = cfg
            .repo
            .as_deref()
            .map(|r| format!("repo={r}"))
            .unwrap_or_default();
        details.insert(
            scheme.clone(),
            ProviderDetail {
                package: cfg.package.clone(),
                extra_info: extra,
            },
        );
    }
    Some(details)
}
