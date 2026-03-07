// Initialization behaviors — project scaffolding

use invariants/core
use invariants/zero-entity-core
use invariants/wasm
use types/config
use ports/outbound
use behaviors/wasm-extensions
use events/compilation

// L5: project_initialized event boundary — scaffold_new_project fires
// project_initialized when extensions are selected; graceful_zero_extension_init
// fires it when zero extensions are selected. These are mutually exclusive paths
// in interactive mode (exactly one fires per init invocation).
behavior scaffold_new_project "Scaffold New Project" {
  category   command
  invariants [spec_root_singleton, init_config_validity, zero_domain_knowledge_core]
  types      [CompilerConfig, InitConfig, InitError]
  ports      [FileSystem]
  produces   [project_initialized]

  contract """
    When specforge init is invoked in an empty directory,
    the system MUST create a specforge.json file with the user-provided
    project name and version. The generated config MUST include the
    $schema field pointing to the SpecForge JSON schema URL to enable
    IDE autocomplete. The generated config MUST be syntactically valid
    and parseable by the compiler. If a specforge.json already exists
    in the current directory OR in any ancestor directory (as resolved
    by find_project_root()), the system MUST reject the operation with
    an error message and exit code 1. The full init-check-export cycle
    MUST complete in under 60 seconds on commodity hardware, enforcing
    Principle 8 (seconds to value).
  """

  verify unit        "scaffold creates valid specforge.json"
  verify unit        "scaffold includes $schema field in generated config"
  verify unit        "scaffold rejects when specforge.json already exists"
  verify unit        "scaffold rejects when a parent directory contains specforge.json"
  verify performance "full init-check-export cycle completes in under 60 seconds"
  verify integration "scaffold in non-empty directory preserves existing files"

}

// Sub-step of scaffold_new_project — not an independent entry point
behavior scaffold_starter_spec_file "Scaffold Starter Spec File" {
  category   command
  invariants [zero_domain_knowledge_core]
  types      [CompilerConfig]
  ports      [FileSystem]
  // No produces — sub-step of scaffold_new_project

  contract """
    During specforge init, after creating specforge.json, the system
    MUST create a starter .spec file (e.g., hello.spec) demonstrating
    the basic DSL syntax using only structural syntax: generic entity
    blocks with string fields and reference lists. The core init command
    MUST NOT generate domain-specific content or reference extension
    keywords — the core compiler has zero domain knowledge (Principle 2).
    Extensions MAY contribute starter templates via their manifest
    metadata; if an installed extension declares a starter template,
    init MAY use it, but this is extension-provided content, not
    core-generated content. The starter file MUST be valid and pass
    specforge check with zero diagnostics regardless of which extensions
    are installed. This behavior delivers on Principle 8 (seconds to
    value): the user can run specforge check immediately after init.
  """

  verify unit        "starter spec file is created alongside specforge.json"
  verify unit        "starter spec file passes specforge check with zero errors"
  verify unit        "starter file uses only structural syntax when no extensions contribute templates"
  verify unit        "starter file contains no domain-specific keywords from extensions"
  verify integration "extension-contributed starter templates are used when available"

}

// Sub-step of scaffold_new_project — does not produce an independent event.
// The project_initialized event is produced by the parent orchestrator.
behavior interactive_extension_selection "Interactive Extension Selection" {
  category   command
  invariants [spec_root_singleton, init_config_validity, zero_domain_knowledge_core]
  types      [CompilerConfig, BundledExtensionCatalog]
  ports      [FileSystem, RegistryClient]

  contract """
    During specforge init, the system MUST discover available extensions
    (from registry, local cache, or bundled index) and present them for
    interactive selection. The system SHOULD indicate commonly-used
    extensions as determined by registry metadata (e.g., download counts,
    curated lists) but MUST NOT pre-select
    any extension by default. Selected extensions MUST be added to the
    extensions list in the generated specforge.json. Unselected extensions
    MUST NOT appear. When no TTY is available (e.g., piped input in CI),
    the system MUST skip interactive prompts and proceed with zero
    extensions selected.
    If no registry is reachable, the system MUST fall back to a bundled
    extension index. The init flow MUST NOT fail due to registry
    unavailability. If both the registry and the bundled index are
    unavailable, the system MUST proceed with zero extensions and emit
    an I-level diagnostic explaining that no extension catalog was available.
  """

  verify unit "selected extensions appear in generated config"
  verify unit "unselected extensions are absent from generated config"
  verify unit "no extensions are pre-selected by default"
  verify unit "interactive prompts are skipped when no TTY is available"
  verify unit "registry unavailability falls back to bundled extension index"
  verify unit "init does not fail when registry is unreachable"
  verify unit "missing registry and bundled index proceeds with zero extensions and info diagnostic"

}

