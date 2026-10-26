// @specforge/software extension manifest declaration

use "extensions/software/types"
use "types/zero-entity-core"

behavior se_declare_manifest "Declare @specforge/software Manifest" {
  features [se_core_entity_kinds]
  category command
  types    [ExtensionDeclaration, EntityKindDescriptor, EdgeTypeDescriptor]
  contract """
    The @specforge/software extension MUST declare itself with name
    "@specforge/software". Its declaration MUST declare
    exactly 4 entity kinds (behavior, event, type, port), its 15 edge types
    (References, BehaviorImplementsFeature, BehaviorProducesEvent,
    BehaviorConsumesEvent, BehaviorReferencesType, EventCarriesPayloadType,
    TypeComposesType, PortReferencesType, BehaviorUsesPort,
    BehaviorEnforcesInvariant, TypeExtendsType, ExternalRef,
    MilestoneIncludesBehavior, ModuleConsumesPort, ModuleDefinesPort), and all
    associated validation rules. `invariant` is governance's (ADR 0280). The
    compiled Wasm component MUST serve it.
  """
  requires {
    supported_protocol   "the handshake's protocol major version is the host's"
    valid_extension_name "name == '@specforge/software'"
    wasm_module_exists   "the declaration is read from the compiled Wasm component"
  }
  ensures {
    four_entity_kinds   "entityKinds.length == 4"
    fifteen_edge_types  "edgeTypes.length == 15"
    all_kinds_named     "entityKinds contains behavior, event, type, port"
    declared_categories "the declaration declares entities, edges, enhancements and validation_rules"
    optional_peers      "peer_dependencies holds two optional peers: @specforge/product, whose kinds the feature/module/milestone links target, and @specforge/governance, whose invariant a behavior's invariants target"
  }
  verify unit "manifest name is @specforge/software"
  verify unit "manifest declares exactly 4 entity kinds"
  verify unit "manifest declares exactly 15 edge types"
  verify unit "the handshake's protocol major is the host's"
  verify unit "the declaration declares entities, edges, enhancements and validation_rules"
  verify unit "product and governance are its peers, both optional"
  verify unit "software declares behavior, event, type and port, and governance as an optional peer"
}

invariant se_manifest_four_entity_kinds "Four Entity Kinds" {
  guarantee """
    The @specforge/software manifest MUST declare exactly 4 entity kinds:
    behavior, event, type, port. Features are product's and invariants
    governance's (ADR 0280).
  """
  risk      high
  verify property "manifest entityKinds array has exactly 4 entries"
}

invariant se_manifest_nine_edge_types "Nine Edge Types" {
  guarantee """
    The @specforge/software manifest MUST declare exactly 9 edge types:
    References, Implements, Produces, Consumes, UsesType, UsesPort,
    Enforces, Imports, LinksTo. These edges model all relationships
    between the 6 entity kinds in the software engineering domain.
  """
  risk      medium
  verify property "manifest edgeTypes array has exactly 9 entries"
}
