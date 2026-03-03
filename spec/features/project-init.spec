// Project initialization feature

use behaviors/init

feature project_initialization "Project Initialization" {
  behaviors [scaffold_new_project, interactive_plugin_selection, add_plugin_to_existing_project]

  problem """
    New users need a quick way to scaffold a SpecForge project with
    the correct file structure, spec root configuration, and optional
    plugin selection without reading documentation first.
  """

  solution """
    Interactive specforge init command that generates specforge.spec
    with project name, infix, version, and selected plugins. Also
    supports specforge add for adding plugins to existing projects.
  """
}
