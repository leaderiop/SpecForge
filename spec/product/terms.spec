// Terms — domain vocabulary for the SpecForge product
//
// First-class entity kinds declared by @specforge/product.
// These terms define concepts used across the product planning domain
// that benefit from precise, shared definitions to prevent terminology
// drift between spec authors, agents, and documentation.

term graph_protocol "Graph Protocol" {
  definition "The JSON schema output format produced by specforge export. Contains all entities, edges, metadata, and schema version. The graph protocol is the primary contract between SpecForge and its consumers (agents, dashboards, CI tools)."
  context    "Used in export commands, MCP resources, and agent consumption. The graph is the product — not the compiler, not the DSL."
  aliases    ["graph_schema", "entity_graph"]
  see_also   [surface_contribution, traceability_chain, t_entity_enhancement, impact_analysis, schema_evolution, lifecycle_consistency, owner, effort, release_entity, blocker, release_status_lifecycle]
  tags       ["core", "output"]
}

term surface_contribution "Surface Contribution" {
  definition "A CLI command, MCP tool, or MCP resource declared by an extension in its manifest surfaces field. CLI commands are auto-promoted to MCP tools. Surface contributions are the primary way extensions expose functionality to users and agents."
  context    "Declared in ManifestV2 surfaces field. Each CLI command maps to a cmd__{id} Wasm export; each MCP tool/resource maps to an mcp__{name} Wasm export."
  aliases    ["surface", "cli_command", "mcp_tool"]
  see_also   [declarative_validation]
  tags       ["core", "extension"]
}

term diagnostic "Diagnostic" {
  definition "A compiler message emitted during validation with a severity level (error, warning, info), a unique code (E/W/I prefix + number), a source location (file, line, column), and a human-readable message. Diagnostics are the primary feedback mechanism for spec quality."
  context    "E-codes block compilation, W-codes warn but pass, I-codes inform. Each extension owns a range of diagnostic codes. Diagnostics include source context and did-you-mean suggestions."
  aliases    ["compiler_message", "validation_error"]
  see_also   [orphan_entity, cycle_detection, declarative_validation, health_score, graph_diff, fuzzy_matching]
  tags       ["core", "validation"]
}

term orphan_entity "Orphan Entity" {
  definition "An entity with zero incoming edges of the expected type(s) for its kind. Orphan detection produces warnings (W-codes) or info diagnostics (I-codes) depending on the entity kind and the severity of being unreferenced."
  context    "Orphan features (W041), journeys (W042), modules (W044), terms (I010), personas (I046), channels (I047). Orphan detection uses the no_incoming_edges validation pattern."
  aliases    ["unreferenced_entity", "disconnected_entity"]
  see_also   [traceability_chain]
  tags       ["validation", "graph"]
}

term cycle_detection "Cycle Detection" {
  definition "The process of finding circular dependencies in a directed graph. SpecForge detects cycles in module (E007), milestone (E015), feature (W045), and deliverable (E016) dependency graphs using Tarjan's algorithm. Cycles in E-coded graphs are errors; cycles in W-coded graphs are warnings."
  context    "All dependency graphs (depends_on fields) must form DAGs. Cycle members are named in the diagnostic message."
  aliases    ["circular_dependency", "dag_violation"]
  tags       ["validation", "graph"]
}

term traceability_chain "Traceability Chain" {
  definition "The path through the entity graph from a user need (journey) through planning (feature, milestone) to structural implementation (module) to shippable artifact (deliverable). A complete traceability chain means every journey feature is reachable from a deliverable via both journey and module paths."
  context    "specforge trace prints the chain. Gaps in the chain produce diagnostics (I049 for deliverable journey-module gap, I051 for milestone feature-module gap)."
  aliases    ["trace_chain", "requirement_trace"]
  see_also   [coverage]
  tags       ["core", "planning"]
}

term completion_ratio "Completion Ratio" {
  definition "The fraction of features with status=done within a milestone or deliverable. Computed as done_count / total_features for milestones, or completed_milestones / total_milestones for deliverables. A ratio of 1.0 means all scheduled work is done. A ratio of 0.0 means nothing is done (including when there are zero features)."
  context    "Queried via pe_query_milestone_completion and pe_query_deliverable_completion. Used by product managers to track progress."
  aliases    ["progress_ratio", "done_ratio"]
  see_also   [coverage, feature_status_lifecycle, milestone_status_lifecycle]
  tags       ["metrics", "planning"]
}

