requirement INVARIANT-092 "Nine Entity Kinds" {
  category "Invariant"
  tier     v2
  text     """
    The @specforge/product manifest MUST declare exactly 9 entity kinds:
    journey, deliverable, milestone, module, term, feature, persona,
    channel, release. Feature is a domain-neutral product concept. Persona
    and channel are first-class entity kinds. Release coordinates
    multi-deliverable shipping.
  """
}

requirement INVARIANT-093 "Twenty Edge Types" {
  category "Invariant"
  tier     v2
  text     """
    The @specforge/product manifest MUST declare exactly 20 edge types:
    FeatureDependsOn, FeatureRelatesTo, JourneyExercisesFeature,
    JourneyTargetsPersona, JourneyUsesChannel, DeliverableSupportsJourney,
    DeliverableContainsModule, DeliverableTrackedByMilestone,
    DeliverableDependsOn, MilestoneDeliversFeature, MilestoneScopesModule,
    MilestoneDependsOn, ModuleContainsFeature, ModuleDependsOn,
    TermReferencesRelatedTerm, TermBelongsToModule,
    ReleaseIncludesDeliverable, ReleaseCompletesMilestone, ReleaseDependsOn,
    PersonaPrioritizesFeature. These edges model relationships between the
    9 entity kinds.
  """
}
