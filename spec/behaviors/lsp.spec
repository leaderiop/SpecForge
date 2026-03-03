// LSP behaviors — Language Server Protocol features

use invariants/core
use invariants/validation
use types/core
use types/graph
use types/diagnostics
use ports/inbound

behavior go_to_definition "Go-to-Definition" {
  invariants [reference_resolution_completeness]
  types      [EntityId, SourceSpan]
  ports      [LspProtocol]

  contract """
    When a user Ctrl+clicks on an entity ID in a .spec file, the LSP
    MUST navigate to the declaration site of that entity. The declaration
    site MUST include the file path, line, and column of the entity's
    block header.
  """

  verify unit        "go-to-def navigates to entity declaration"
  verify unit        "go-to-def on non-existent ID returns no result"
  verify integration "go-to-def works across files"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior find_all_references "Find All References" {
  types      [EntityId, SourceSpan]
  ports      [LspProtocol]

  contract """
    When a user triggers find-references on an entity ID, the LSP MUST
    return every location across all .spec files where that entity is
    referenced. Results MUST include the entity's own declaration site.
  """

  verify unit "find-refs returns all reference sites"
  verify unit "find-refs includes the declaration site"
  verify unit "find-refs across multiple files"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior hover_information "Hover Information" {
  types      [EntityId, Node]
  ports      [LspProtocol]

  contract """
    When a user hovers over an entity ID, the LSP MUST display the
    entity's title, guarantee/contract text, reference count, and test
    count. The hover MUST include the entity type and ID.
  """

  verify unit "hover shows entity title and type"
  verify unit "hover shows contract or guarantee text"
  verify unit "hover shows reference count"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior autocomplete_entity_ids "Autocomplete Entity IDs" {
  types      [EntityId]
  ports      [LspProtocol]

  contract """
    When a user types inside a reference list (e.g., invariants [INV-),
    the LSP MUST suggest matching entity IDs from the current scope.
    Suggestions MUST include the entity title. Only entities of the
    correct type for the field MUST be suggested.
  """

  verify unit "autocomplete suggests matching IDs"
  verify unit "suggestions include entity titles"
  verify unit "suggestions filter by expected entity type"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior rename_entity_id "Rename Entity ID" {
  invariants [entity_id_uniqueness]
  types      [EntityId, GeneratedFile]
  ports      [LspProtocol]

  contract """
    When a user renames an entity ID via the LSP, the system MUST
    update the entity declaration and every reference to it across
    all .spec files. The rename MUST be atomic — all files are updated
    or none are.
  """

  verify unit "rename updates declaration and all references"
  verify unit "rename is atomic — all or nothing"
  verify unit "rename across multiple files"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior live_diagnostics "Live Diagnostics" {
  invariants [multi_error_collection, incremental_correctness]
  types      [DiagnosticBag]
  ports      [LspProtocol]

  contract """
    The LSP MUST provide real-time diagnostics as the user types.
    After each file change, the LSP MUST incrementally recompile and
    push updated diagnostics to the editor. Error squiggles MUST appear
    within 100ms of the user stopping typing.
  """

  verify unit        "diagnostics update after file change"
  verify unit        "only changed file diagnostics are refreshed"
  verify integration "diagnostics appear within 100ms"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior code_actions_for_missing_tests "Code Actions for Missing Tests" {
  types      [EntityId]
  ports      [LspProtocol]

  contract """
    The LSP SHOULD offer code actions on untested behaviors: "Generate
    test stub for TypeScript/Python/Go". The code action MUST produce
    a test file stub with the behavior ID and verify descriptions
    pre-filled.
  """

  verify unit "code action offered on untested behavior"
  verify unit "generated stub includes behavior ID"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior outline_view "Outline View" {
  types      [Node, EntityId]
  ports      [LspProtocol]

  contract """
    The LSP MUST provide an outline view showing all entities in the
    current file as a tree. Each entry MUST show the entity type,
    ID, and title. Test coverage indicators SHOULD be shown when
    coverage data is available.
  """

  verify unit "outline lists all entities in file"
  verify unit "outline shows entity type and ID"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior workspace_symbol_search "Workspace Symbol Search" {
  types      [EntityId, SourceSpan]
  ports      [LspProtocol]

  contract """
    The LSP MUST support workspace symbol search. Typing an entity ID
    prefix or title fragment MUST return matching entities across all
    .spec files in the workspace.
  """

  verify unit "search by ID prefix returns matches"
  verify unit "search by title fragment returns matches"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior shared_incremental_pipeline "Shared Incremental Pipeline" {
  invariants [incremental_correctness]
  types      [Graph]
  ports      [LspProtocol]

  contract """
    The LSP server MUST share the same incremental compilation pipeline
    as specforge watch. The in-memory graph MUST be shared between
    diagnostics, navigation, and completion features. There MUST NOT
    be separate compilation passes for each LSP feature.
  """

  verify unit        "LSP and watch share the same graph"
  verify integration "graph update serves all LSP features"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior provide_semantic_tokens "Provide Semantic Tokens" {
  types      [SourceSpan]
  ports      [LspProtocol]

  contract """
    The LSP MUST provide semantic tokens for .spec files. Entity IDs MUST
    be classified by entity type (behavior, feature, invariant, etc.).
    Keywords (behavior, feature, use, etc.) MUST be classified as keywords.
    Triple-quoted strings MUST be classified as strings. Custom entity
    keywords from plugins and define blocks MUST be classified as keyword.
    Custom entity IDs MUST be classified by their entity kind. Enhanced
    fields from entity enhancements MUST be classified as property.
    Semantic token updates MUST use the shared incremental pipeline.
  """

  verify unit "entity IDs receive correct semantic token type"
  verify unit "keywords are classified as keywords"
  verify unit "triple-quoted strings are classified as strings"
  verify unit "custom entity keywords from plugins are classified as keyword"
  verify unit "enhanced fields are classified as property"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior complete_field_names "Complete Field Names" {
  types      [EntityId]
  ports      [LspProtocol]

  contract """
    When a user types inside an entity block body, the LSP MUST suggest
    valid field names for the current entity kind. For example, inside a
    behavior block, typing `con` MUST suggest `contract`, `constraints`,
    etc. Suggestions MUST be filtered to fields valid for the entity kind.
  """

  verify unit "field name completion inside a behavior block"
  verify unit "suggestions are filtered by entity kind"
  verify unit "no field name suggestions outside entity blocks"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior complete_keywords "Complete Keywords" {
  types      [EntityId]
  ports      [LspProtocol]

  contract """
    When a user types at the top level of a .spec file (outside any entity
    block), the LSP MUST suggest entity keywords (behavior, feature,
    invariant, type, port, use, etc.). The suggestions SHOULD include
    snippet templates for block scaffolding.
  """

  verify unit "keyword completion at file top level"
  verify unit "no keyword suggestions inside entity blocks"
  verify unit "snippet templates include block structure"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior goto_import_definition "Go-to-Definition on Imports" {
  types      [SourceSpan]
  ports      [LspProtocol]

  contract """
    When a user Ctrl+clicks on a `use` import path (e.g., `use behaviors/core`),
    the LSP MUST navigate to the target .spec file. The definition site
    MUST be the first line of the resolved file.
  """

  verify unit "go-to-def on use path navigates to target file"
  verify unit "go-to-def on non-existent use path returns no result"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior code_action_add_missing_import "Code Action: Add Missing Import" {
  types      [EntityId]
  ports      [LspProtocol]

  contract """
    When the validator emits E001 (unresolved reference) for an entity ID
    that exists in another file, the LSP MUST offer a code action to add
    the appropriate `use` import statement. The import MUST be inserted
    at the top of the file, after existing `use` statements.
  """

  verify unit "code action offered on E001 for resolvable entity"
  verify unit "import is inserted after existing use statements"
  verify unit "no code action when entity does not exist anywhere"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior code_action_create_entity_stub "Code Action: Create Entity Stub" {
  types      [EntityId]
  ports      [LspProtocol]

  contract """
    When the validator emits E001 (unresolved reference) for an entity ID
    that does not exist in any file, the LSP SHOULD offer a code action
    to create a stub entity definition. The stub MUST be placed in the
    current file and MUST include the correct entity kind inferred from
    the reference context.
  """

  verify unit "code action offered on E001 for non-existent entity"
  verify unit "stub uses correct entity kind from reference context"
  verify unit "stub is inserted at end of current file"

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}

behavior incremental_document_sync "Incremental Document Sync" {
  invariants [incremental_correctness]
  types      [SourceSpan]
  ports      [LspProtocol]

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

  tests ["../crates/specforge-lsp/tests/integration.rs"]
}
