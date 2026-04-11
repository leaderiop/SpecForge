// LSP behaviors — Language Server Protocol features

use "invariants/core"
use "invariants/lsp"
use "invariants/validation"
use "invariants/wasm"
use "invariants/zero-entity-core"
use "types/core"
use "types/graph"
use "types/diagnostics"
use "types/zero-entity-core"
use "types/lsp"
use "types/wasm"
use "events/compilation"
use "ports/inbound"
use "types/errors"
use "ports/outbound"
behavior lsp_initialize "LSP Initialize" {
  invariants [zero_domain_knowledge_core, lsp_extension_reload_consistency]
  category   command
  types      [KindRegistryEntry, FieldRegistryEntry, SemanticTokenLegendEntry]
  ports      [LspProtocol]
  produces   [lsp_initialized]

  requires {
    extensions_loaded "extensions have been loaded and KindRegistry/FieldRegistry are populated"
  }

  ensures {
    capabilities_reflect_extensions "initialize response capabilities derive from loaded extension state, not hardcoded domain logic"
    semantic_legend_populated "semantic token legend includes all extension-defined token types from KindRegistry"
    incremental_sync_advertised "document sync kind is INCREMENTAL"
    lsp_initialized_emitted "lsp_initialized event is produced on successful initialization"
  }

  contract """
    When the LSP server receives an initialize request, it MUST respond
    with capabilities reflecting the current extension state: semantic
    token legend entries derived from the KindRegistry, completion
    trigger characters, rename support, code action kinds, and document
    sync kind (INCREMENTAL). The server MUST register all extension-defined
    semantic token types from KindRegistryEntry.semantic_token into the
    legend. The server MUST report support for workspace symbol search
    and document symbol outline. The initialization response MUST NOT
    hardcode any domain-specific capabilities — all capabilities beyond
    structural defaults MUST derive from loaded extensions.
  """

  verify unit "initialize response includes semantic token legend"
  verify unit "semantic token legend includes extension-defined token types"
  verify unit "initialize response advertises incremental sync"
  verify unit "initialize response includes completion trigger characters"
  verify unit "zero extensions produces structural-only capabilities"
  verify contract "requires/ensures consistency for LSP initialization"

}

behavior lsp_shutdown "LSP Shutdown" {
  invariants [incremental_correctness]
  category   command
  types      [Graph]
  ports      [LspProtocol]
  produces   [lsp_shutdown_complete]

  requires {
    lsp_initialized_fired "LSP server has been initialized and is in a running state"
  }

  ensures {
    resources_released "all held resources (graph, file watchers, Wasm engines, document buffers) are released"
    post_shutdown_rejected "all subsequent requests except exit return InvalidRequest errors"
    no_disk_persistence "no state is persisted to disk during shutdown"
    lsp_shutdown_complete_emitted "lsp_shutdown_complete event is produced"
  }

  contract """
    When the LSP server receives a shutdown request, it MUST release
    all held resources: the in-memory graph, file watchers, extension
    Wasm engines, and open document buffers. After shutdown, all
    subsequent requests except exit MUST return InvalidRequest errors.
    The server MUST NOT persist any state to disk during shutdown.
  """

  verify unit "shutdown releases in-memory graph"
  verify unit "shutdown releases Wasm engines"
  verify unit "requests after shutdown return InvalidRequest"
  verify contract "requires/ensures consistency for LSP shutdown"

}

