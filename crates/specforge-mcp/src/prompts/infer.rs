//! `specforge://prompts/infer`: guidance for inferring spec entities from
//! code, by scope.

use serde_json::{Value, json};
use specforge_protocol_types::EntityKindDescriptor;

use specforge_ops::infer::{self, Progress};
use specforge_ops::navigate::{anchors_of_file, source_anchors};

use crate::args::Arguments;
use crate::prompt::{PromptOutcome, Rendered};
use crate::target::Call;
use crate::tool::{ErrorCode, McpError};
use crate::tools::find_spec_for_source::{anchor_json, file_match_name};
use specforge_ops::view::ProjectView;

/// Maximum number of files listed per page in the plan prompt (C9-08).
const MAX_LISTED_FILES: usize = 50;

/// `specforge://prompts/infer`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Scope: omit for overview, 'kind:{name}' for focused guide, 'file:{path}' for file deduplication
    scope: Option<String>,
    /// Directory where generated .spec files are written (scope "plan")
    target_spec_directory: Option<String>,
    /// Offset into the plan's unanalyzed/stale file lists for paging (scope "plan")
    #[arg(default = 0)]
    cursor: usize,
}

/// What the prompt is about: the `scope` argument read.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Scope {
    /// No scope, or one the prompt does not know: every kind's guide.
    Overview,
    /// `kind:<name>`, the name lowercased.
    Kind(String),
    /// `file:<path>`.
    File(String),
    /// `plan`.
    Plan,
    /// `workflow`.
    Workflow,
}

impl Scope {
    /// The scope `scope` names. An empty kind or file is invalid input.
    fn parse(scope: Option<&str>) -> Result<Scope, Box<McpError>> {
        let invalid = |message: &str| {
            Box::new(McpError::new(ErrorCode::InvalidInput, message).with_argument("scope"))
        };
        Ok(match scope {
            Some("plan") => Scope::Plan,
            Some("workflow") => Scope::Workflow,
            Some(s) if s.starts_with("kind:") => {
                let kind = s["kind:".len()..].to_string();
                if kind.is_empty() {
                    return Err(invalid("Empty kind name in scope 'kind:'"));
                }
                Scope::Kind(kind)
            }
            Some(s) if s.starts_with("file:") => {
                let file = &s["file:".len()..];
                if file.is_empty() {
                    return Err(invalid("Empty file path in scope 'file:'"));
                }
                Scope::File(file.to_string())
            }
            _ => Scope::Overview,
        })
    }
}

/// Page `files` to the window starting at `cursor`, capped at
/// MAX_LISTED_FILES entries, with a trailing "... and K more (use the
/// cursor param)" marker when the tail was cut (C9-08).
fn page_files(files: &[String], cursor: usize) -> Vec<Value> {
    let start = cursor.min(files.len());
    let end = (start + MAX_LISTED_FILES).min(files.len());
    let mut page: Vec<Value> = files[start..end]
        .iter()
        .map(|f| Value::from(f.as_str()))
        .collect();
    let remaining = files.len() - end;
    if remaining > 0 {
        page.push(Value::from(format!(
            "... and {} more (use the cursor param)",
            remaining
        )));
    }
    page
}

pub fn render(call: &Call<'_>, args: Args) -> PromptOutcome {
    respond(&call.view(), args)
}

/// The prompt over `project`.
fn respond(project: &ProjectView, args: Args) -> PromptOutcome {
    match Scope::parse(args.scope.as_deref())? {
        Scope::Plan => get_plan(project, args.target_spec_directory.as_deref(), args.cursor),
        Scope::Workflow => Ok(get_workflow(project)),
        Scope::Kind(kind) => get_kind_scoped(project, &kind),
        Scope::File(file) => get_file_scoped(project, &file),
        Scope::Overview => Ok(get_overview(project)),
    }
}

