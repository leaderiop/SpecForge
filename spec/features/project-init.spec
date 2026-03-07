// Project initialization feature

use behaviors/init
use behaviors/mcp-operations

feature project_initialization "Project Initialization" {
  behaviors [
    scaffold_new_project, scaffold_starter_spec_file,
    interactive_extension_selection, non_interactive_init,
    add_extension_to_existing_project, graceful_zero_extension_init,
    find_project_root,
    // Bridge: MCP agent init surface (see features/mcp.spec::mcp_mutation_tools)
    provide_mcp_init_tool,
  ]

  problem """
    New users and AI agents need a quick way to scaffold a SpecForge
    project that produces a consumable graph. The correct file structure,
    specforge.json configuration, and optional extension selection must
    work without reading documentation first. CI pipelines and automated
    tooling need a non-interactive path to the same outcome.
  """

  solution """
    Interactive specforge init command that generates specforge.json
    with project name, version, and selected extensions, plus a starter
    .spec file that validates immediately. Non-interactive mode via
    --name flag enables scripted project creation. specforge add
    supports adding extensions to existing projects. The full
    init-check-export path produces Graph Protocol JSON that any AI
    agent can consume immediately. Together these deliver Principle 8:
    install to first validated output in under 60 seconds via three
    commands (init, check, export). The MCP agent init surface is
    provided by provide_mcp_init_tool (bridged above from mcp-operations).
  """
}