behavior document_open_close "Document Open/Close" {
  invariants [incremental_correctness]
  category   command
  types      [SourceSpan]
  ports      [LspProtocol]
  produces   [file_changed]

  requires {
    lsp_initialized_fired "LSP server has been initialized and is ready to receive notifications"
  }

  ensures {
    document_tracked "open/close state of the document is correctly reflected in the open document set"
    file_changed_emitted "file_changed event is produced on didOpen to trigger initial compilation"
    closed_diagnostics_cleared "diagnostics for closed documents are cleared from the editor"
  }

  contract """
    When the LSP server receives a textDocument/didOpen notification,
    it MUST register the document in its open document set and trigger
    an initial compilation for diagnostics. When the server receives a
    textDocument/didClose notification, it MUST remove the document from
    its open document set. Diagnostics for closed documents MUST be
    cleared from the editor. The server MUST track which documents are
    open to determine the scope of incremental recompilation.
  """

  verify unit "didOpen registers document and triggers compilation"
  verify unit "didClose removes document and clears diagnostics"
  verify unit "only open documents participate in incremental compilation"
  verify contract "requires/ensures consistency for document open/close"

}

// Event consumer chain: didChange -> file_changed -> debounce window ->
// incremental rebuild (see shared_incremental_pipeline and behaviors/incremental.spec).
behavior handle_text_document_change "Handle Text Document Change" {
  types      [SpecFile, SourceSpan, ContentChangeEvent]
  category   command
  ports      [LspProtocol]
  produces   [file_changed]
  invariants [incremental_correctness, lsp_response_latency]

  requires {
    document_open "the document has been opened via didOpen and is in the open document set"
  }

  ensures {
    buffer_updated "in-memory document buffer reflects the incremental text edits"
    file_changed_emitted "file_changed event is produced to schedule recompile"
    event_loop_unblocked "the LSP event loop is not blocked by the handler"
  }

  contract """
    On textDocument/didChange notification, the LSP MUST apply
    incremental text edits to the in-memory document buffer, trigger
    incremental_document_sync, and schedule a recompile via the shared
    incremental pipeline. The handler MUST NOT block the LSP event loop.
  """

  verify unit "didChange applies incremental edits to buffer"
  verify unit "didChange triggers incremental recompile"
  verify contract "requires/ensures consistency for text document change"
}

behavior go_to_definition "Go-to-Definition" {
  category query
  invariants [reference_resolution_completeness, lsp_response_latency, zero_domain_knowledge_core]
  types      [EntityId, SourceSpan]
  ports      [LspProtocol]

  requires {
    graph_available "in-memory graph is built and contains resolved entity declarations"
  }

  ensures {
    declaration_site_returned "the file path, line, and column of the entity declaration block header are returned"
  }

  contract """
    When a user Ctrl+clicks on an entity ID in a .spec file, the LSP
    MUST navigate to the declaration site of that entity. The declaration
    site MUST include the file path, line, and column of the entity's
    block header.
  """

  verify unit        "go-to-def navigates to entity declaration"
  verify unit        "go-to-def on non-existent ID returns no result"
  verify integration "go-to-def works across files"
  verify contract "requires/ensures consistency for go-to-definition"

}

behavior find_all_references "Find All References" {
  category query
  invariants [reference_resolution_completeness, lsp_response_latency, zero_domain_knowledge_core]
  types      [EntityId, SourceSpan]
  ports      [LspProtocol]

  requires {
    graph_available "in-memory graph is built and contains resolved entity references"
  }

  ensures {
    all_references_returned "every location across all .spec files where the entity is referenced is returned"
    declaration_included "the entity's own declaration site is included in results"
  }

  contract """
    When a user triggers find-references on an entity ID, the LSP MUST
    return every location across all .spec files where that entity is
    referenced. Results MUST include the entity's own declaration site.
  """

  verify unit "find-refs returns all reference sites"
  verify unit "find-refs includes the declaration site"
  verify unit "find-refs across multiple files"
  verify contract "requires/ensures consistency for find all references"

}