/// A rendered prompt.
fn rendered(instruction: impl Into<String>, payload: Value) -> Rendered {
    Rendered {
        instruction: instruction.into(),
        payload,
    }
}

fn get_overview(project: &ProjectView) -> Rendered {
    let guide = infer::guide(project);
    let mut payload = guide.to_json();
    payload["output_format"] = Value::from(format!(
        "Write .spec files in the {} directory. Use `keyword entity_id \"Title\" {{ fields }}` syntax. Entity IDs are snake_case identifiers (letters, digits, underscores, 2-60 chars).",
        guide.spec_directory
    ));
    payload["validation"] = Value::from(validation());

    let instruction = "You are inferring spec entities from this codebase. \
        Use the inference guides below to identify entities in the code, \
        write .spec files, and validate them with specforge_validate. \
        Each kind has signals describing what to look for in code. \
        Do not duplicate entities that already exist.";

    rendered(instruction, payload)
}

fn get_kind_scoped(project: &ProjectView, kind_name: &str) -> PromptOutcome {
    let guide = infer::kind_guide(project, kind_name)
        .map_err(|error| Box::new(McpError::from(error).with_argument("scope")))?;
    let mut payload = guide.to_json();
    payload["validation"] = Value::from(validation());

    let instruction = format!(
        "You are inferring '{kind_name}' entities from this codebase. \
         Use the guide below. Do not duplicate the existing entity IDs listed."
    );

    Ok(rendered(instruction, payload))
}

/// What the agent runs after writing `.spec` files.
fn validation() -> String {
    "After writing .spec files, call specforge_validate to check for errors (and specforge_analyze for coverage/contract findings). Fix any errors before proceeding.".to_string()
}

fn get_file_scoped(project: &ProjectView, file_path: &str) -> PromptOutcome {
    // The entities anchored to the file: the one file rule
    // (specforge_ops::navigate::anchors_of_file) over the anchors manifest,
    // the answer specforge.find_spec_for_source gives (C9-09). With no
    // project there is no manifest.
    let manifest = source_anchors(project).map_err(McpError::from)?;
    let found = anchors_of_file(&manifest, file_path);
    let referencing_entities: Vec<Value> = found
        .anchors
        .iter()
        .map(|anchor| anchor_json(anchor, project.graph()))
        .collect();
    let match_mode = file_match_name(found.mode);

    let guide = infer::guide(project);
    let kinds_info: Vec<Value> = guide
        .kinds
        .iter()
        .map(|kind| json!({ "kind": kind.keyword, "inference_guide": kind.guide }))
        .collect();

    let result = json!({
        "file": file_path,
        "match_mode": match_mode,
        "existing_entities_referencing_file": referencing_entities,
        "kinds": kinds_info,
        "project_conventions": guide.conventions.unwrap_or(""),
        "validation": validation(),
    });

    let instruction = format!(
        "You are inferring spec entities from the file '{}'. \
         The entities listed below already reference this file — do not duplicate them. \
         Use the kind guides to identify new entities.",
        file_path
    );

    Ok(rendered(instruction, result))
}