term coverage "Coverage" {
  definition "The fraction of features referenced by a journey that have status=done. Journey coverage measures how much of a user experience has been implemented. Unlike completion_ratio (which measures milestone/deliverable progress), coverage measures journey-level feature readiness."
  context    "Queried via pe_query_journey_coverage. Features without a status field are treated as uncovered."
  aliases    ["journey_coverage", "feature_coverage"]
  tags       ["metrics", "planning"]
}

term declarative_validation "Declarative Validation" {
  definition "The pattern-based validation engine where extensions declare validation rules as data (ValidationRulePattern entries in the manifest) rather than imperative code. The core compiler interprets these patterns. Patterns include: no_incoming_edges, no_outgoing_edges, missing_field_when_flag_set, field_value_constraint, cycle_detection, file_exists, and custom (Wasm-backed)."
  context    "All 47 product diagnostic codes are declared as validation patterns. The engine executes them deterministically in diagnostic-code order."
  aliases    ["pattern_validation", "rule_engine"]
  tags       ["core", "validation"]
}

term t_entity_enhancement "Entity Enhancement" {
  definition "A mechanism by which one extension adds fields to another extension's entity kinds without modifying the original extension. Declared in the enhancing extension's manifest via entity_enhancements. Enables cross-extension composition without coupling."
  context    "Example: @specforge/software adds a behaviors field to @specforge/product's milestone entity kind via entity_enhancements. The product extension itself declares no entity_enhancements — it is standalone."
  aliases    ["field_enhancement", "cross_extension_field"]
  see_also   [surface_contribution, peer_dependency]
  tags       ["extension", "architecture"]
}

term peer_dependency "Peer Dependency" {
  definition "A declaration in an extension's manifest that it requires another extension to be installed for certain features to work. Peer dependencies enable cross-extension edges and entity enhancements while keeping each extension independently installable."
  context    "Example: @specforge/software declares peer_dependency on @specforge/product to create Implements edges (behavior->feature). The product extension has no peer dependencies — it is fully standalone."
  aliases    ["extension_dependency", "peer_dep"]
  see_also   [cross_extension_coexistence]
  tags       ["extension", "architecture"]
}

term cross_extension_coexistence "Cross-Extension Coexistence" {
  definition "The behavior of multiple installed extensions operating on the same entity graph without conflicts. Each extension owns its entity kinds, edge types, and diagnostic codes. Cross-extension interactions occur only via peer_dependency declarations and entity_enhancements. Product queries operate exclusively on product-owned entities and edges."
  context    "When @specforge/software is co-installed with @specforge/product, software creates Implements edges (behavior->feature) but product queries do not traverse these edges. Product queries are standalone by design."
  aliases    ["extension_coexistence", "multi_extension"]
  tags       ["extension", "architecture"]
}

term feature_status_lifecycle "Feature Status Lifecycle" {
  definition "The progression of a feature through FeatureStatus values: proposed -> accepted -> in_progress -> done. Features may also be deferred (with a reason field documenting justification). Status drives completion_ratio and coverage calculations. Features without a status field are treated as not-done for coverage purposes."
  aliases    ["feature_lifecycle"]
  see_also   [coverage, milestone_status_lifecycle, deliverable_status_lifecycle]
  tags       ["lifecycle", "planning"]
}

term milestone_status_lifecycle "Milestone Status Lifecycle" {
  definition "The progression of a milestone through MilestoneStatus values: planned -> in_progress -> completed. Milestones may also be blocked (with a depends_on reference documenting what blocks them). Completed milestones should have non-empty exit_criteria. Blocked milestones should have dependencies."
  aliases    ["milestone_lifecycle"]
  see_also   [deliverable_status_lifecycle]
  tags       ["lifecycle", "planning"]
}

term deliverable_status_lifecycle "Deliverable Status Lifecycle" {
  definition "The progression of a deliverable through DeliverableStatus values: draft -> in_progress -> shipped. Deliverables may also be deprecated (with a reason field documenting justification). Shipped deliverables should have all referenced milestones completed. Absent status is treated as draft for incremental adoption."
  aliases    ["deliverable_lifecycle"]
  tags       ["lifecycle", "planning"]
}