behavior hover_information "Hover Information" {
  category query
  invariants [zero_domain_knowledge_core, reference_resolution_completeness, lsp_response_latency]
  types      [EntityId, Node, KindRegistryEntry, FieldRegistryEntry, HoverContent]
  ports      [LspProtocol]

  requires {
    graph_available "in-memory graph is built and entity nodes are queryable"
    kind_registry_available "KindRegistry is populated with extension-defined entity metadata"
  }

  ensures {
    hover_delegated "hover content generation is delegated to provide_extension_entity_hover"
    markdown_produced "the returned hover content is formatted as markdown"
  }

  // Delegation: hover_information delegates ALL entity metadata, reference
  // counts, and field summaries to provide_extension_entity_hover
  // (behaviors/zero-entity-lsp.spec). This behavior is the LSP entry point;
  // provide_extension_entity_hover is the authoritative owner of hover content.
  contract """
    When a user hovers over an entity ID, the LSP MUST delegate to
    provide_extension_entity_hover (behaviors/zero-entity-lsp.spec) for
    all extension-aware hover content: entity kind, title, source extension,
    testability, reference counts, and first string field summary.
    This behavior is responsible only for dispatching the hover request
    and returning the formatted result. The hover content MUST be
    formatted as markdown.
  """

  verify unit "hover delegates to provide_extension_entity_hover"
  verify unit "hover returns markdown-formatted content"
  verify contract "requires/ensures consistency for hover information"

}

// Completion behaviors (autocomplete_entity_ids, complete_field_names, complete_keywords)
// also cover verify declaration editing — verify kind names are suggested via the
// same completion pipeline.
behavior autocomplete_entity_ids "Autocomplete Entity IDs" {
  category query
  invariants [zero_domain_knowledge_core, reference_resolution_completeness, lsp_response_latency]
  types      [EntityId, CompletionItem]
  ports      [LspProtocol]

  requires {
    graph_available "in-memory graph is built and entity IDs are queryable"
    field_registry_available "FieldRegistry is populated with extension-declared field metadata"
  }

  ensures {
    matching_ids_suggested "matching entity IDs from the current scope are returned as completions"
    target_kind_filtering_applied "suggestions are filtered by target_kind constraint when present in FieldRegistry"
  }

  contract """
    When a user types inside a reference list (e.g., deps [...]),
    the LSP MUST suggest matching entity IDs from the current scope.
    Suggestions MUST include the entity title and kind. When the
    enclosing field has a target_kind constraint in the FieldRegistry
    (e.g., a "deps" field with a target_kind constraint filters to
    entities of that kind), suggestions MUST be filtered to entities of
    that kind. When no target_kind constraint exists, all entity IDs
    MUST be suggested. Entity IDs are globally unique regardless of
    kind — the filtering is a UX optimization based on
    extension-declared field metadata, not a compiler requirement.
  """

  verify unit "autocomplete suggests matching IDs"
  verify unit "suggestions include entity titles and kinds"
  verify unit "suggestions filtered by target_kind when FieldRegistry has constraint"
  verify unit "all IDs suggested when no target_kind constraint exists"
  verify contract "requires/ensures consistency for entity ID autocomplete"

}

behavior prepare_rename "Prepare Rename" {
  category query
  invariants [entity_id_uniqueness, lsp_response_latency, zero_domain_knowledge_core]
  types      [EntityId, SourceSpan]
  ports      [LspProtocol]

  requires {
    graph_available "in-memory graph is built and entity declarations are locatable"
  }

  ensures {
    token_range_returned "the range of the renameable token is returned when cursor is on an entity ID"
    non_renameable_rejected "rename is reported as not available when cursor is not on a renameable token"
  }

  contract """
    When a user initiates a rename, the LSP MUST first respond to
    textDocument/prepareRename to validate that the cursor is on a
    renameable token (entity ID in a declaration or reference). The
    response MUST include the range of the token to be renamed. If
    the cursor is not on a renameable token, the response MUST indicate
    that rename is not available at that position.
  """

  verify unit "prepare rename on entity ID returns token range"
  verify unit "prepare rename on non-renameable token returns not available"
  verify contract "requires/ensures consistency for prepare rename"

}

