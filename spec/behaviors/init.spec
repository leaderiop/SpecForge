// Initialization behaviors — project scaffolding

use invariants/core
use types/core
use types/config
use ports/outbound

behavior scaffold_new_project "Scaffold New Project" {
  invariants [spec_root_singleton]
  types      [CompilerConfig, PersonaConfig, SurfaceConfig, Persona, Surface]
  ports      [FileSystem]

  contract """
    When specforge init is invoked in an empty directory,
    the system MUST create a specforge.spec file with the user-provided
    project name, infix, and version. The generated spec block MUST be
    syntactically valid and parseable by the compiler.
  """

  verify unit        "scaffold creates valid specforge.spec"
  verify integration "scaffold in non-empty directory preserves existing files"
}

behavior interactive_plugin_selection "Interactive Plugin Selection" {
  invariants [spec_root_singleton]
  types      [CompilerConfig, PluginManifest]
  ports      [FileSystem]

  contract """
    During specforge init, the system MUST present an interactive prompt
    listing available plugins (@specforge/product, @specforge/governance).
    Selected plugins MUST be added to the plugins list in the generated
    specforge.spec. Unselected plugins MUST NOT appear.
  """

  verify unit "selected plugins appear in generated config"
  verify unit "unselected plugins are absent from generated config"
}

behavior add_plugin_to_existing_project "Add Plugin to Existing Project" {
  invariants [spec_root_singleton]
  types      [CompilerConfig, PluginManifest]
  ports      [FileSystem]

  contract """
    When specforge add @specforge/product is invoked on an existing project,
    the system MUST add the plugin to the plugins list in specforge.spec.
    The system MUST NOT duplicate an already-installed plugin.
    The system MUST NOT modify any other field in the spec block.
  """

  verify unit        "add plugin appends to plugins list"
  verify unit        "add duplicate plugin is a no-op with info message"
  verify integration "add plugin preserves all other spec fields"
}
