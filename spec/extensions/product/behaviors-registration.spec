// @specforge/product — Registration and field validation behaviors
//
// Entity kind registration, edge type registration, field definitions,
// validation rules, and field-level validation for product entities.

use "extensions/product/invariants"
use "extensions/product/types"
use "extensions/product/ports"
use "product/features"
use "types/zero-entity-core"
use "types/diagnostics"

behavior pe_register_entity_kinds "Register Product Entity Kinds" {
  category   command
  invariants [pe_feature_non_testable, pe_persona_non_testable, pe_channel_non_testable, pe_product_verify_support]
  types      [
    ManifestEntityKind,
    ProductFeature,
    ProductJourney,
    ProductDeliverable,
    ProductMilestone,
    ProductModule,
    ProductTerm,
    ProductPersona,
    ProductChannel,
    ProductRelease,
    ProductEntityKindsRegisteredPayload,
    ProductEntityRegistrationPayload,
    RegistrationError,
  ]
  produces  [pe_entity_kinds_registered]
  contract   """
    The @specforge/product extension MUST register 9 entity kinds with
    full metadata in the KindRegistry.
  """
  requires   {
    manifest_loaded        "ManifestV2 is parsed and schema-validated"
    no_duplicate_kinds     "KindRegistry has no entries with names matching this extension's kinds"
  }
  ensures    {
    journey_registered     "KindRegistry contains journey: testable=false, supportsVerify=false, semanticToken=class, lspIcon=Module, dotShape=hexagon"
    deliverable_registered "KindRegistry contains deliverable: testable=false, supportsVerify=true, semanticToken=class, lspIcon=Package, dotShape=box3d"
    milestone_registered   "KindRegistry contains milestone: testable=false, supportsVerify=true, semanticToken=class, lspIcon=Calendar, dotShape=tab"
    module_registered      "KindRegistry contains module: testable=false, supportsVerify=false, semanticToken=namespace, lspIcon=Folder, dotShape=folder"
    term_registered        "KindRegistry contains term: testable=false, supportsVerify=false, semanticToken=class, lspIcon=File, dotShape=note"
    feature_registered     "KindRegistry contains feature: testable=false, supportsVerify=true, semanticToken=class, lspIcon=Package, dotShape=hexagon"
    persona_registered     "KindRegistry contains persona: testable=false, supportsVerify=false, semanticToken=class, lspIcon=Person, dotShape=oval"
    channel_registered     "KindRegistry contains channel: testable=false, supportsVerify=false, semanticToken=class, lspIcon=SymbolInterface, dotShape=component"
    release_registered     "KindRegistry contains release: testable=false, supportsVerify=false, semanticToken=class, lspIcon=Symbol, dotShape=box"
    nine_kinds_total       "KindRegistry has exactly 9 domain entries after registration"
  }

  ports    [ProductRegistrationPort, KindRegistryPort]
  features [pe_core_entity_kinds, product_entity_registration]

  verify unit "journey registered with testable=false"
  verify unit "deliverable registered with testable=false, supportsVerify=true"
  verify unit "milestone registered with dotShape=tab"
  verify unit "module registered with lspIcon=Folder"
  verify unit "term registered with testable=false"
  verify unit "feature registered with testable=false, supportsVerify=true, dotShape=hexagon"
  verify unit "persona registered with testable=false and dotShape=oval"
  verify unit "channel registered with testable=false and dotShape=component"
  verify unit "release registered with testable=false"
}