behavior rename_entity_id "Rename Entity ID" {
  invariants [entity_id_uniqueness, lsp_response_latency, rename_atomicity, zero_domain_knowledge_core]
  category   mutation
  types      [EntityId, TextEdit, WorkspaceEditResult]
  ports      [LspProtocol]
  produces   [entity_renamed]

  requires {
    graph_available "in-memory graph is built and all references are resolved"
    prepare_rename_ready "prepareRename has validated the cursor is on a renameable token"
  }

  ensures {
    all_references_updated "entity declaration and every reference across all .spec files are updated"
    rename_atomic "all files are updated or none are (atomic operation)"
    entity_renamed_emitted "entity_renamed event is produced on successful rename"
  }

  contract """
    When a user renames an entity ID via the LSP, the system MUST
    update the entity declaration and every reference to it across
    all .spec files. The rename MUST be atomic — all files are updated
    or none are.
  """

  verify unit "rename updates declaration and all references"
  verify unit "rename is atomic — all or nothing"
  verify unit "rename across multiple files"
  verify unit "rename rejects new name that duplicates existing entity ID"
  verify contract "requires/ensures consistency for entity rename"

}

// No produces — delegates to shared_incremental_pipeline which produces incremental_diagnostics_complete
behavior live_diagnostics "Live Diagnostics" {
  invariants [multi_error_collection, incremental_correctness, diagnostic_determinism, lsp_response_latency, zero_domain_knowledge_core]
  category   command
  types      [DiagnosticBag]
  ports      [LspProtocol]

  requires {
    lsp_initialized_fired "LSP server has been initialized and the incremental pipeline is ready"
    graph_available "in-memory graph is built and can be incrementally updated"
  }

  ensures {
    diagnostics_pushed "updated diagnostics are pushed to the editor after each file change"
    latency_enforced "error squiggles appear within 100ms of the user stopping typing"
  }

  contract """
    The LSP MUST provide real-time diagnostics as the user types.
    After each file change, the LSP MUST incrementally recompile and
    push updated diagnostics to the editor. Error squiggles MUST appear
    within 100ms of the user stopping typing.
  """

  verify unit        "diagnostics update after file change"
  verify unit        "only changed file diagnostics are refreshed"
  verify integration "diagnostics appear within 100ms"
  verify contract "requires/ensures consistency for live diagnostics"

}

behavior code_actions_for_missing_verify "Code Actions for Missing Verify" {
  invariants [zero_domain_knowledge_core, testable_entity_classification, lsp_response_latency, lsp_text_edit_non_overlapping]
  category   validation
  types      [KindRegistryEntry, CodeAction]
  ports      [LspProtocol]

  requires {
    kind_registry_available "KindRegistry is populated with testable flags and allowed_verify_kinds"
    graph_available "in-memory graph is built and entity verify declarations are queryable"
  }

  ensures {
    quickfix_offered "code action with CodeActionKind::QuickFix is offered on untested testable entities"
    verify_stubs_produced "generated stubs use allowed_verify_kinds from KindRegistry, not hardcoded kinds"
    no_code_generated "no test source files or application code are generated"
  }

  contract """
    The LSP SHOULD offer code actions on entities whose kind has
    testable=true in the KindRegistry but no verify declarations or
    linked test files. The code action MUST add verify stub declarations
    to the entity block in the .spec file, using verify kinds from the
    entity kind's allowed_verify_kinds in the KindRegistry (not hardcoded
    kinds). If no allowed_verify_kinds are specified, the stub MUST use
    the first verify kind from the extension's verify_kinds list. The
    generated stub MUST use the format
    verify <kind> "<entity_id> — TODO" where <kind> is the first entry
    from the entity kind's allowed_verify_kinds. The code action MUST use
    CodeActionKind::QuickFix. The LSP MUST NOT generate test source files
    or application code — SpecForge provides context, agents produce
    output. The set of testable kinds comes from extension manifests, not
    hardcoded logic.
  """

  verify unit "code action offered on untested testable entity"
  verify unit "generated verify stubs added to entity block in .spec file"
  verify unit "verify stub uses allowed_verify_kinds from KindRegistry"
  verify unit "stub format is verify <kind> entity_id TODO"
  verify unit "code action kind is QuickFix"
  verify unit "no test source files or application code generated"
  verify contract "requires/ensures consistency for missing verify code actions"

}

