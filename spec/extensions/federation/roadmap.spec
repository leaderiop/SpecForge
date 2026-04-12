// Federation roadmap — cross-project federation delivery phase

use "extensions/federation/behaviors"
use "extensions/federation/features"
roadmap federation_phase "Phase 9b: Cross-Project Federation" {
  status     planned
  features   [cross_project_references, federated_graph_export]

  criteria [
    "project::entity_id syntax resolves cross-project references",
    "Remote project graphs loaded from local filesystem path",
    "Cross-project edge compatibility validated",
    "specforge export --federated merges local and remote graphs",
    "Missing remote projects degrade gracefully with I012 info",
  ]
}