term domain_neutral "Domain-Neutral" {
  definition "A design principle where product entities contain no domain-specific fields. Features use problem/solution framing, not story points or sprints. Journeys use free-form flow steps, not domain-specific step types. This ensures the product extension works for any industry: healthcare, finance, education, aerospace, or any other domain."
  context    "Core design principle of @specforge/product. Domain-specific fields are added by domain extensions via entity_enhancements and peer_dependency."
  aliases    ["domain_agnostic", "industry_neutral"]
  see_also   [peer_dependency]
  tags       ["principle", "architecture"]
}

term planning_entity "Planning Entity" {
  definition "One of the 9 entity kinds declared by @specforge/product: feature, journey, deliverable, milestone, module, term, persona, channel, release. All planning entities have testable=false — they describe what to build and when, not how the system behaves. Planning entities form a traceability chain from user need (journey) through scheduling (milestone) to shipped artifact (deliverable), with coordinated shipping via release."
  context    "Distinct from domain entities (behavior, invariant, event, type, port) declared by @specforge/software. Planning entities are tested indirectly through domain extensions."
  aliases    ["product_entity", "planning_construct"]
  see_also   [traceability_chain, domain_neutral, feature_status_lifecycle]
  tags       ["core", "planning"]
}

term product_graph "Product Graph" {
  definition "The subgraph of the SpecForge entity graph consisting of only the 9 product entity kinds and 16 product edge types. Product queries operate exclusively on this subgraph — they do not traverse edges owned by other extensions. The product graph enables planning-to-delivery traceability without coupling to domain-specific extensions."
  context    "Product graph isolation is enforced by pe_cross_extension_query_isolation. The graph is the product of @specforge/product — consumed by product managers, agents, and dashboards."
  aliases    ["product_subgraph", "planning_graph"]
  see_also   [graph_protocol, traceability_chain, cross_extension_coexistence]
  tags       ["core", "graph"]
}

term health_score "Health Score" {
  definition "A composite metric (0.0–1.0) that aggregates five product quality sub-scores: completion ratio (30%), coverage ratio (25%), orphan ratio (20%), cycle count (15%), and error ratio (10%). Maps to a grade: excellent (>=0.9), good (>=0.7), fair (>=0.5), poor (>=0.3), critical (<0.3). Queried via pe_query_product_health."
  context    "Provides a single number for quick project health assessment. Empty projects score 1.0. Weights are fixed in v1."
  aliases    ["product_health", "composite_health"]
  see_also   [completion_ratio, coverage, scalability_tier]
  tags       ["metrics", "planning"]
}

term impact_analysis "Impact Analysis" {
  definition "A transitive graph traversal that computes all entities affected by a change to a given feature. Starting from a feature, follows reverse JourneyFeature, MilestoneFeature, ModuleFeature to find affected containers, then DeliverableJourney/DeliverableModule to find affected deliverables, and forward FeatureDependsOn for transitive dependent features."
  context    "Queried via pe_query_feature_impact. Answers: 'What breaks if I defer this feature?' Used by product managers for prioritization decisions."
  aliases    ["blast_radius", "change_impact"]
  see_also   [traceability_chain, what_if_simulation]
  tags       ["metrics", "graph"]
}

term what_if_simulation "What-If Simulation" {
  definition "A read-only query that applies a hypothetical action (defer, remove, block, complete) to a temporary graph clone and computes before/after metrics without mutating the real graph. Returns completion and coverage deltas, affected milestones and deliverables, and status boundary crossing predictions."
  context    "Queried via pe_query_what_if. The graph is NEVER mutated. Four actions: defer (status=deferred), remove (delete entity), block (status=blocked), complete (status=done/completed)."
  aliases    ["simulation", "hypothetical_query"]
  see_also   [completion_ratio, coverage]
  tags       ["metrics", "query"]
}

term graph_diff "Graph Diff" {
  definition "A structural comparison between two graph snapshots showing added/removed/modified entities, added/removed edges, and status field changes. Snapshots are stored in .specforge/snapshots/ after each successful build with configurable retention (default 50 builds)."
  context    "Queried via pe_query_graph_diff. Answers: 'What changed since last build/sprint/milestone?' Complements delta notifications (real-time) with historical comparison."
  aliases    ["snapshot_diff", "graph_comparison"]
  see_also   [graph_protocol, traceability_chain]
  tags       ["metrics", "graph"]
}