behavior outline_view "Outline View" {
  category query
  invariants [zero_domain_knowledge_core, reference_resolution_completeness, lsp_response_latency]
  types      [Node, EntityId, KindRegistryEntry, DocumentSymbolEntry]
  ports      [LspProtocol]

  requires {
    graph_available "in-memory graph is built and entities in the current file are queryable"
    kind_registry_available "KindRegistry is populated with lsp_icon metadata for SymbolKind mapping"
  }

  ensures {
    all_entities_listed "all entities in the current file are represented in the outline tree"
    symbol_kind_delegated "SymbolKind for each entry is determined by provide_extension_defined_lsp_icons, not hardcoded"
  }

  contract """
    The LSP MUST provide an outline view showing all entities in the
    current file as a tree. Each entry MUST show the entity kind,
    ID, and title. The SymbolKind for each entry MUST be determined
    by delegating to provide_extension_defined_lsp_icons which reads
    the lsp_icon field from the KindRegistry — the outline MUST NOT
    hardcode any SymbolKind mappings for specific entity types. Test
    coverage indicators SHOULD be shown when coverage data is available.
  """

  verify unit "outline lists all entities in file"
  verify unit "outline shows entity kind, ID, and title"
  verify unit "outline uses extension-defined SymbolKind from KindRegistry lsp_icon"
  verify contract "requires/ensures consistency for outline view"

}

behavior workspace_symbol_search "Workspace Symbol Search" {
  category query
  invariants [zero_domain_knowledge_core, reference_resolution_completeness, lsp_response_latency]
  types      [EntityId, SourceSpan, KindRegistryEntry, WorkspaceSymbolEntry]
  ports      [LspProtocol]

  requires {
    graph_available "in-memory graph is built and all workspace entities are indexed"
    kind_registry_available "KindRegistry is populated for SymbolKind resolution"
  }

  ensures {
    matching_entities_returned "matching entities across all .spec files are returned for the query"
    symbol_kind_delegated "SymbolKind for each result is determined by provide_extension_defined_lsp_icons"
  }

  contract """
    The LSP MUST support workspace symbol search. Typing an entity ID
    prefix or title fragment MUST return matching entities across all
    .spec files in the workspace. The SymbolKind for each result MUST
    be determined by provide_extension_defined_lsp_icons.
  """

  verify unit "search by ID prefix returns matches"
  verify unit "search by title fragment returns matches"
  verify unit "search results use extension-defined SymbolKind"
  verify contract "requires/ensures consistency for workspace symbol search"

}

// Delegates to behaviors/incremental.spec pipeline: watch_file_system_for_changes ->
// debounce_file_changes -> invalidate_changed_files -> rebuild_affected_subgraph ->
// emit_incremental_diagnostics
behavior shared_incremental_pipeline "Shared Incremental Pipeline" {
  invariants [incremental_correctness, zero_domain_knowledge_core, watch_mode_response_latency, lsp_extension_reload_consistency]
  category   command
  types      [Graph]
  consumes   [incremental_rebuild_complete]
  // Delegates to emit_incremental_diagnostics which produces incremental_diagnostics_complete.
  // This behavior does not independently trigger the event.
  ports      [LspProtocol]

  requires {
    incremental_rebuild_complete_fired "incremental_rebuild_complete event has fired, confirming the rebuild pipeline produced updated results"
  }

  ensures {
    shared_graph_updated "the in-memory graph shared between diagnostics, navigation, and completion is updated"
    diagnostics_pushed "incremental diagnostics are pushed to the client after pipeline completes"
    pipeline_parity_enforced "the parse-validate-emit pipeline is identical between LSP and CLI watch mode"
  }

  contract """
    The LSP server MUST share the same incremental compilation pipeline
    as specforge watch. File change MUST trigger an identical
    parse-validate-emit pipeline in both watch mode and LSP, using the
    same debounce window and validator dispatch order. The in-memory
    graph MUST be shared between diagnostics, navigation, and completion
    features. There MUST NOT be separate compilation passes for each
    LSP feature.
  """

  verify unit        "LSP and watch share the same graph"
  verify integration "graph update serves all LSP features"
  verify property "CLI and LSP share identical debounce window"
  verify property "CLI and LSP share identical validator dispatch order"
  verify contract "requires/ensures consistency for shared incremental pipeline"

}