fn get_plan(
    project: &ProjectView,
    target_spec_directory: Option<&str>,
    cursor: usize,
) -> PromptOutcome {
    let target_spec_directory = target_spec_directory.unwrap_or("spec/");

    // Nothing is planned from a specforge-infer.json that cannot be used:
    // every mark_analyzed the plan sent the agent to make would be refused.
    // Without a root there is nothing to count.
    let progress = match project.root() {
        None => Progress::none(),
        Some(_) => specforge_ops::infer::progress(project).map_err(McpError::from)?,
    };
    let (summary, unanalyzed, stale) = (progress.summary, progress.unanalyzed, progress.stale);

    let kind_priorities: Vec<Value> = project
        .registries()
        .declarations()
        .iter()
        .flat_map(|d| d.entities.iter().map(move |k| (d, k)))
        .map(|(d, k)| {
            let keyword = keyword(k).to_string();
            let existing_count = project
                .graph()
                .nodes()
                .into_iter()
                .filter(|n| n.kind.raw == keyword.as_str())
                .count();
            json!({
                "kind": keyword,
                "extension": d.name(),
                "existing_count": existing_count,
            })
        })
        .collect();

    // File lists are capped to a page so prompt size stays bounded
    // regardless of project size (C9-08); the remainder pages via `cursor`.
    let unanalyzed_page = page_files(&unanalyzed, cursor);
    let stale_page = page_files(&stale, cursor);
    let next_cursor = if unanalyzed.len() > cursor + MAX_LISTED_FILES
        || stale.len() > cursor + MAX_LISTED_FILES
    {
        Some(cursor + MAX_LISTED_FILES)
    } else {
        None
    };

    let result = json!({
        "plan": {
            "target_spec_directory": target_spec_directory,
            "progress": {
                "files_total": summary.files_total,
                "files_analyzed": summary.files_analyzed,
                "entities_produced": summary.entities_produced,
            },
            "cursor": cursor,
            "next_cursor": next_cursor,
            "unanalyzed_files": unanalyzed_page,
            "unanalyzed_total": unanalyzed.len(),
            "stale_files": stale_page,
            "stale_total": stale.len(),
            "kind_priorities": kind_priorities,
        }
    });

    let instruction = format!(
        "Create a prioritized inference plan. There are {} unanalyzed files and {} stale files. \
         Write .spec files to '{}'. Process files with the most entity signals first. \
         Use specforge.infer_session to track progress (start → mark_analyzed per file → end). \
         After each file, call specforge.validate to check for errors.",
        unanalyzed.len(),
        stale.len(),
        target_spec_directory
    );

    Ok(rendered(instruction, result))
}

fn get_workflow(project: &ProjectView) -> Rendered {
    let tool_names: Vec<&str> = vec![
        "specforge.infer_session",
        "specforge.infer_progress",
        "specforge.validate",
        "specforge.query",
        "specforge.search",
        "specforge.schema",
    ];

    let installed_kinds: Vec<String> = project
        .registries()
        .declarations()
        .iter()
        .flat_map(|d| d.entities.iter())
        .map(|k| keyword(k).to_string())
        .collect();

    let result = json!({
        "tools": tool_names,
        "installed_kinds": installed_kinds,
    });

    let workflow = "\
## Inference Workflow Protocol

### Step 1: Start Session
Call `specforge.infer_session` with `action: \"start\"` and `agent: \"<your-id>\"`.
Optionally set `source_roots` to limit scanning scope.

### Step 2: Check Progress
Call `specforge.infer_progress` to see unanalyzed files and current project.

### Step 3: For Each Source File
1. Read the source file
2. Identify entities (behaviors, types, events, etc.) using entity kind guides
3. Write a `.spec` file with the discovered entities
4. Call `specforge.validate` to check for errors — fix any before proceeding
5. Call `specforge.infer_session` with `action: \"mark_analyzed\"`, `source_file`, and `entities_produced`

### Step 4: Validate Continuously
After every 3-5 files, call `specforge.validate` to catch cross-file issues.
Use `specforge.search` to find existing entities and avoid duplicates.
Use `specforge.query` to check how new entities connect to the graph.

### Step 5: End Session
Call `specforge.infer_session` with `action: \"end\"` and the `session_id` from Step 1.
Use `status: \"completed\"` when done, or `status: \"paused\"` to resume later.

### Retry Pattern
If validation fails, fix the .spec file and re-validate. Do not skip errors.
If a file has no identifiable entities, still mark it as analyzed with an empty `entities_produced`.
";

    rendered(workflow, result)
}

/// The keyword a kind is written with: its declared keyword, else its name.
fn keyword(kind: &EntityKindDescriptor) -> &str {
    kind.keyword.as_deref().unwrap_or(&kind.name)
}
