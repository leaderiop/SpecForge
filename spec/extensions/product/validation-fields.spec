// Field-level validation rules — enum values, date formats, references, version formats
//
// Split from validation-rules.spec. These behaviors describe field-level
// checks declared as ValidationRulePattern entries in the @specforge/product
// manifest. The core declarative validation engine executes these patterns.
//
// Naming convention: validate_* — field-level checks (enum values, date formats, references)

use "types/diagnostics"
use "types/graph"

behavior validate_feature_status_field "Validate Feature Status Field" {
  category validation
  types    [ProductFeature, FeatureStatus, Diagnostic]
  contract """
    The @specforge/product extension MUST validate that when a feature's
    status field is present, its value is a valid FeatureStatus enum value
    (proposed, accepted, in_progress, done, deferred). Invalid status
    values MUST produce a W077 warning.
  """
  ensures  {
    valid_status   "feature with invalid status value produces W077"
    absent_passes  "feature without status field produces no W077"
  }

  features [pe_validation_suite]

  verify unit "feature with valid status passes"
  verify unit "feature with invalid status produces diagnostic"
}

behavior validate_feature_priority_field "Validate Feature Priority Field" {
  category validation
  types    [ProductFeature, Priority, Diagnostic]
  contract """
    The @specforge/product extension MUST validate that when a feature's
    priority field is present, its value is a valid Priority enum value
    (critical, high, medium, low). Invalid priority values MUST produce
    a W078 warning. W078 is shared across all entity kinds that use Priority.
  """
  ensures  {
    valid_priority "feature with invalid priority value produces W078"
    absent_passes  "feature without priority field produces no W078"
  }

  features [pe_validation_suite]

  verify unit "feature with valid priority passes"
  verify unit "feature with invalid priority produces diagnostic"
}

behavior validate_journey_priority_field "Validate Journey Priority Field" {
  category validation
  types    [ProductJourney, Priority, Diagnostic]
  contract """
    The @specforge/product extension MUST validate that when a journey's
    priority field is present, its value is a valid Priority enum value
    (critical, high, medium, low). Invalid priority values MUST produce
    a W078 warning. W078 is shared across all entity kinds that use Priority.
  """
  ensures  {
    valid_priority "journey with invalid priority value produces W078"
    absent_passes  "journey without priority field produces no W078"
  }

  features [pe_validation_suite]

  verify unit "journey with valid priority passes"
  verify unit "journey with invalid priority produces diagnostic"
}

behavior validate_milestone_priority_field "Validate Milestone Priority Field" {
  category validation
  types    [ProductMilestone, Priority, Diagnostic]
  contract """
    The @specforge/product extension MUST validate that when a milestone's
    priority field is present, its value is a valid Priority enum value
    (critical, high, medium, low). Invalid priority values MUST produce
    a W078 warning. W078 is shared across all entity kinds that use Priority.
  """
  ensures  {
    valid_priority "milestone with invalid priority value produces W078"
    absent_passes  "milestone without priority field produces no W078"
  }

  features [pe_validation_suite]

  verify unit "milestone with valid priority passes"
  verify unit "milestone with invalid priority produces diagnostic"
}

behavior validate_deliverable_artifact_type_field "Validate Deliverable Artifact Type Field" {
  category validation
  types    [ProductDeliverable, ArtifactType, Diagnostic]
  contract """
    The @specforge/product extension MUST validate that when a deliverable's
    artifact_type field is present, its value is a valid ArtifactType enum
    value (cli, service, library, web_app, mobile_app, api, extension,
    documentation, package). Invalid artifact_type values MUST produce
    a W080 warning.
  """
  ensures  {
    valid_artifact_type "deliverable with invalid artifact_type value produces W080"
    absent_passes       "deliverable without artifact_type field produces no W080"
  }

  features [pe_validation_suite]

  verify unit "deliverable with valid artifact_type passes"
  verify unit "deliverable with invalid artifact_type produces diagnostic"
}

