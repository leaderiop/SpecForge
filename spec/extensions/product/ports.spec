// @specforge/product extension ports — integration boundaries
//
// Inbound ports define the query and registration interfaces the product
// extension exposes to the compiler and consumers (CLI, MCP, agents).
// Outbound ports define what the product extension requires from
// the compiler core (KindRegistry, FieldRegistry, graph queries).

use "extensions/product/types"
use "types/zero-entity-core"

port ProductQueryPort {
  direction inbound
  category  "api/product-queries"

  method queryMilestoneCompletion(milestoneId: EntityId) -> Result<MilestoneCompletionPayload, ProductQueryError>
  method queryDeliverableTraceability(deliverableId: EntityId) -> Result<DeliverableTraceabilityPayload, ProductQueryError>
  method queryJourneyCoverage(journeyId: EntityId) -> Result<JourneyCoveragePayload, ProductQueryError>
  method queryFeatureOrdering() -> Result<FeatureOrderingPayload, ProductQueryError>
  method queryMilestoneTimeline(asOfDate?: string) -> Result<MilestoneTimelinePayload, ProductQueryError>
  method queryFeatureDeliverables(featureId: EntityId) -> Result<FeatureDeliverablePayload, ProductQueryError>
  method queryFeatureMilestones(featureId: EntityId) -> Result<FeatureMilestonePayload, ProductQueryError>
  method queryPersonaJourneys(personaId: EntityId) -> Result<PersonaJourneyPayload, ProductQueryError>
  method queryChannelJourneys(channelId: EntityId) -> Result<ChannelJourneyPayload, ProductQueryError>
  method queryModuleDeliverables(moduleId: EntityId) -> Result<ModuleDeliverablePayload, ProductQueryError>
  method queryMilestoneDeliverables(milestoneId: EntityId) -> Result<MilestoneDeliverablePayload, ProductQueryError>
  method queryModuleFeatures(moduleId: EntityId) -> Result<ModuleFeaturePayload, ProductQueryError>
  // maxHops defaults to 1 when omitted; values > 5 are clamped to 5
  method queryTermGraph(termId: EntityId, maxHops?: integer) -> Result<TermGraphPayload, ProductQueryError>
  method queryDeliverableCompletion(deliverableId: EntityId) -> Result<DeliverableCompletionPayload, ProductQueryError>
  method queryPersonaChannels(personaId: EntityId) -> Result<PersonaChannelPayload, ProductQueryError>
  method queryJourneyDeliverables(journeyId: EntityId) -> Result<JourneyDeliverablePayload, ProductQueryError>
  method queryFeatureDependents(featureId: EntityId) -> Result<FeatureDependentPayload, ProductQueryError>
  method queryDeliverableDependents(deliverableId: EntityId) -> Result<DeliverableDependentPayload, ProductQueryError>
  method queryDeliverablePriority(deliverableId: EntityId) -> Result<DeliverablePriorityPayload, ProductQueryError>
  method queryPersonaFeatures(personaId: EntityId) -> Result<PersonaFeaturePayload, ProductQueryError>
  method queryFeatureImpact(featureId: EntityId) -> Result<FeatureImpactPayload, ProductQueryError>
  method queryMilestoneVelocity(milestoneId: EntityId, asOfDate?: string) -> Result<MilestoneVelocityPayload, ProductQueryError>
  method queryDeliverablePersonas(deliverableId: EntityId) -> Result<DeliverablePersonaPayload, ProductQueryError>
  method queryUnscheduledFeatures() -> Result<UnscheduledFeaturesPayload, ProductQueryError>
  method queryFeatureOverlap(pagination?: PaginatedQueryInput) -> Result<FeatureOverlapPayload, ProductQueryError>
  method queryPersonaCoverageMatrix(pagination?: PaginatedQueryInput) -> Result<PersonaCoverageMatrixPayload, ProductQueryError>
  method queryCriticalPath() -> Result<CriticalPathPayload, ProductQueryError>
  // v1.1 methods — ownership, effort, release
  method queryOwnerWorkload(pagination?: PaginatedQueryInput) -> Result<OwnerWorkloadPayload, ProductQueryError>
  method queryWeightedMilestoneCompletion(milestoneId: EntityId) -> Result<WeightedMilestoneCompletionPayload, ProductQueryError>
  method queryReleaseDeliverables(releaseId: EntityId) -> Result<ReleaseDeliverablePayload, ProductQueryError>
  method queryReleaseMilestones(releaseId: EntityId) -> Result<ReleaseMilestonePayload, ProductQueryError>
  method queryReleaseCompletion(releaseId: EntityId) -> Result<ReleaseCompletionPayload, ProductQueryError>
  method queryChannelFeatures(channelId: EntityId) -> Result<ChannelFeaturePayload, ProductQueryError>
  // Term analytics — global views over the TermSeeAlso subgraph
  method queryTermClusters() -> Result<TermClusterPayload, ProductQueryError>
  method queryTermDensity() -> Result<TermDensityPayload, ProductQueryError>
  // Module analytics — dependency structure metrics
  method queryModuleDependencyDepth(moduleId: EntityId) -> Result<ModuleDependencyDepthPayload, ProductQueryError>
  method queryModuleCoupling(pagination?: PaginatedQueryInput) -> Result<ModuleCouplingPayload, ProductQueryError>
  // Channel analytics — symmetric counterpart to queryPersonaCoverageMatrix
  method queryChannelCoverageMatrix(pagination?: PaginatedQueryInput) -> Result<ChannelCoverageMatrixPayload, ProductQueryError>

  verify unit "ProductQueryPort"
}