behavior provide_semantic_tokens "Provide Semantic Tokens" {
  category query
  invariants [zero_domain_knowledge_core, lsp_response_latency]
  types      [SourceSpan, KindRegistryEntry, SemanticTokenLegendEntry, SemanticToken]
  ports      [LspProtocol]

  requires {
    graph_available "in-memory graph is built and entity kinds are resolvable"
    kind_registry_available "KindRegistry is populated with semantic_token classification metadata"
  }

  ensures {
    tokens_classified "all tokens in .spec files are classified according to their semantic role"
    structural_keywords_enforced "use, define, and verify are always classified as keyword regardless of extensions"
    extension_delegation_applied "entity keyword classification delegates to provide_extension_entity_semantic_tokens"
  }

  contract """
    The LSP MUST provide semantic tokens for .spec files. Entity keyword
    classification MUST delegate to provide_extension_entity_semantic_tokens
    (behaviors/zero-entity-lsp.spec) which uses the semantic_token field
    from the KindRegistry. Default semantic_token classification MUST be
    determined by provide_extension_entity_semantic_tokens. Entity IDs
    MUST be classified by their kind's semantic token type. Structural
    keywords (use, define, verify) MUST always be classified as
    "keyword". Triple-quoted strings MUST be classified as strings.
    Enhanced fields from entity enhancements MUST be classified as
    property. For entity kinds with registered grammar contributions,
    semantic token classification MAY delegate to the extension grammar
    for finer-grained highlighting within entity bodies. Semantic token
    updates MUST use the shared incremental pipeline.
  """

  verify unit "entity keywords use KindRegistry semantic_token"
  verify unit "structural keywords are classified as keyword"
  verify unit "triple-quoted strings are classified as strings"
  verify unit "default semantic_token is keyword when not specified"
  verify unit "enhanced fields are classified as property"
  verify contract "requires/ensures consistency for semantic tokens"

}

behavior complete_field_names "Complete Field Names" {
  category query
  invariants [zero_domain_knowledge_core, lsp_response_latency]
  types      [EntityId, FieldRegistryEntry, CompletionItem]
  ports      [LspProtocol]

  requires {
    field_registry_available "FieldRegistry is populated with field definitions for all registered entity kinds"
    cursor_inside_entity "the cursor is positioned inside an entity block body"
  }

  ensures {
    fields_suggested "valid field names for the current entity kind are returned as completions"
    snippets_informed "completion snippets reflect field types from the FieldRegistry"
  }

  contract """
    When a user types inside an entity block body, the LSP MUST query
    the FieldRegistry for valid field names for the current entity kind
    and suggest them as completions. Suggestions MUST be filtered to
    fields registered for the entity kind by its extension manifest.
    Field types from the registry MUST inform the completion snippet
    (e.g., reference fields offer bracket-list scaffolding).
  """

  verify unit "field name completion uses FieldRegistry for entity kind"
  verify unit "suggestions are filtered by entity kind"
  verify unit "no field name suggestions outside entity blocks"
  verify contract "requires/ensures consistency for field name completion"

}