behavior validate_persona_status_field "Validate Persona Status Field" {
  category validation
  types    [ProductPersona, PersonaStatus, Diagnostic]
  contract """
    The @specforge/product extension MUST validate that when a persona's
    status field is present, its value is a valid PersonaStatus enum value
    (active, deprecated). Invalid status values MUST produce a W083 warning.
  """
  ensures  {
    valid_status   "persona with invalid status value produces W083"
    absent_passes  "persona without status field produces no W083"
  }

  features [pe_validation_suite]

  verify unit "persona with valid status passes"
  verify unit "persona with invalid status produces diagnostic"
}

behavior validate_channel_status_field "Validate Channel Status Field" {
  category validation
  types    [ProductChannel, ChannelStatus, Diagnostic]
  contract """
    The @specforge/product extension MUST validate that when a channel's
    status field is present, its value is a valid ChannelStatus enum value
    (active, deprecated). Invalid status values MUST produce a W084 warning.
  """
  ensures  {
    valid_status   "channel with invalid status value produces W084"
    absent_passes  "channel without status field produces no W084"
  }

  features [pe_validation_suite]

  verify unit "channel with valid status passes"
  verify unit "channel with invalid status produces diagnostic"
}

behavior validate_deliverable_status_field "Validate Deliverable Status Field" {
  category validation
  types    [ProductDeliverable, DeliverableStatus, Diagnostic]
  contract """
    The @specforge/product extension MUST validate that when a deliverable's
    status field is present, its value is a valid DeliverableStatus enum value
    (draft, in_progress, shipped, deprecated). Invalid status values MUST
    produce a W085 warning.
  """
  ensures  {
    valid_status   "deliverable with invalid status value produces W085"
    absent_passes  "deliverable without status field produces no W085"
  }

  features [pe_validation_suite]

  verify unit "deliverable with valid status passes"
  verify unit "deliverable with invalid status produces diagnostic"
}

behavior validate_module_family_field "Validate Module Family Field" {
  category validation
  types    [Diagnostic, ProductModule, ModuleFamily]
  contract """
    The @specforge/product extension SHOULD validate that when a module's
    family field is present, its value is one of the ModuleFamily enum
    members: core, platform, extension, integration, advisory. Non-standard
    family values SHOULD produce an I062 info diagnostic with a fuzzy-match
    suggestion from the ModuleFamily enum. Info-level allows custom family
    values while suggesting standardization. The ModuleFamily enum is an
    open set — custom values are valid but produce informational notices.

    Enum semantics:
      core        — fundamental library or framework component
      platform    — user-facing binary (CLI, LSP, MCP server)
      extension   — plugin or extension package
      integration — integration adapter or bridge
      advisory    — non-code module (docs, process, governance)
  """
  ensures  {
    fires_non_standard   "module with family value not in ModuleFamily enum produces I062"
    suppresses_standard  "module with family value in ModuleFamily enum suppresses I062"
    absent_passes        "module with no family produces no I062"
    suggests_match       "I062 includes fuzzy-match suggestion from ModuleFamily enum"
  }

  features [pe_validation_suite]

  verify unit "module with family=core passes"
  verify unit "module with family=integration passes"
  verify unit "module with family=advisory passes"
  verify unit "module with non-standard family produces I062 with suggestion"
  verify unit "module with no family produces no I062"
}

behavior validate_milestone_target_date_format "Validate Milestone Target Date Format" {
  category validation
  types    [Diagnostic, ProductMilestone]
  contract """
    The @specforge/product extension SHOULD validate that when a milestone's
    target_date field is present, its value matches the ISO 8601 date format
    YYYY-MM-DD. Invalid formats SHOULD produce an I053 info diagnostic.
    Uses a field_value_constraint validation pattern with regex
    ^\d{4}-\d{2}-\d{2}$. Info-level respects incremental adoption —
    projects may use free-form dates before standardizing.
  """
  ensures  {
    fires_invalid_format "milestone with non-YYYY-MM-DD target_date produces I053"
    suppresses_valid     "milestone with valid YYYY-MM-DD target_date suppresses I053"
    absent_passes        "milestone with no target_date produces no I053"
  }

  features [pe_validation_suite]

  verify unit "milestone with valid YYYY-MM-DD target_date passes"
  verify unit "milestone with invalid target_date format produces I053"
  verify unit "milestone with no target_date produces no I053"
}