term query_cache "Query Cache" {
  definition "An in-memory cache of product query results keyed on (query_name, entity_id). Lazily populated on first query, atomically invalidated when any graph rebuild begins. Eliminates redundant graph traversals for repeated queries in watch mode and MCP server mode."
  context    "Managed by pe_cache_query_results. Cache is not persisted across compiler restarts. Hit rate observable via product.query_cache_state channel."
  aliases    ["result_cache"]
  see_also   [product_graph, scalability_tier]
  tags       ["performance", "infrastructure"]
}

term schema_evolution "Schema Evolution" {
  definition "The strategy for evolving product entity shapes across extension versions. Minor versions (1.x) use additive-only changes: new @optional fields, no removals or renames. Major versions (2.0) may break schemas but must provide a migration_hook Wasm export that transforms old entities to new format."
  context    "Governed by pe_schema_evolution_strategy decision. migration_hook is null in v1 (no prior version). Absent fields treated as null — no migration needed for new @optional fields."
  aliases    ["schema_migration", "version_evolution"]
  see_also   [declarative_validation, planning_entity]
  tags       ["architecture", "evolution"]
}

term scalability_tier "Scalability Tier" {
  definition "One of three performance tiers defined for the @specforge/product extension: Tier 1 (up to 500 entities, validation <50ms, queries <100ms), Tier 2 (up to 5000 entities, validation <500ms, queries <1s), Tier 3 (over 5000 entities, validation <2s, queries <2s with timeout). Tiers provide graduated performance expectations for different project scales."
  context    "Defined by decision pe_scalability_tiers. Tier 1 targets are MUST-level; Tier 2 and 3 are SHOULD-level."
  aliases    ["performance_tier", "scale_tier"]
  see_also   [product_graph, completion_ratio]
  tags       ["performance", "infrastructure"]
}

term lifecycle_consistency "Lifecycle Consistency" {
  definition "The principle that entities with status-based lifecycles (feature, milestone, deliverable, persona, channel) enforce consistent documentation requirements at each lifecycle stage. Deferred features need a reason (I059). Blocked milestones need a reason (I060). Deprecated deliverables need a reason (I066). Deprecated personas need a reason (I069). Deprecated channels need a reason (I070). Shipped deliverables need completed milestones (I065)."
  context    "All lifecycle validations are info-level to support incremental adoption. Reason content is opaque — only presence is validated per pe_reason_content_opaque."
  aliases    ["status_consistency", "lifecycle_hygiene"]
  see_also   [feature_status_lifecycle, milestone_status_lifecycle, deliverable_status_lifecycle]
  tags       ["lifecycle", "validation"]
}

// ---------------------------------------------------------------------------
// v1.1 terms — ownership, effort, release, blockers
// ---------------------------------------------------------------------------

term owner "Owner" {
  definition "A free-form string identifying the person or team responsible for a feature, milestone, deliverable, or release. Not an entity reference — supports any naming convention (email, handle, team name)."
  context    "Used on features, milestones, deliverables, and releases"
  aliases    ["assignee", "responsible"]
  see_also   [contributors]
  tags       ["ownership", "planning"]
}

term contributors "Contributors" {
  definition "A list of free-form strings identifying people or teams contributing to a feature, milestone, deliverable, or release. Complementary to owner — owner is the single accountable party, contributors are additional participants."
  context    "Used alongside owner on features, milestones, deliverables, and releases"
  aliases    ["collaborators", "participants"]
  tags       ["ownership", "planning"]
}

term effort "Effort" {
  definition "A t-shirt size (xs, s, m, l, xl) indicating the relative effort required to complete a feature. Used for weighted milestone completion calculations with Fibonacci-inspired weights: xs=1, s=2, m=3, l=5, xl=8."
  context    "Assigned to features; used by queryWeightedMilestoneCompletion"
  aliases    ["size", "story_points", "complexity"]
  see_also   [completion_ratio]
  tags       ["estimation", "planning"]
}

