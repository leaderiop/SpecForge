requirement FEATURE-014 "Cross-Project References" {
  category "Feature"
  tier     v2
  text     """
    Large organizations split specifications across multiple projects (e.g.,
    platform, services, infrastructure). There is no mechanism to reference
    entities across project boundaries, creating information silos where
    architects cannot trace dependencies between related projects.

    Federation via project::entity_id syntax. Projects declare dependencies
    in specforge.json. The compiler loads remote published graphs, resolves
    cross-project references, validates edge compatibility, and creates
    cross-project edges. Missing remotes degrade gracefully with info
    diagnostics — local graphs are always independently valid.
  """
}

requirement FEATURE-043 "Federated Graph Export" {
  category "Feature"
  tier     v2
  text     """
    Even with cross-project references resolved, there is no way to export
    a unified view of the federated graph spanning multiple projects. Agents
    and dashboards need a single graph containing all projects and their
    inter-project relationships.

    specforge export --federated merges local and remote graphs into a
    FederatedGraph with project-qualified entity IDs and cross-project edges.
    The output conforms to the Graph Protocol schema with federation metadata.
  """
}
