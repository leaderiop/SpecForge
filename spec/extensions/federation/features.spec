// Federation features — cross-project references and graph merging

use extensions/federation/behaviors

feature cross_project_references "Cross-Project References" {
  behaviors [load_federation_config, resolve_cross_project_references, load_remote_project_graph, validate_cross_project_edge_consistency]

  problem """
    Large organizations split specifications across multiple projects (e.g.,
    platform, services, infrastructure). There is no mechanism to reference
    entities across project boundaries, creating information silos where
    architects cannot trace dependencies between related projects.
  """

  solution """
    Federation via project::entity_id syntax. Projects declare dependencies
    in specforge.json. The compiler loads remote published graphs, resolves
    cross-project references, validates edge compatibility, and creates
    cross-project edges. Missing remotes degrade gracefully with info
    diagnostics — local graphs are always independently valid.
  """
}

feature federated_graph_export "Federated Graph Export" {
  behaviors [export_federated_graph]

  problem """
    Even with cross-project references resolved, there is no way to export
    a unified view of the federated graph spanning multiple projects. Agents
    and dashboards need a single graph containing all projects and their
    inter-project relationships.
  """

  solution """
    specforge export --federated merges local and remote graphs into a
    FederatedGraph with project-qualified entity IDs and cross-project edges.
    The output conforms to the Graph Protocol schema with federation metadata.
  """
}