behavior validate_deliverable_version_format "Validate Deliverable Version Format" {
  category validation
  types    [Diagnostic, ProductDeliverable]
  contract """
    The @specforge/product extension SHOULD validate that when a deliverable's
    version field is present, its value conforms to Semantic Versioning 2.0.0
    (https://semver.org). The full regex is:
      ^\d+\.\d+\.\d+(-[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*)?(\+[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*)?$
    This accepts:
      - Core versions: 1.0.0, 0.1.0, 2.3.4
      - Pre-release tags: 1.0.0-alpha, 1.0.0-alpha.1, 1.0.0-0.3.7
      - Build metadata: 1.0.0+build.42, 1.0.0-beta+exp.sha.5114f85
    Invalid formats (e.g., "v1.0", "1.0", "latest") SHOULD produce an I061
    info diagnostic. Info-level respects incremental adoption — projects may
    use free-form versions before standardizing.
  """
  ensures  {
    fires_invalid_format   "deliverable with non-semver version produces I061"
    suppresses_valid       "deliverable with valid semver version suppresses I061"
    absent_passes          "deliverable with no version produces no I061"
    accepts_prerelease     "deliverable with pre-release tag (e.g., 1.0.0-alpha.1) suppresses I061"
    accepts_build_metadata "deliverable with build metadata (e.g., 1.0.0+build.42) suppresses I061"
  }

  features [pe_validation_suite]

  verify unit "deliverable with valid semver version passes"
  verify unit "deliverable with invalid version format produces I061"
  verify unit "deliverable with no version produces no I061"
  verify unit "deliverable with pre-release tag passes"
  verify unit "deliverable with build metadata passes"
  verify unit "deliverable with version 'v1.0' produces I061"
}

behavior validate_tag_format "Validate Tag Format" {
  category validation
  types    [Diagnostic]
  contract """
    The @specforge/product extension SHOULD validate that tags on all 9 entity
    kinds follow the naming convention: lowercase, hyphen-separated, matching
    the pattern [a-z0-9][a-z0-9-]*[a-z0-9] with minimum length 2 and maximum
    length 50. Tags violating this pattern SHOULD produce an I068 info diagnostic
    with a suggested normalized form (lowercased, spaces/underscores replaced
    with hyphens, trimmed to 50 chars). Empty strings in tags arrays are silently
    ignored without diagnostics.
  """
  ensures  {
    fires_on_uppercase         "tag with uppercase characters produces I068 with lowercased suggestion"
    fires_on_spaces            "tag with spaces produces I068 with hyphenated suggestion"
    fires_on_underscores       "tag with underscores produces I068 with hyphenated suggestion"
    fires_on_single_char       "single-character tag produces I068"
    fires_on_too_long          "tag exceeding 50 characters produces I068 with truncated suggestion"
    suppresses_valid           "tag matching [a-z0-9][a-z0-9-]*[a-z0-9] suppresses I068"
    ignores_empty              "empty string in tags array is silently ignored"
    fires_on_special_chars     "tag with special characters (!, @, #, etc.) produces I068"
  }

  features [pe_validation_suite]

  verify unit "lowercase hyphen-separated tag passes"
  verify unit "uppercase tag produces I068 with lowercase suggestion"
  verify unit "tag with spaces produces I068 with hyphenated suggestion"
  verify unit "single-character tag produces I068"
  verify unit "tag exceeding 50 chars produces I068"
  verify unit "empty string in tags array is silently ignored"
}

behavior validate_journey_flow_non_empty "Validate Journey Flow Non-Empty" {
  category validation
  types    [ProductJourney, Diagnostic]
  contract """
    The @specforge/product extension SHOULD detect journeys with an empty
    flow field. Journeys without flow steps SHOULD produce an I050 info
    diagnostic. A journey with at least one flow step is valid.
  """
  ensures  {
    fires_when_empty     "journey with empty flow produces I050"
    suppresses_non_empty "journey with at least one flow step suppresses I050"
  }

  features [pe_validation_suite]

  verify unit "journey with empty flow produces I050"
  verify unit "journey with flow steps suppresses I050"
}