port ProductValidationPort {
  direction inbound
  category  "api/product-validation"

  method validateProductEntities() -> Result<ProductValidationPayload, ProductValidationError>

  requires {
    registries_populated "KindRegistry and FieldRegistry contain all 9 product entity kinds"
  }
  ensures {
    all_rules_executed   "all E007-E009, E015-E016, W041-W046, W049, W057, W075-W095, I010, I046-I097 rules are evaluated (I058 excluded — query-time only)"
    deterministic        "same graph input + same cache file always produces same diagnostic set"
    no_time_dependency   "validation never depends on wall-clock time — I058 overdue detection is query-time only"
    profile_respected    "I-codes emitted only when diagnostic profile is pedantic"
    cache_aware          "W087-W091/W094 transition rules require build cache; suppressed when absent"
  }

  verify unit "ProductValidationPort"
}

port ProductRegistrationPort {
  direction inbound
  category  "api/product-registration"

  method registerEntityKinds() -> Result<ProductEntityRegistrationPayload, RegistrationError>
  method registerEdgeTypes() -> Result<void, RegistrationError>
  method registerFieldDefinitions() -> Result<void, RegistrationError>
  method registerValidationRules() -> Result<void, RegistrationError>

  requires {
    manifest_valid "ManifestV2 has been parsed and schema-validated"
  }
  ensures {
    nine_kinds     "KindRegistry contains exactly 9 product entity kinds"
    sixteen_edges  "EdgeTypeSet contains exactly 16 product edge types"
  }

  verify unit "ProductRegistrationPort"
}

port KindRegistryPort {
  direction outbound
  category  "spi/compiler-core"

  method registerKind(kind: ManifestEntityKind) -> Result<void, RegistrationError>
  method lookupKind(name: string) -> Result<ManifestEntityKind, ProductQueryError>
  method hasKind(name: string) -> Result<boolean, never>

  verify unit "KindRegistryPort"
}

port FieldRegistryPort {
  direction outbound
  category  "spi/compiler-core"

  method registerField(kindName: string, field: ManifestField) -> Result<void, RegistrationError>
  method lookupFields(kindName: string) -> Result<ManifestField[], ProductQueryError>

  verify unit "FieldRegistryPort"
}

port EdgeTypeRegistryPort {
  direction outbound
  category  "spi/compiler-core"

  method registerEdgeType(edge: ManifestEdgeType) -> Result<void, RegistrationError>
  method lookupEdgesForKind(kindName: string) -> Result<ManifestEdgeType[], ProductQueryError>

  verify unit "EdgeTypeRegistryPort"
}

port GraphQueryPort {
  direction outbound
  category  "spi/compiler-core"

  method getIncomingEdges(nodeId: EntityId, edgeType: string) -> Result<EntityId[], ProductQueryError>
  method getOutgoingEdges(nodeId: EntityId, edgeType: string) -> Result<EntityId[], ProductQueryError>
  method getNodesByKind(kind: string) -> Result<EntityId[], ProductQueryError>
  method detectCycles(edgeType: string) -> Result<EntityId[][], never>

  requires {
    graph_built "in-memory graph has been constructed by the resolver"
  }

  verify unit "GraphQueryPort"
}