term release_entity "Release" {
  definition "A coordination entity grouping deliverables and milestones for simultaneous shipping. Provides version, lifecycle status, release date, and changelog. Answers 'what ships together?' structurally."
  context    "9th product entity kind; tracks coordinated multi-deliverable releases"
  aliases    ["ship"]
  see_also   [deliverable_status_lifecycle, milestone_status_lifecycle]
  tags       ["release", "coordination", "planning"]
}

term blocker "Blocker" {
  definition "A free-text description of an external dependency that prevents a milestone from progressing. Unlike depends_on (which references other milestones), blockers describe external factors: regulatory approval, third-party APIs, hiring, hardware procurement."
  context    "Used as string list on milestones with status=blocked"
  aliases    ["impediment", "external_dependency"]
  see_also   [milestone_status_lifecycle]
  tags       ["planning", "risk"]
}

term release_status_lifecycle "Release Status Lifecycle" {
  definition "The valid status progression for release entities: planned -> in_progress -> released -> recalled. Recalled is terminal. Invalid transitions produce W094."
  context    "Declared as ReleaseStatusTransition state machine"
  aliases    []
  see_also   [feature_status_lifecycle, milestone_status_lifecycle, deliverable_status_lifecycle]
  tags       ["lifecycle", "governance"]
}

// ---------------------------------------------------------------------------
// Infrastructure & protocol terms
// ---------------------------------------------------------------------------

term t_mcp "Model Context Protocol" {
  definition "The JSON-RPC over stdio protocol used by AI agents to interact with SpecForge. Provides tools, resources, and prompts for graph queries, entity inspection, and project management."
  context    "SpecForge exposes MCP tools (auto-promoted from CLI commands) and MCP resources (read-only graph views). Extensions declare surface contributions that map to mcp__{name} Wasm exports."
  aliases    ["model_context_protocol"]
  see_also   [surface_contribution, lsp, graph_protocol]
  tags       ["core", "protocol"]
}

term lsp "Language Server Protocol" {
  definition "The protocol used by IDEs and editors to interact with the SpecForge language server. Provides diagnostics, hover info, go-to-definition, completions, and code actions for .spec files."
  context    "Implemented via tower-lsp in the specforge-lsp crate. Runs as a long-lived process with warm Wasm engines for low-latency responses."
  aliases    ["language_server_protocol", "language_server"]
  see_also   [watch_mode]
  tags       ["core", "protocol"]
}

term token_budget "Token Budget" {
  definition "The maximum number of tokens an AI agent can consume in a single interaction. SpecForge optimizes output formats (brief, context, graph) to stay within token budgets while maximizing information density."
  context    "Multi-resolution graph queries expose the entity graph at multiple zoom levels. specforge export --format=brief produces the most compact output; --format=graph produces the most detailed."
  aliases    ["context_window", "token_limit"]
  see_also   [graph_protocol, t_mcp]
  tags       ["agent", "performance"]
}

term define_block "Define Block" {
  definition "A meta-block in the .spec DSL that allows users to declare custom types beyond what extensions provide. Uses the syntax define { ... } to extend the type system without requiring extension authorship."
  context    "Parsed by the core compiler as a structural construct. Define blocks enable project-specific vocabulary while keeping extensions focused on reusable domain concepts."
  aliases    ["define", "meta_block"]
  see_also   [declarative_validation, planning_entity, token_budget]
  tags       ["core", "dsl"]
}

term fuzzy_matching "Fuzzy Matching" {
  definition "Approximate string matching used in diagnostic suggestions (did you mean X?) and the specforge search command. Powered by edit-distance algorithms to find entities with similar names."
  context    "Implemented via the strsim crate. Fuzzy matching improves developer experience by suggesting corrections for typos in entity references, field names, and keyword usage."
  aliases    ["approximate_matching", "edit_distance"]
  see_also   [define_block]
  tags       ["core", "developer_experience"]
}

term watch_mode "Watch Mode" {
  definition "Incremental recompilation triggered by file system changes. The compiler watches spec files and rebuilds only affected entities when changes are detected, enabling real-time feedback in development."
  context    "Implemented via the notify crate. Watch mode keeps the graph hot in memory for LSP and MCP server modes, invalidating the query cache on each rebuild."
  aliases    ["file_watching", "incremental_mode"]
  see_also   [query_cache]
  tags       ["core", "performance"]
}