behavior validate_channel_references "Validate Channel References" {
  category   validation
  types      [Diagnostic]
  contract   """
    The @specforge/product extension MUST validate that all channel
    references in entity fields resolve to channel entity kinds declared
    in the project. References to undeclared channels MUST produce an
    E009 diagnostic. Valid channel references MUST pass without
    diagnostics.
  """
  ensures    {
    fires_when_missing "reference to undeclared channel produces E009"
    suppresses_valid   "reference to declared channel produces no diagnostic"
  }

  features [pe_validation_suite]

  verify unit "references to undeclared channels produce E009"
  verify unit "valid channel references pass without diagnostics"
}

behavior detect_term_see_also_non_term_refs "Detect Term See-Also Non-Term References" {
  category validation
  types    [Diagnostic, ProductTerm]
  contract """
    The @specforge/product extension SHOULD detect when a term's see_also
    field references entities that are NOT term entities. Per decision
    pe_term_see_also_term_only, only term-to-term references create
    TermSeeAlso graph edges; cross-kind references pass E001 resolution
    but are documentation-only and produce no edges. This behavior SHOULD
    produce an I056 info diagnostic for each non-term reference in
    see_also to inform the user that no graph edge was created.
  """
  ensures  {
    fires_non_term       "see_also reference to a non-term entity produces I056"
    suppresses_term      "see_also reference to another term produces no I056"
    empty_passes         "term with empty see_also produces no I056"
  }

  features [pe_validation_suite]

  verify unit "term see_also referencing another term produces no I056"
  verify unit "term see_also referencing a module produces I056"
  verify unit "term see_also referencing a feature produces I056"
  verify unit "term with empty see_also produces no I056"
}

behavior detect_term_alias_conflicts "Detect Term Alias Conflicts" {
  category validation
  types    [Diagnostic]
  contract """
    The @specforge/product extension SHOULD detect when a term's alias
    conflicts with another term's alias or another term's ID. When two
    terms share an alias, or a term's alias matches another term's entity
    ID, it SHOULD produce a W086 warning identifying both terms and the
    conflicting alias. This prevents vocabulary ambiguity where the same
    word resolves to different definitions depending on context.
  """
  ensures  {
    fires_alias_alias_conflict   "two terms sharing the same alias produce W086"
    fires_alias_id_conflict      "term alias matching another term's ID produces W086"
    case_insensitive             "comparison is case-insensitive to catch 'API' vs 'api'"
    identifies_both_terms        "W086 names both conflicting terms and the shared alias"
    suppresses_no_conflict       "terms with unique aliases suppress W086"
  }

  features [pe_validation_suite]

  verify unit "two terms with same alias produce W086"
  verify unit "term alias matching another term ID produces W086"
  verify unit "case-insensitive match produces W086"
  verify unit "terms with distinct aliases suppress W086"
}

behavior detect_journey_deprecated_persona "Detect Journey Referencing Deprecated Persona" {
  category validation
  types    [Diagnostic, ProductJourney, ProductPersona, PersonaStatus]
  contract """
    The @specforge/product extension SHOULD detect journeys that reference
    a persona with status=deprecated. Journeys referencing deprecated
    personas SHOULD produce a W075 warning identifying both the journey
    ID and the deprecated persona ID.
  """
  ensures  {
    deprecated_detected    "journey referencing deprecated persona produces W075"
    active_suppressed      "journey referencing active persona suppresses W075"
    absent_suppressed      "journey with no persona reference suppresses W075"
    missing_status_active  "persona without status field is treated as active (no W075)"
  }

  features [pe_validation_suite]

  verify unit "journey referencing deprecated persona produces W075"
  verify unit "journey referencing active persona suppresses W075"
  verify unit "journey with no persona suppresses W075"
  verify unit "persona without status field treated as active"
}

