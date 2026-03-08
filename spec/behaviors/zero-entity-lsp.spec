// Zero-entity core — extension-driven LSP features

use invariants/zero-entity-core
use types/zero-entity-core
use types/lsp
use invariants/lsp
use ports/inbound

// -- Extension-Driven LSP ----------------------------------------------------

behavior complete_extension_defined_keywords "Complete Extension-Defined Keywords" {
  invariants [zero_domain_knowledge_core, lsp_response_latency]
  types      [KindRegistryEntry]
  ports      [LspProtocol]

  contract """
    The LSP autocomplete MUST query the KindRegistry for all registered
    entity kinds when completing keywords at the top level of a .spec file.
    Each completion item MUST include the keyword, a snippet template
    for the block body based on the kind's fields, and a detail string
    showing the source extension name.
  """

  requires {
    kind_registry_populated "KindRegistry is populated (registries_populated event has fired)"
  }

  ensures {
    all_registered_keywords_included "Every keyword from the KindRegistry is included in the completion list"
    snippet_templates_provided "Each completion item includes a snippet template for block scaffolding"
  }

  verify unit "completion includes all registered keywords"
  verify unit "completion items include snippet templates"
  verify unit "completion detail shows source extension"
  verify contract "requires/ensures consistency for extension keyword completion"

}

behavior provide_extension_entity_semantic_tokens "Provide Extension Entity Semantic Tokens" {
  invariants [zero_domain_knowledge_core, lsp_response_latency]
  types      [KindRegistryEntry]
  ports      [LspProtocol]

  contract """
    The LSP semantic token provider MUST classify entity keywords using
    the semantic_token field from the KindRegistry entry. If no semantic
    token is specified, the default MUST be "keyword". The token type
    MUST be included in the server's semantic token legend at initialization.
    Enhanced fields from entity enhancements MUST be classified as
    property. This is the authoritative behavior for extension-aware
    semantic token logic; provide_semantic_tokens (behaviors/lsp.spec)
    delegates here for keyword classification.
  """

  requires {
    kind_registry_populated "KindRegistry is populated (registries_populated event has fired)"
  }

  ensures {
    token_type_resolved_from_registry "Token type for each entity keyword is resolved from KindRegistryEntry.semantic_token"
    default_keyword_for_unregistered "Unspecified semantic_token defaults to keyword"
  }

  verify unit "custom semantic token type used for extension keyword"
  verify unit "default keyword token used when semantic_token not specified"
  verify unit "custom token types included in legend"
  verify contract "requires/ensures consistency for extension semantic tokens"

}

behavior provide_extension_entity_hover "Provide Extension Entity Hover" {
  invariants [zero_domain_knowledge_core, lsp_response_latency]
  types      [KindRegistryEntry, HoverContent]
  ports      [LspProtocol]

  contract """
    When hovering over an entity keyword or entity ID, the LSP MUST show
    the entity kind name, the source extension that defines it, the
    entity's title, and the incoming/outgoing reference count from the
    graph. For entity kinds with testable=true, the hover MUST also
    indicate testability. The LSP MUST also display the first string
    field value (if any) as a summary — "first string field" means the
    first string field in declaration order within the .spec file (the
    order fields appear in the entity block's AST). The specific field
    name depends on the extension's field definitions. The hover content MUST be
    formatted as markdown. This is the authoritative behavior for
    extension-aware hover logic; hover_information (behaviors/lsp.spec)
    delegates here.
  """

  requires {
    kind_registry_populated "KindRegistry is populated (registries_populated event has fired)"
  }

  ensures {
    hover_content_from_registry "Hover content is generated from KindRegistryEntry metadata"
    source_extension_shown "Source extension name is displayed in hover content"
  }

  verify unit "hover shows entity kind and source extension"
  verify unit "hover shows testability for testable kinds"
  verify unit "hover content formatted as markdown"
  verify unit "hover shows first string field as summary"
  verify unit "hover shows reference count from graph"
  verify contract "requires/ensures consistency for extension entity hover"

}

behavior provide_extension_defined_lsp_icons "Provide Extension-Defined LSP Icons" {
  invariants [zero_domain_knowledge_core, lsp_response_latency]
  types      [KindRegistryEntry]
  ports      [LspProtocol]

  contract """
    The LSP document symbols and workspace symbols MUST use the lsp_icon
    field from the KindRegistry entry to determine the SymbolKind. If no
    lsp_icon is specified, the default MUST be SymbolKind::Object.
    Extension-defined icons MUST appear in the outline view and symbol search.
  """

  requires {
    kind_registry_populated "KindRegistry is populated (registries_populated event has fired)"
  }

  ensures {
    symbol_kind_from_registry "SymbolKind is resolved from KindRegistryEntry.lsp_icon"
    default_object_for_unregistered "Unspecified lsp_icon defaults to SymbolKind::Object"
  }

  verify unit "custom SymbolKind used from manifest lsp_icon"
  verify unit "default SymbolKind::Object when lsp_icon not specified"
  verify unit "extension icons appear in outline view"
  verify contract "requires/ensures consistency for extension LSP icons"

}