behavior pe_register_edge_types "Register Product Edge Types" {
  category command
  types    [ManifestEdgeType, ProductEdgeTypesRegisteredPayload]
  produces  [pe_edge_types_registered]
  contract """
    The @specforge/product extension MUST register 16 edge types that
    model relationships between the 9 entity kinds.
  """
  requires {
    kinds_registered       "all 9 entity kinds are in KindRegistry"
  }
  ensures  {
    journey_feature        "EdgeTypeSet contains JourneyFeature (journey->feature)"
    deliverable_journey    "EdgeTypeSet contains DeliverableJourney (deliverable->journey)"
    module_depends_on      "EdgeTypeSet contains ModuleDependsOn (module->module)"
    milestone_feature      "EdgeTypeSet contains MilestoneFeature (milestone->feature)"
    deliverable_module     "EdgeTypeSet contains DeliverableModule (deliverable->module)"
    module_feature         "EdgeTypeSet contains ModuleFeature (module->feature)"
    feature_depends_on     "EdgeTypeSet contains FeatureDependsOn (feature->feature)"
    journey_persona        "EdgeTypeSet contains JourneyPersona (journey->persona)"
    journey_channel        "EdgeTypeSet contains JourneyChannel (journey->channel)"
    milestone_module       "EdgeTypeSet contains MilestoneModule (milestone->module)"
    term_see_also          "EdgeTypeSet contains TermSeeAlso (term->term)"
    milestone_depends_on   "EdgeTypeSet contains MilestoneDependsOn (milestone->milestone)"
    deliverable_milestone  "EdgeTypeSet contains DeliverableMilestone (deliverable->milestone)"
    deliverable_depends_on "EdgeTypeSet contains DeliverableDependsOn (deliverable->deliverable)"
    release_deliverable    "EdgeTypeSet contains ReleaseDeliverable (release->deliverable)"
    release_milestone      "EdgeTypeSet contains ReleaseMilestone (release->milestone)"
    sixteen_edges_total    "EdgeTypeSet has exactly 16 entries"
  }

  ports    [ProductRegistrationPort, EdgeTypeRegistryPort]
  features [pe_core_entity_kinds, product_entity_registration]

  verify unit "all 16 edge types registered in edge set"
  verify unit "JourneyFeature edge has sourceKind=journey and targetKind=feature"
  verify unit "ModuleDependsOn edge has sourceKind=module and targetKind=module"
  verify unit "FeatureDependsOn edge has sourceKind=feature and targetKind=feature"
  verify unit "DeliverableModule edge has sourceKind=deliverable and targetKind=module"
  verify unit "ModuleFeature edge has sourceKind=module and targetKind=feature"
  verify unit "JourneyPersona edge has sourceKind=journey and targetKind=persona"
  verify unit "JourneyChannel edge has sourceKind=journey and targetKind=channel"
  verify unit "MilestoneModule edge has sourceKind=milestone and targetKind=module"
  verify unit "TermSeeAlso edge has sourceKind=term and targetKind=term"
  verify unit "MilestoneDependsOn edge has sourceKind=milestone and targetKind=milestone"
  verify unit "DeliverableMilestone edge has sourceKind=deliverable and targetKind=milestone"
  verify unit "DeliverableDependsOn edge has sourceKind=deliverable and targetKind=deliverable"
  verify unit "ReleaseDeliverable edge has sourceKind=release and targetKind=deliverable"
  verify unit "ReleaseMilestone edge has sourceKind=release and targetKind=milestone"
}

behavior pe_register_field_definitions "Register Product Field Definitions" {
  category command
  types    [ManifestField, ManifestEntityKind, ProductFieldsRegisteredPayload]
  produces  [pe_field_definitions_registered]
  contract """
    The @specforge/product extension MUST register field definitions for
    each entity kind with name, type, edge mapping, and target kind.
  """
  requires {
    kinds_and_edges_registered "all 9 kinds and 16 edge types are registered"
  }
  ensures  {
    feature_fields         "feature has: problem(string), solution(string), priority(Priority), status(string), acceptance(string[]), depends_on(reference[]->feature, FeatureDependsOn), reason(string), tags(string[])"
    journey_fields         "journey has: persona(reference->persona, JourneyPersona), description(string), channels(reference[]->channel, JourneyChannel), features(reference[]->feature, JourneyFeature), flow(string[]), priority(Priority), tags(string[])"
    deliverable_fields     "deliverable has: artifact_type(ArtifactType), status(DeliverableStatus), journeys(reference[]->journey, DeliverableJourney), modules(reference[]->module, DeliverableModule), version(string), milestones(reference[]->milestone, DeliverableMilestone), depends_on(reference[]->deliverable, DeliverableDependsOn), reason(string), tags(string[])"
    milestone_fields       "milestone has: status(MilestoneStatus), features(reference[]->feature, MilestoneFeature), exit_criteria(string[]), target_date(string), modules(reference[]->module, MilestoneModule), depends_on(reference[]->milestone, MilestoneDependsOn), priority(Priority), reason(string), tags(string[])"
    module_fields          "module has: family(string), description(string), features(reference[]->feature, ModuleFeature), depends_on(reference[]->module, ModuleDependsOn), tags(string[])"
    term_fields            "term has: definition(string), context(string), aliases(string[]), see_also(reference[]->term, TermSeeAlso), tags(string[])"
    persona_fields         "persona has: description(string), technical_level(TechnicalLevel), goals(string[]), pain_points(string[]), status(PersonaStatus), tags(string[])"
    channel_fields         "channel has: description(string), interaction_model(InteractionModel), status(ChannelStatus), tags(string[])"
    status_defaults        "status fields declare default_value: feature.status='proposed', milestone.status='planned', deliverable.status='draft', persona.status='active', channel.status='active', release.status='planned'"
  }

  ports    [FieldRegistryPort]
  features [pe_core_entity_kinds, product_entity_registration]

  verify unit "feature problem field registered as string type"
  verify unit "feature depends_on field registered with FeatureDependsOn edge"
  verify unit "journey features field registered with JourneyFeature edge"
  verify unit "module depends_on field registered with ModuleDependsOn edge"
  verify unit "deliverable modules field registered with DeliverableModule edge"
  verify unit "module features field registered with ModuleFeature edge"
  verify unit "milestone modules field registered with MilestoneModule edge"
  verify unit "milestone depends_on field registered with MilestoneDependsOn edge"
  verify unit "term see_also field registered with TermSeeAlso edge"
  verify unit "deliverable milestones field registered with DeliverableMilestone edge"
  verify unit "deliverable depends_on field registered with DeliverableDependsOn edge"
  verify unit "persona pain_points field registered as string[] type"
}