behavior detect_journey_deprecated_channels "Detect Journey Referencing Deprecated Channels" {
  category validation
  types    [Diagnostic, ProductJourney, ProductChannel, ChannelStatus]
  contract """
    The @specforge/product extension SHOULD detect journeys that reference
    channels with status=deprecated. Each deprecated channel reference
    SHOULD produce a W076 warning identifying both the journey ID and
    the deprecated channel ID.
  """
  ensures  {
    deprecated_detected    "journey referencing deprecated channel produces W076"
    active_suppressed      "journey referencing active channel suppresses W076"
    absent_suppressed      "journey with no channel references suppresses W076"
    missing_status_active  "channel without status field is treated as active (no W076)"
    per_channel_diagnostic "each deprecated channel in a journey produces a separate W076"
  }

  features [pe_validation_suite]

  verify unit "journey referencing deprecated channel produces W076"
  verify unit "journey referencing active channel suppresses W076"
  verify unit "journey with no channels suppresses W076"
  verify unit "channel without status field treated as active"
  verify unit "journey with two deprecated channels produces two W076"
}

behavior validate_milestone_status_field "Validate Milestone Status Field" {
  category validation
  types    [ProductMilestone, MilestoneStatus, Diagnostic]
  contract """
    The @specforge/product extension MUST validate that when a milestone's
    status field is present, its value is a valid MilestoneStatus enum value
    (planned, in_progress, completed, blocked). Invalid status values MUST
    produce a W079 warning.
  """
  ensures  {
    valid_status   "milestone with invalid status value produces W079"
    absent_passes  "milestone without status field produces no W079"
  }

  features [pe_validation_suite]

  verify unit "milestone with valid status passes"
  verify unit "milestone with invalid status produces diagnostic"
}

behavior validate_effort_field "Validate Feature Effort Field" {
  category validation
  types    [ProductFeature, Effort, Diagnostic]
  contract """
    The @specforge/product extension MUST validate that when a feature's
    effort field is present, its value is a valid Effort enum value
    (xs, s, m, l, xl). Invalid effort values MUST produce a W095 warning.
    This ensures weighted milestone completion queries use valid effort
    weights (xs=1, s=2, m=3, l=5, xl=8).
  """
  ensures  {
    valid_effort   "feature with invalid effort value produces W095"
    absent_passes  "feature without effort field produces no W095"
  }

  features [pe_validation_suite]

  verify unit "feature with effort=xl passes"
  verify unit "feature with effort=huge produces W095"
  verify unit "feature without effort produces no W095"
}

behavior detect_priority_effort_mismatch "Detect Priority-Effort Mismatch" {
  category   validation
  invariants [pe_effort_enum_validity]
  types      [ProductFeature, Priority, Effort, Diagnostic]
  contract   """
    The @specforge/product extension SHOULD inform when a feature has a
    high or critical priority but a very small effort estimate, suggesting
    the effort may be underestimated. Specifically:
      - priority=critical with effort in [xs, s] produces I095
      - priority=high with effort=xs produces I095
    All other priority-effort combinations are valid and produce no
    diagnostic. When either priority or effort is absent, the check is
    skipped. I095 is informational — it does not block compilation and
    respects incremental adoption where effort estimates may be rough
    initial guesses.
  """
  ensures    {
    fires_critical_xs      "feature with priority=critical and effort=xs produces I095"
    fires_critical_s       "feature with priority=critical and effort=s produces I095"
    fires_high_xs          "feature with priority=high and effort=xs produces I095"
    suppresses_medium_xs   "feature with priority=medium and effort=xs produces no I095"
    suppresses_low_xs      "feature with priority=low and effort=xs produces no I095"
    suppresses_critical_m  "feature with priority=critical and effort=m produces no I095"
    suppresses_high_s      "feature with priority=high and effort=s produces no I095"
    absent_effort_skips    "feature without effort field produces no I095"
    absent_priority_skips  "feature without priority field produces no I095"
    identifies_feature     "I095 includes the feature ID and suggests reviewing effort estimate"
  }

  features [pe_validation_suite]

  verify unit "flags critical+xs combination"
  verify unit "flags critical+s combination"
  verify unit "flags high+xs combination"
  verify unit "no flag for medium+xs"
  verify unit "no flag for high+s"
  verify unit "no flag for critical+m"
  verify unit "no flag when effort absent"
  verify unit "no flag when priority absent"
}