behavior non_interactive_init "Non-Interactive Init" {
  category   command
  invariants [spec_root_singleton, init_config_validity, zero_domain_knowledge_core]
  types      [CompilerConfig, InitConfig, InitOutput, InitError]
  ports      [FileSystem, RegistryClient]
  produces   [project_initialized]

  contract """
    When specforge init is invoked with --name and optional --extensions
    flags, the system MUST skip all interactive prompts and create the
    project non-interactively. This enables CI pipelines, scripts, and
    automated tooling to scaffold projects without user interaction.
    The generated specforge.json MUST be identical in structure to
    one created interactively with the same inputs. When --format=json
    is specified, output MUST be a JSON object:
    { project_root, config_path, spec_file_path, extensions_installed[] }.
    When --version is specified, it MUST override the default version
    (0.1.0) in the generated specforge.json.
    If --extensions specifies an extension name that cannot be resolved
    (not found in registry, bundled index, or local cache), the system
    MUST reject the operation with a diagnostic naming the unresolvable
    extension and exit code 1.
  """

  verify unit "non-interactive init creates valid specforge.json"
  verify unit "non-interactive init skips all prompts"
  verify unit "non-interactive init with --extensions populates extensions list"
  verify unit "non-interactive init with unknown extension rejects with diagnostic and exit code 1"
  verify unit "invalid project name is rejected with InitError::invalid_name"
  verify integration "non-interactive output matches interactive output for same inputs"
  verify unit "non-interactive init with --format=json outputs InitOutput JSON"
  verify unit "non-interactive init with --version overrides default version in specforge.json"
  verify integration "non_interactive_init completes full init-check-export cycle in under 60 seconds"

}

// L7: Lock file interaction (download, integrity checks, version pinning) is
// handled by the write_lock_file behavior — see behaviors/wasm-extensions.spec.
behavior add_extension_to_existing_project "Add Extension to Existing Project" {
  category   command
  invariants [spec_root_singleton, init_config_validity, peer_dependency_satisfaction]
  types      [CompilerConfig]
  ports      [FileSystem, RegistryClient]
  produces   [extension_added]

  contract """
    When specforge add <extension-specifier> is invoked on an existing project,
    the system MUST add the extension to the extensions list in specforge.json.
    The extension specifier MUST accept @scope/name@version syntax; version
    resolution is delegated to parse_extension_specifier from the wasm
    behaviors. If no version is specified, the system MUST resolve to the
    latest compatible version.
    The system MUST NOT duplicate an already-installed extension.
    The system MUST NOT modify any other field in specforge.json.
    When adding an extension, the compiler MUST check peer dependencies
    of the new extension. Unsatisfied peer dependencies MUST be reported
    as E-level diagnostics naming each missing peer and the operation
    MUST be rejected with exit code 1.
    If no specforge.json exists in the current directory or any ancestor
    directory (as resolved by find_project_root()), the system MUST reject
    the operation with an error message and exit code 1. If the extension
    specifier cannot be resolved, the system MUST reject the operation with
    a diagnostic naming the unresolvable extension.
  """

  verify unit        "add extension appends to extensions list"
  verify unit        "add duplicate extension is a no-op with info message"
  verify unit        "add extension with no specforge.json rejects with error and exit code 1"
  verify unit        "add unresolvable extension rejects with diagnostic"
  verify unit        "add extension with @scope/name@version resolves version via parse_extension_specifier"
  verify unit        "add extension without version resolves to latest compatible version"
  verify unit        "add extension with unsatisfied peer dependencies emits error diagnostics and rejects"
  verify integration "add extension preserves all other config fields"

}

behavior graceful_zero_extension_init "Graceful Zero-Extension Init" {
  category   command
  invariants [spec_root_singleton, init_config_validity, zero_domain_knowledge_core]
  types      [CompilerConfig]
  ports      [FileSystem]
  produces   [project_initialized]

  contract """
    When specforge init completes with zero extensions selected,
    the system MUST still produce a valid specforge.json with an
    empty extensions list. The generated starter .spec file MUST
    use only structural syntax (entity blocks with string fields
    and reference lists) that the core compiler can parse without
    any extensions. Running specforge check on the resulting
    project MUST produce zero errors. Running specforge export
    MUST produce a valid Graph Protocol JSON with the structural
    entities. This ensures Principle 1 (structure is a spectrum)
    and Principle 8 (seconds to value): even a project with zero
    domain extensions provides value.
  """

  verify unit        "zero-extension init creates valid specforge.json with empty extensions"
  verify unit        "zero-extension starter file passes specforge check"
  verify integration "zero-extension project produces valid graph via specforge export"
  verify integration "graceful_zero_extension_init completes full init-check-export cycle in under 60 seconds"

}

behavior find_project_root "Find Project Root" {
  category   internal
  contract """
    The system MUST locate the project root by walking from the current
    directory upward to the filesystem root. At each directory level,
    the system MUST check for both specforge.json and specforge.spec
    before ascending to the parent. Within a single directory,
    specforge.json takes precedence over specforge.spec. The first
    directory containing either file wins (closest-wins semantics).
    The system MUST resolve symlinks before path comparison to avoid
    infinite loops caused by circular symlink chains. If neither file
    is found in any directory up to the filesystem root, the function
    MUST return None. The calling command decides how to handle a
    missing project root (e.g., init may proceed, check may emit a
    diagnostic). This function MUST NOT contain command-specific logic.
  """
  types      [CompilerConfig]
  ports      [FileSystem]
  invariants [spec_root_singleton]

  verify unit "specforge.json found in current directory"
  verify unit "specforge.json found in ancestor directory"
  verify unit "specforge.spec found when specforge.json is absent at same level"
  verify unit "closest directory wins over ancestor directory"
  verify unit "specforge.json takes precedence over specforge.spec in same directory"
  verify unit "symlinks are resolved before path comparison"
  verify unit "circular symlink chain does not cause infinite loop"
  verify unit "no config found returns None"
}