behavior complete_keywords "Complete Keywords" {
  category query
  invariants [zero_domain_knowledge_core]
  types      [EntityId, KindRegistryEntry, CompletionItem]
  ports      [LspProtocol]

  requires {
    kind_registry_available "KindRegistry is populated with extension-defined entity kinds"
    cursor_at_top_level "the cursor is positioned at the top level of a .spec file, outside any entity block"
  }

  ensures {
    keywords_delegated "keyword completion delegates to complete_extension_defined_keywords for extension-aware results"
    structural_keywords_included "use and define are always included in suggestions regardless of extensions"
  }

  contract """
    When a user types at the top level of a .spec file (outside any entity
    block), the LSP MUST delegate to complete_extension_defined_keywords
    (behaviors/zero-entity-lsp.spec) for extension-aware keyword completions.
    Structural keywords (use, define) MUST always be included in addition
    to extension-defined keywords. Each suggestion SHOULD include a snippet
    template for block scaffolding based on the kind's field definitions
    from the FieldRegistry. The detail string MUST show the source extension
    name for each keyword.
  """

  verify unit "keyword completion includes all registered kinds"
  verify unit "structural keywords always included"
  verify unit "no keyword suggestions inside entity blocks"
  verify unit "snippet templates based on kind field definitions"
  verify contract "requires/ensures consistency for keyword completion"

}

behavior goto_import_definition "Go-to-Definition on Imports" {
  category query
  invariants [reference_resolution_completeness, lsp_response_latency, zero_domain_knowledge_core]
  types      [SourceSpan]
  ports      [LspProtocol]

  requires {
    imports_resolved "use import paths have been resolved to file system locations"
  }

  ensures {
    target_file_navigated "navigation jumps to the first line of the resolved target .spec file"
  }

  contract """
    When a user Ctrl+clicks on a `use` import path (e.g., `use behaviors/core`),
    the LSP MUST navigate to the target .spec file. The definition site
    MUST be the first line of the resolved file.
  """

  verify unit "go-to-def on use path navigates to target file"
  verify unit "go-to-def on non-existent use path returns no result"
  verify contract "requires/ensures consistency for import go-to-definition"

}

behavior code_action_add_missing_import "Code Action: Add Missing Import" {
  category mutation
  invariants [reference_resolution_completeness, zero_domain_knowledge_core, lsp_response_latency, lsp_text_edit_non_overlapping]
  types      [EntityId, CodeAction]
  ports      [LspProtocol]

  requires {
    graph_available "in-memory graph is built and E001 diagnostics are available"
    entity_exists_elsewhere "the referenced entity ID exists in another file in the workspace"
  }

  ensures {
    import_added "a use import statement is inserted after existing use statements at the top of the file"
  }

  contract """
    When an E001 diagnostic (unresolved reference) exists for an entity ID
    that exists in another file, the LSP MUST offer a code action to add
    the appropriate `use` import statement. (E001 is emitted by
    link_entity_references during resolution, not by the validator.) The import MUST be inserted
    at the top of the file, after existing `use` statements.
  """

  verify unit "code action offered on E001 for resolvable entity"
  verify unit "import is inserted after existing use statements"
  verify unit "no code action when entity does not exist anywhere"
  verify contract "requires/ensures consistency for add missing import"

}