behavior pe_register_validation_rules "Register Product Validation Rules" {
  category command
  types    [ValidationRulePattern, ValidationPatternKind, ProductValidationError]
  contract """
    The @specforge/product extension MUST register declarative validation
    rules in its manifest.
  """
  requires {
    field_definitions_registered "all field definitions for 9 kinds and 16 edge types are in FieldRegistry"
  }
  ensures  {
    rules_registered       "all diagnostic codes (E007-E009, E015-E016, W041-W046, W049, W057, W075-W095, I010, I046-I097) are registered as declarative validation rules"
    rules_sorted           "rules are sorted by diagnostic code for deterministic execution"
    rules_count            "69 rules total: field validation (W077-W086, W095, I050, I053, I056, I061, I062, I068, I095, E008, E009), structural (W041-W046, W049, W057, I010, I046-I052, I067, I071, I072, I075, I096, W075, W076, W086), lifecycle (W087-W094, I054-I060, I063-I066, I069-I070, I073-I079, I080-I091, I092-I094, I097, W092-W093), DAG (E007, E015, E016, W045)"
  }

  ports    [ProductValidationPort]
  features [pe_core_entity_kinds, pe_validation_suite, product_validation]

  verify unit "validation rules registered from manifest"
  verify unit "rules include E007-E009, E015-E016, W041-W046, W049, W057, W075-W095, I010, I046-I097"
  verify unit "rules sorted by diagnostic code"
}

behavior pe_validate_persona_fields "Validate Persona Fields" {
  invariants [persona_channel_lifecycle]
  category   validation
  types      [ProductPersona, TechnicalLevel, Diagnostic]
  produces  [pe_query_failed]
  contract   """
    The @specforge/product extension MUST validate that persona entities
    have a non-empty description field and that technical_level, when
    present, is a valid TechnicalLevel enum value. Invalid technical_level
    values MUST produce a W081 warning.
  """
  ensures    {
    description_required   "persona without description produces a diagnostic"
    technical_level_valid  "persona with invalid technical_level produces W081"
  }

  features [pe_validation_suite]

  verify unit "persona with valid fields passes"
  verify unit "persona with invalid technical_level produces diagnostic"
}

behavior pe_validate_channel_fields "Validate Channel Fields" {
  category   validation
  types      [ProductChannel, InteractionModel, Diagnostic, ProductValidationPayload]
  produces  [pe_validation_complete]
  contract   """
    The @specforge/product extension MUST validate that channel entities
    have a non-empty description field and that interaction_model is a
    valid InteractionModel enum value. Invalid interaction_model values
    MUST produce a W082 warning.
  """
  ensures    {
    description_required      "channel without description produces a diagnostic"
    interaction_model_valid   "channel with invalid interaction_model produces W082"
  }

  features [pe_validation_suite]

  verify unit "channel with valid fields passes"
  verify unit "channel with invalid interaction_model produces diagnostic"
}

behavior pe_validate_deliverable_completeness "Validate Deliverable Completeness" {
  invariants [pe_validation_deterministic]
  category   validation
  types      [ProductDeliverable, Diagnostic, ProductDiagnosticCounts]
  produces  [pe_validation_summary]
  contract   """
    The @specforge/product extension MUST validate deliverable completeness
    by checking both journeys (W043) and modules (W046). A deliverable
    with neither journeys nor modules is structurally empty.
  """
  ensures    {
    journeys_checked   "deliverable with no journeys produces W043"
    modules_checked    "deliverable with no modules produces W046"
  }

  features [pe_validation_suite]

  verify unit "deliverable with journeys and modules passes both checks"
  verify unit "deliverable with no journeys produces W043"
  verify unit "deliverable with no modules produces W046"
}

behavior pe_validate_milestone_status "Validate Milestone Status Consistency" {
  invariants [milestone_status_consistency]
  category   validation
  types      [ProductMilestone, MilestoneStatus, Diagnostic, ProductEntityDiagnostic]
  produces  [pe_validation_rule_fired]
  contract   """
    The @specforge/product extension MUST validate milestone status consistency.
    This behavior orchestrates three underlying validation rules:
    validate_milestone_status_field (W079 for invalid enum values),
    detect_completed_milestone_without_criteria (W057 for completed without exit_criteria),
    detect_blocked_milestone_without_dependency (I057 for blocked without depends_on).
  """
  ensures    {
    delegates_to_rules  "milestone status validation delegates to three individual validation rules"
    all_three_executed  "W079, W057, and I057 validation rules are all executed during milestone validation"
  }

  features [pe_validation_suite]

  verify unit "milestone status validation runs all three sub-rules"
}