behavior code_action_create_entity_stub "Code Action: Create Entity Stub" {
  category mutation
  invariants [zero_domain_knowledge_core, lsp_response_latency, lsp_text_edit_non_overlapping]
  types      [EntityId, KindRegistryEntry, FieldRegistryEntry, CodeAction]
  ports      [LspProtocol]

  requires {
    graph_available "in-memory graph is built and E001 diagnostics are available"
    field_registry_available "FieldRegistry is populated with target_kind constraints for kind inference"
  }

  ensures {
    stub_created "a structural entity block stub is inserted at the end of the current file"
    kind_inferred "entity kind is inferred from the enclosing field's target_kind constraint in the FieldRegistry"
    no_code_generated "no application code, test files, or implementation scaffolding are generated"
  }

  contract """
    When an E001 diagnostic (unresolved reference) exists for an entity ID
    that does not exist in any file, the LSP SHOULD offer a code action
    to create a stub entity definition. The entity kind for the stub MUST
    be inferred from the enclosing field's target_kind constraint in the
    FieldRegistry — this is extension-driven metadata, not hardcoded logic.
    When no target_kind constraint exists on the enclosing field, the code
    action MUST NOT be offered (the kind cannot be inferred without domain
    knowledge). The stub MUST be placed in the current file. The code
    action MUST use CodeActionKind::Refactor.
    The generated stub MUST contain only the structural entity block
    (keyword, ID, placeholder fields). It MUST NOT generate application
    code, test files, or implementation scaffolding. SpecForge provides
    structural context; agents produce implementation.
  """

  verify unit "code action offered on E001 for non-existent entity"
  verify unit "stub uses correct entity kind from FieldRegistry target_kind"
  verify unit "no code action when enclosing field has no target_kind"
  verify unit "stub is inserted at end of current file"
  verify unit "code action kind is Refactor"
  verify unit "generated stub contains no application code or test files"
  verify contract "requires/ensures consistency for create entity stub"

}

behavior incremental_document_sync "Incremental Document Sync" {
  invariants [incremental_correctness, lsp_response_latency]
  category   command
  types      [SourceSpan]
  ports      [LspProtocol]

  requires {
    lsp_initialized_fired "LSP server has been initialized with INCREMENTAL sync kind advertised"
    document_open "the document is registered in the open document set"
  }

  ensures {
    buffer_consistent "the in-memory source buffer is identical to the full content after applying changes"
    partial_update_applied "only the changed range is applied, not the entire file content"
  }

  contract """
    The LSP MUST support incremental text document synchronization
    (TextDocumentSyncKind::INCREMENTAL). On each change event, the LSP
    MUST apply only the changed range to its in-memory source buffer
    rather than replacing the entire file content. The resulting source
    MUST be identical to the full content at all times.
  """

  verify unit "incremental change applies correctly to source buffer"
  verify unit "multiple incremental changes produce correct source"
  verify integration "incremental sync reduces transfer size vs full sync"
  verify contract "requires/ensures consistency for incremental document sync"

}

// -- Extension Grammar Highlighting -------------------------------------------

behavior load_extension_grammars_for_highlighting "Load Extension Grammars for Highlighting" {
  invariants [grammar_injection_isolation, grammar_composition_determinism]
  category   command
  types      [GrammarContribution, GrammarCacheEntry, GrammarError, KindRegistryEntry]
  ports      [WasmRuntime]

  requires {
    extensions_loaded "extensions have been loaded and grammar contributions are registered"
    wasm_runtime_available "WasmRuntime port is available for loading grammar .wasm files"
  }

  ensures {
    grammars_cached "loaded grammars are cached for the lifetime of the LSP session"
    grammar_kind_association "each grammar is associated with its declared entity kinds for targeted highlighting"
    loading_failures_isolated "grammar loading failures produce diagnostics without degrading highlighting for other entity kinds"
  }

  contract """
    At LSP startup and on extension configuration change, the LSP MUST
    load grammar .wasm files from registered grammar contributions. Each
    grammar MUST be associated with its declared entity kinds for targeted
    syntax highlighting. Grammar loading MUST respect the grammar_policy
    for conflict resolution. Loaded grammars MUST be cached for the
    lifetime of the LSP session. Grammar loading failures MUST produce
    diagnostics without degrading highlighting for other entity kinds.
  """

  verify unit "grammar loaded at LSP startup for registered contributions"
  verify unit "grammar reloaded on extension configuration change"
  verify unit "grammar conflict resolved per grammar_policy"
  verify unit "grammar loading failure does not affect other kinds"
  verify contract "requires/ensures consistency for extension grammar loading"

}
